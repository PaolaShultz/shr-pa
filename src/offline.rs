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
}
impl Signal {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "silence" => Self::Silence,
            "impulse" => Self::Impulse,
            "sweep" => Self::Sweep,
            "noise" => Self::Noise,
            _ if s.starts_with("sine:") => {
                let hz: f64 = s[5..].parse()?;
                if !hz.is_finite() || hz <= 0. {
                    return Err("invalid sine frequency".into());
                }
                Self::Sine(hz)
            }
            _ => return Err("source: silence, impulse, sweep, noise, sine:HZ or WAV path".into()),
        })
    }
}
pub struct Generator {
    signal: Signal,
    rate: u32,
    frame: u64,
    total: u64,
    random: u64,
}
impl Generator {
    pub fn new(signal: Signal, rate: u32, total: u64) -> Result<Self> {
        if rate == 0
            || total == 0
            || matches!(signal, Signal::Sine(hz) if !hz.is_finite() || hz <= 0. || hz >= rate as f64/2.)
        {
            return Err("invalid generator rate, duration or frequency".into());
        }
        Ok(Self {
            signal,
            rate,
            frame: 0,
            total,
            random: 0x123456789abcdef,
        })
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
                    self.random ^= self.random << 13;
                    self.random ^= self.random >> 7;
                    self.random ^= self.random << 17;
                    0.1 * (2. * (self.random as f64 / u64::MAX as f64) - 1.)
                }
            };
            *frame = [x as f32; 2];
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
    c.validate()?;
    if !seconds.is_finite() || !(0.001..=3600.).contains(&seconds) {
        return Err("duration must be 0.001..3600 seconds".into());
    }
    let total = (seconds * c.sample_rate as f64).round() as u64;
    let mut generator = Signal::parse(source)
        .ok()
        .map(|s| Generator::new(s, c.sample_rate, total))
        .transpose()?;
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
