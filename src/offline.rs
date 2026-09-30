//! Deterministic sources and inspectable six-channel IEEE float WAV artifacts.
use crate::{config::Config, dsp::Engine};
use std::{error::Error, f64::consts::PI, path::Path, time::Instant};
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
#[derive(Clone, Copy, Debug)]
pub enum Signal {
    Silence,
    Impulse,
    Sine(f64),
    Sweep,
    Noise,
    Pink,
}
impl Signal {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "silence" => Self::Silence,
            "impulse" => Self::Impulse,
            "sweep" => Self::Sweep,
            "noise" => Self::Noise,
            "pink" => Self::Pink,
            _ if s.starts_with("sine:") => {
                let hz: f64 = s[5..].parse()?;
                if !hz.is_finite() || hz <= 0. {
                    return Err("invalid sine frequency".into());
                }
                Self::Sine(hz)
            }
            _ => {
                return Err(
                    "source: silence, impulse, sweep, noise, pink, sine:HZ or WAV path".into(),
                );
            }
        })
    }
}
/// Runtime-only source peak bound in dBFS, before program processing.
/// Noise RMS and the peak of a finite noise record are lower than this bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorLevel {
    dbfs: f64,
    scale: f64,
}
impl GeneratorLevel {
    pub fn new(dbfs: f64) -> Result<Self> {
        if !dbfs.is_finite() || !(-60. ..=0.).contains(&dbfs) {
            return Err("generator level must be finite and -60..0 dBFS".into());
        }
        Ok(Self {
            dbfs,
            scale: 10_f64.powf((dbfs + 20.) / 20.),
        })
    }
    pub(crate) fn scale(self) -> f64 {
        self.scale
    }
    pub fn dbfs(self) -> f64 {
        self.dbfs
    }
}
impl Default for GeneratorLevel {
    fn default() -> Self {
        Self {
            dbfs: -20.,
            scale: 1.,
        }
    }
}
pub struct Generator {
    signal: Signal,
    level_scale: f64,
    level_target: f64,
    level_step: f64,
    level_remaining: u32,
    rate: u32,
    frame: u64,
    total: u64,
    random: u64,
    pink: Pink,
}

// Voss-McCartney octave rows plus a full-rate white component. Integer sums
// avoid accumulated rounding drift. Each of the 17 terms is in [-2^23, 2^23).
// See docs/DSP.md for the approximation, normalization and source attribution.
#[derive(Default)]
struct Pink {
    rows: [i64; 16],
    sum: i64,
    counter: u16,
}
fn random_next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
fn pink_white(state: &mut u64) -> i64 {
    (random_next(state) >> 40) as i64 - (1 << 23)
}
impl Pink {
    fn next(&mut self, random: &mut u64) -> f64 {
        self.counter = self.counter.wrapping_add(1);
        let row = self.counter.trailing_zeros() as usize;
        // At wrap there is no row 16. Retain all rows for this sample.
        if row < self.rows.len() {
            let next = pink_white(random);
            self.sum += next - self.rows[row];
            self.rows[row] = next;
        }
        (self.sum + pink_white(random)) as f64 * (0.1 / (17. * (1_u64 << 23) as f64))
    }
}
impl Generator {
    pub fn new(signal: Signal, rate: u32, total: u64) -> Result<Self> {
        Self::with_level(signal, rate, total, GeneratorLevel::default())
    }
    pub fn with_level(
        signal: Signal,
        rate: u32,
        total: u64,
        level: GeneratorLevel,
    ) -> Result<Self> {
        if rate == 0
            || total == 0
            || matches!(signal, Signal::Sine(hz) if !hz.is_finite() || hz <= 0. || hz >= rate as f64/2.)
        {
            return Err("invalid generator rate, duration or frequency".into());
        }
        let mut generator = Self {
            signal,
            // Preserve the existing -20 dBFS samples exactly at the default.
            level_scale: level.scale(),
            level_target: level.scale(),
            level_step: 0.,
            level_remaining: 0,
            rate,
            frame: 0,
            total,
            random: 0x123456789abcdef,
            pink: Pink::default(),
        };
        if matches!(signal, Signal::Pink) {
            for row in &mut generator.pink.rows {
                *row = pink_white(&mut generator.random);
                generator.pink.sum += *row;
            }
        }
        Ok(generator)
    }
    /// Retarget from the current gain; sequence and source history are untouched.
    pub(crate) fn ramp_scale(&mut self, scale: f64) {
        if scale != self.level_target {
            self.level_target = scale;
            self.level_remaining = (self.rate / 200).max(1);
            self.level_step = (scale - self.level_scale) / self.level_remaining as f64;
        }
    }
    pub fn fill(&mut self, buffer: &mut [[f32; 2]]) {
        for frame in buffer {
            let t = self.frame as f64 / self.rate as f64;
            let x = match self.signal {
                Signal::Silence => 0.,
                // 10 ms lead-in lets startup mute ramps finish before the impulse.
                Signal::Impulse => {
                    if self.frame == self.rate as u64 / 100 {
                        0.1
                    } else {
                        0.
                    }
                }
                Signal::Sine(hz) => 0.1 * (2. * PI * hz * t).sin(),
                Signal::Sweep => {
                    let duration = self.total as f64 / self.rate as f64;
                    let k = ((self.rate as f64 * 0.4) / 20.).ln() / duration;
                    0.1 * (2. * PI * 20. * (k * t).exp_m1() / k).sin()
                }
                Signal::Noise => {
                    0.1 * (2. * (random_next(&mut self.random) as f64 / u64::MAX as f64) - 1.)
                }
                Signal::Pink => self.pink.next(&mut self.random),
            };
            if self.level_remaining > 0 {
                self.level_remaining -= 1;
                self.level_scale = if self.level_remaining == 0 {
                    self.level_target
                } else {
                    self.level_scale + self.level_step
                };
            }
            *frame = [(x * self.level_scale) as f32; 2];
            self.frame += 1;
        }
    }
}
#[derive(Debug)]
pub struct RenderReport {
    pub frames: u64,
    pub max_render_us: f64,
    pub mean_render_us: f64,
    pub peaks: [f32; 6],
}
pub fn render(
    c: Config,
    source: &str,
    destination: impl AsRef<Path>,
    seconds: f64,
) -> Result<RenderReport> {
    render_with_level(c, source, destination, seconds, None)
}
/// An explicit level applies only to generated sources, never to input WAV audio.
pub fn render_with_level(
    c: Config,
    source: &str,
    destination: impl AsRef<Path>,
    seconds: f64,
    level: Option<GeneratorLevel>,
) -> Result<RenderReport> {
    c.validate()?;
    if !seconds.is_finite() || !(0.001..=3600.).contains(&seconds) {
        return Err("duration must be 0.001..3600 seconds".into());
    }
    let total = (seconds * c.sample_rate as f64).round() as u64;
    let mut generator = Signal::parse(source)
        .ok()
        .map(|s| Generator::with_level(s, c.sample_rate, total, level.unwrap_or_default()))
        .transpose()?;
    if level.is_some() && generator.is_none() {
        return Err("generator level requires a generated source, not a WAV path".into());
    }
    let mut reader = if generator.is_none() {
        Some(hound::WavReader::open(source)?)
    } else {
        None
    };
    if let Some(r) = &reader {
        let s = r.spec();
        if s.channels != 2 || s.sample_rate != c.sample_rate {
            return Err("WAV input must have two channels and the preset sample rate".into());
        }
    }
    if reader.is_some()
        && destination.as_ref().exists()
        && std::fs::canonicalize(source)? == std::fs::canonicalize(destination.as_ref())?
    {
        return Err("input and output WAV paths must differ".into());
    }
    let spec = hound::WavSpec {
        channels: 6,
        sample_rate: c.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut engine = Engine::new(c)?;
    engine.set_mutes([false; 6]);
    let mut input = vec![[0.; 2]; c.max_block];
    let mut output = vec![[0.; 6]; c.max_block];
    let mut writer = hound::WavWriter::create(destination, spec)?;
    let mut report = RenderReport {
        frames: 0,
        max_render_us: 0.,
        mean_render_us: 0.,
        peaks: [0.; 6],
    };
    let mut blocks = 0;
    while report.frames < total {
        let mut n = ((total - report.frames) as usize).min(c.max_block);
        if let Some(g) = &mut generator {
            g.fill(&mut input[..n]);
        }
        if let Some(r) = &mut reader {
            let spec = r.spec();
            let mut got = 0;
            for frame in &mut input[..n] {
                let mut ended = false;
                for (ch, sample) in frame.iter_mut().enumerate() {
                    let value = match spec.sample_format {
                        hound::SampleFormat::Float => r.samples::<f32>().next().transpose()?,
                        hound::SampleFormat::Int => {
                            r.samples::<i32>().next().transpose()?.map(|x| {
                                (x as f64 / 2_f64.powi(spec.bits_per_sample as i32 - 1)) as f32
                            })
                        }
                    };
                    if let Some(x) = value {
                        *sample = x;
                    } else {
                        if ch != 0 {
                            return Err("incomplete WAV frame".into());
                        }
                        ended = true;
                        break;
                    }
                }
                if ended {
                    break;
                }
                got += 1;
            }
            n = got;
        }
        if n == 0 {
            break;
        }
        let start = Instant::now();
        engine.render(&input[..n], &mut output[..n])?;
        let us = start.elapsed().as_secs_f64() * 1e6;
        report.max_render_us = report.max_render_us.max(us);
        report.mean_render_us += us;
        blocks += 1;
        if engine.faulted() {
            return Err("numerical fault in offline source or processing".into());
        }
        for frame in &output[..n] {
            for (ch, &sample) in frame.iter().enumerate() {
                writer.write_sample(sample)?;
                report.peaks[ch] = report.peaks[ch].max(sample.abs());
            }
        }
        report.frames += n as u64;
    }
    writer.finalize()?;
    report.mean_render_us /= blocks.max(1) as f64;
    Ok(report)
}
