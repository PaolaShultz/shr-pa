//! Direct ALSA transport. Channel maps never mix logical outputs.
use crate::{
    config::Config,
    dsp::Engine,
    offline::{Generator, GeneratorLevel, Result, Signal},
};
use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, HwParams, PCM},
};
use std::{
    sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Mapping {
    pub capture_channels: usize,
    pub playback_channels: usize,
    pub inputs: [usize; 2],
    pub outputs: [Option<usize>; 6],
}
impl Mapping {
    pub fn validate(&self) -> Result<()> {
        if !(1..=64).contains(&self.capture_channels)
            || !(1..=64).contains(&self.playback_channels)
            || self.inputs.iter().any(|&v| v >= self.capture_channels)
        {
            return Err("invalid capture channel mapping".into());
        }
        let mut used = [false; 64];
        for &ch in self.outputs.iter().flatten() {
            if ch >= self.playback_channels || used[ch] {
                return Err(
                    "physical output unavailable or assigned twice; outputs are never mixed".into(),
                );
            }
            used[ch] = true;
        }
        Ok(())
    }
    /// Encode exactly the selected logical outputs; clear all unused physical channels.
    pub fn pack(&self, frames: &[[f32; 6]], format: SampleFormat, target: &mut [u8]) -> Result<()> {
        self.validate()?;
        let frame_bytes = self.playback_channels * format.width();
        target.fill(0);
        if target.len() / frame_bytes != frames.len() || !target.len().is_multiple_of(frame_bytes) {
            return Err("playback conversion buffer shape mismatch".into());
        }
        for (frame, bytes) in frames.iter().zip(target.chunks_exact_mut(frame_bytes)) {
            for (logical, physical) in self.outputs.iter().enumerate() {
                if let Some(physical) = physical {
                    format.encode(frame[logical], &mut bytes[physical * format.width()..]);
                }
            }
        }
        Ok(())
    }
    pub fn parse(
        capture_channels: usize,
        playback_channels: usize,
        inputs: &str,
        outputs: &str,
    ) -> Result<Self> {
        let inputs: Vec<usize> = inputs
            .split(',')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        let outputs: Vec<Option<usize>> = outputs
            .split(',')
            .map(|s| {
                if s == "-" {
                    Ok(None)
                } else {
                    s.parse().map(Some)
                }
            })
            .collect::<std::result::Result<_, _>>()?;
        let m = Self {
            capture_channels,
            playback_channels,
            inputs: inputs.try_into().map_err(|_| "need two input indices")?,
            outputs: outputs
                .try_into()
                .map_err(|_| "need six output indices or '-'")?,
        };
        m.validate()?;
        Ok(m)
    }
}
/// Lock-free control and approximate telemetry; no audio is queued through it.
pub struct Shared {
    pub controls: crate::control::Handoff,
    pub generator: crate::control::GeneratorControl,
    pub transitioning: AtomicBool,
    pub actual_block: AtomicU32,
    pub reduction: [AtomicU32; 4],
    pub clips: AtomicU8,
    pub stop: AtomicBool,
    pub mutes: AtomicU8,
    pub input: [AtomicU32; 2],
    pub output: [AtomicU32; 6],
    pub fault: AtomicBool,
    pub blocks: AtomicU32,
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            controls: crate::control::Handoff::default(),
            generator: crate::control::GeneratorControl::default(),
            transitioning: AtomicBool::new(false),
            actual_block: AtomicU32::new(0),
            reduction: std::array::from_fn(|_| AtomicU32::new(0)),
            clips: AtomicU8::new(0),
            stop: AtomicBool::new(false),
            mutes: AtomicU8::new(63),
            input: std::array::from_fn(|_| AtomicU32::new(0)),
            output: std::array::from_fn(|_| AtomicU32::new(0)),
            fault: AtomicBool::new(false),
            blocks: AtomicU32::new(0),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    S32,
    S24Packed,
    S16,
    Float,
}
impl SampleFormat {
    fn alsa(self) -> Format {
        match self {
            Self::S32 => Format::S32LE,
            Self::S24Packed => Format::S243LE,
            Self::S16 => Format::S16LE,
            Self::Float => Format::FloatLE,
        }
    }
    pub fn width(self) -> usize {
        match self {
            Self::S32 | Self::Float => 4,
            Self::S24Packed => 3,
            Self::S16 => 2,
        }
    }
    pub fn decode(self, b: &[u8]) -> f32 {
        match self {
            Self::S32 => {
                (i32::from_le_bytes(b[..4].try_into().unwrap()) as f64 / 2147483648.) as f32
            }
            Self::S16 => i16::from_le_bytes(b[..2].try_into().unwrap()) as f32 / 32768.,
            Self::S24Packed => {
                ((i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f64 / 8388608.) as f32
            }
            Self::Float => f32::from_le_bytes(b[..4].try_into().unwrap()),
        }
    }
    pub fn encode(self, x: f32, b: &mut [u8]) {
        let x = if x.is_finite() { x.clamp(-1., 1.) } else { 0. };
        match self {
            Self::S32 => b[..4].copy_from_slice(
                &((x as f64 * 2147483648.)
                    .round()
                    .clamp(i32::MIN as f64, i32::MAX as f64) as i32)
                    .to_le_bytes(),
            ),
            Self::S16 => b[..2].copy_from_slice(
                &((x * 32768.).round().clamp(-32768., 32767.) as i16).to_le_bytes(),
            ),
            Self::S24Packed => b[..3].copy_from_slice(
                &((x as f64 * 8388608.).round().clamp(-8388608., 8388607.) as i32).to_le_bytes()
                    [..3],
            ),
            Self::Float => b[..4].copy_from_slice(&x.to_le_bytes()),
        }
    }
}
#[derive(Debug)]
pub struct Negotiated {
    pub format: SampleFormat,
    pub channels: usize,
    pub rate: u32,
    pub period: usize,
    pub buffer: usize,
}
fn open(
    device: &str,
    direction: Direction,
    channels: usize,
    c: &Config,
) -> Result<(PCM, Negotiated)> {
    // Require raw hw for real devices: no silent plugin resampling or fallback.
    // null is an explicitly selected software transport test, never hardware evidence.
    if !device.starts_with("hw:") && device != "null" {
        return Err(
            "use an explicit hw:CARD=...,DEV=... endpoint (or null for software testing)".into(),
        );
    }
    let pcm = PCM::new(device, direction, true)?;
    let p = HwParams::any(&pcm)?;
    p.set_access(Access::RWInterleaved)?;
    p.set_rate_resample(false)?;
    p.set_channels(channels as u32)?;
    p.set_rate(c.sample_rate, ValueOr::Nearest)?;
    let format = [
        SampleFormat::S32,
        SampleFormat::S24Packed,
        SampleFormat::S16,
        SampleFormat::Float,
    ]
    .into_iter()
    .find(|f| p.test_format(f.alsa()).is_ok())
    .ok_or("no supported PCM sample format")?;
    p.set_format(format.alsa())?;
    let period = p.set_period_size_near(c.max_block as i64, ValueOr::Nearest)?;
    p.set_buffer_size_near(period * 4)?;
    pcm.hw_params(&p)?;
    let actual = pcm.hw_params_current()?;
    let n = Negotiated {
        format,
        channels: actual.get_channels()? as usize,
        rate: actual.get_rate()?,
        period: actual.get_period_size()? as usize,
        buffer: actual.get_buffer_size()? as usize,
    };
    if n.rate != c.sample_rate
        || n.channels != channels
        || n.period == 0
        || n.period > 8192
        || n.buffer < n.period * 2
        || n.buffer > 131072
    {
        return Err("negotiated stream violates rate/channel/buffer bounds".into());
    }
    let sw = pcm.sw_params_current()?;
    sw.set_avail_min(n.period as i64)?;
    sw.set_start_threshold(sw.get_boundary()?)?;
    pcm.sw_params(&sw)?;
    drop(sw);
    drop(actual);
    drop(p);
    pcm.prepare()?;
    Ok((pcm, n))
}
/// ALSA errors requiring restart terminate the session with both streams dropped.
#[derive(Debug, PartialEq, Eq)]
pub enum TransferAction {
    Retry,
    Xrun,
    Disconnected,
    Fatal,
}
pub fn classify(errno: i32) -> TransferAction {
    match errno {
        4 | 11 => TransferAction::Retry,
        32 | 86 => TransferAction::Xrun,
        19 | 6 => TransferAction::Disconnected,
        _ => TransferAction::Fatal,
    }
}
#[derive(Debug)]
struct Stopped;
impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("stopped")
    }
}
impl std::error::Error for Stopped {}

/// Pure transfer driver, shared by ALSA and deterministic fault/partial-I/O tests.
/// Offsets/counts are frames; waits and stop checks are bounded by the caller.
pub fn transfer_frames(
    frames: usize,
    mut stopped: impl FnMut() -> bool,
    mut io: impl FnMut(usize) -> alsa::Result<usize>,
    mut wait: impl FnMut() -> alsa::Result<bool>,
) -> Result<()> {
    let mut offset = 0;
    let mut progress = Instant::now();
    while offset < frames {
        if stopped() {
            return Err(Box::new(Stopped));
        }
        match io(offset) {
            Ok(n) if n > frames - offset => return Err("PCM returned too many frames".into()),
            Ok(n) if n > 0 => {
                offset += n;
                progress = Instant::now();
            }
            Ok(_) => {
                wait_retry(&mut wait)?;
            }
            Err(e) if classify(e.errno()) == TransferAction::Retry => {
                wait_retry(&mut wait)?;
            }
            Err(e) => return Err(e.into()),
        }
        if progress.elapsed() > Duration::from_secs(2) {
            return Err("PCM made no progress for two seconds".into());
        }
    }
    Ok(())
}
fn wait_retry(wait: &mut impl FnMut() -> alsa::Result<bool>) -> Result<()> {
    match wait() {
        Ok(_) => Ok(()),
        Err(e) if classify(e.errno()) == TransferAction::Retry => Ok(()),
        Err(e) => Err(e.into()),
    }
}
/// Partial transfers advance by frames. No stale prefix is sent twice.
fn transfer(
    pcm: &PCM,
    buffer: &mut [u8],
    frame_bytes: usize,
    capture: bool,
    shared: &Shared,
) -> Result<()> {
    let io = pcm.io_bytes();
    transfer_frames(
        buffer.len() / frame_bytes,
        || shared.stop.load(Ordering::Relaxed),
        |offset| {
            let offset = offset * frame_bytes;
            if capture {
                io.readi(&mut buffer[offset..])
            } else {
                io.writei(&buffer[offset..])
            }
        },
        || pcm.wait(Some(20)),
    )
}
#[derive(Debug, Default)]
pub struct LiveReport {
    pub frames: u64,
    pub max_render_us: f64,
    pub mean_render_us: f64,
    pub xruns: u32,
    pub input_peaks: [f32; 2],
    pub output_peaks: [f32; 6],
    pub fault: Option<String>,
}
pub struct LiveOptions<'a> {
    pub capture: &'a str,
    pub playback: &'a str,
    pub map: Mapping,
    pub seconds: f64,
    pub signal: Option<Signal>,
    pub generator_level: Option<GeneratorLevel>,
}
pub fn run(c: Config, options: LiveOptions<'_>, shared: &Shared) -> Result<LiveReport> {
    c.validate()?;
    options.map.validate()?;
    if !options.seconds.is_finite() || !(0.01..=86400.).contains(&options.seconds) {
        return Err("live duration must be 0.01..86400 seconds".into());
    }
    if options.generator_level.is_some() && options.signal.is_none() {
        return Err("generator level requires --signal".into());
    }
    let total = (options.seconds * c.sample_rate as f64).round() as u64;
    let mut generator = options
        .signal
        .map(|s| {
            Generator::with_level(
                s,
                c.sample_rate,
                total,
                options.generator_level.unwrap_or_default(),
            )
        })
        .transpose()?;
    let (capture, cn) = open(
        options.capture,
        Direction::Capture,
        options.map.capture_channels,
        &c,
    )?;
    let (playback, pn) = open(
        options.playback,
        Direction::Playback,
        options.map.playback_channels,
        &c,
    )?;
    if capture.info()?.get_card() != playback.info()?.get_card() {
        return Err("capture and playback must share one hardware clock/card".into());
    }
    eprintln!(
        "capture: {cn:?}\nplayback: {pn:?}\nlogical H-L,H-R,M-L,M-R,L-L,L-R -> {:?}",
        options.map.outputs
    );
    let block = cn.period.min(pn.period);
    let mut config = c;
    config.max_block = block;
    let mut engine = Engine::new(config)?;
    shared.actual_block.store(block as u32, Ordering::Release);
    let mut input = vec![[0.; 2]; block];
    let mut output = vec![[0.; 6]; block];
    let cb = cn.channels * cn.format.width();
    let pb = pn.channels * pn.format.width();
    let mut raw_in = vec![0; block * cb];
    let mut raw_out = vec![0; block * pb];
    let mut prime = vec![0; (pn.buffer - pn.period) * pb];
    let mut report = LiveReport::default();
    let mut blocks = 0u64;
    let mut started = false;
    let result: Result<()> = (|| {
        transfer(&playback, &mut prime, pb, false, shared)?;
        playback.start()?;
        capture.start()?;
        started = true;
        while report.frames < total && !shared.stop.load(Ordering::Relaxed) {
            transfer(&capture, &mut raw_in, cb, true, shared)?;
            for (frame, bytes) in input.iter_mut().zip(raw_in.chunks_exact(cb)) {
                for (ch, sample) in frame.iter_mut().enumerate() {
                    let start = options.map.inputs[ch] * cn.format.width();
                    *sample = cn.format.decode(&bytes[start..]);
                }
            }
            for frame in &input {
                for (ch, x) in frame.iter().enumerate() {
                    report.input_peaks[ch] = report.input_peaks[ch].max(x.abs());
                }
            }
            if let Some(g) = &mut generator {
                shared.generator.service(g);
                g.fill(&mut input);
            }
            let bits = shared.mutes.load(Ordering::Relaxed);
            engine.set_mutes(std::array::from_fn(|ch| bits & (1 << ch) != 0));
            let start = Instant::now();
            shared.controls.service(&mut engine);
            engine.render(&input, &mut output)?;
            shared.transitioning.store(engine.busy(), Ordering::Relaxed);
            let us = start.elapsed().as_secs_f64() * 1e6;
            report.max_render_us = report.max_render_us.max(us);
            report.mean_render_us += us;
            blocks += 1;
            if engine.faulted() {
                return Err("numerical fault; restart required".into());
            }
            for (a, x) in shared.input.iter().zip(engine.meters.input_peak) {
                a.store(x.to_bits(), Ordering::Relaxed);
            }
            for (a, x) in shared.output.iter().zip(engine.meters.output_peak) {
                a.store(x.to_bits(), Ordering::Relaxed);
            }
            for (a, x) in shared.reduction.iter().zip(
                engine
                    .meters
                    .reduction_db
                    .into_iter()
                    .chain([engine.meters.compressor_db]),
            ) {
                a.store(x.to_bits(), Ordering::Relaxed);
            }
            shared.clips.store(
                u8::from(engine.meters.clips[0]) | (u8::from(engine.meters.clips[1]) << 1),
                Ordering::Relaxed,
            );
            shared.blocks.fetch_add(1, Ordering::Relaxed);
            for frame in &output {
                for (ch, sample) in frame.iter().enumerate() {
                    report.output_peaks[ch] = report.output_peaks[ch].max(sample.abs());
                }
            }
            options.map.pack(&output, pn.format, &mut raw_out)?;
            transfer(&playback, &mut raw_out, pb, false, shared)?;
            report.frames += block as u64;
        }
        Ok(())
    })();
    let normal_stop = result.is_ok() || result.as_ref().is_err_and(|e| e.is::<Stopped>());
    let result = if normal_stop && !started {
        Ok(())
    } else if normal_stop {
        // Keep the transport running just long enough to ramp and flush silence.
        // Ignore the already-requested stop during this bounded cleanup, but
        // preserve hardware faults and the no-progress deadline.
        let cleanup = Shared::default();
        engine.set_mutes([true; 6]);
        (|| -> Result<()> {
            for _ in 0..((c.sample_rate as usize / 100 + pn.buffer) / block + 1) {
                transfer(&capture, &mut raw_in, cb, true, &cleanup)?;
                for (frame, bytes) in input.iter_mut().zip(raw_in.chunks_exact(cb)) {
                    for (ch, sample) in frame.iter_mut().enumerate() {
                        *sample = cn
                            .format
                            .decode(&bytes[options.map.inputs[ch] * cn.format.width()..]);
                    }
                }
                if let Some(g) = &mut generator {
                    g.fill(&mut input);
                }
                engine.render(&input, &mut output)?;
                options.map.pack(&output, pn.format, &mut raw_out)?;
                transfer(&playback, &mut raw_out, pb, false, &cleanup)?;
            }
            Ok(())
        })()
    } else {
        result
    };
    let _ = capture.drop();
    let _ = playback.drop();
    if let Err(e) = result {
        if e.downcast_ref::<alsa::Error>()
            .is_some_and(|a| classify(a.errno()) == TransferAction::Xrun)
        {
            report.xruns += 1;
        }
        shared.fault.store(true, Ordering::Relaxed);
        shared.mutes.store(63, Ordering::Relaxed);
        report.fault = Some(e.to_string());
    }
    for value in &shared.output {
        value.store(0, Ordering::Relaxed);
    }
    shared.mutes.store(63, Ordering::Relaxed);
    report.mean_render_us /= blocks.max(1) as f64;
    Ok(report)
}
