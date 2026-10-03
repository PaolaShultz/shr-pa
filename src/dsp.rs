//! Synchronous bounded 2 x 6 DSP. Construction is the only allocating operation.
use crate::config::{Config, Crossover, Edge, EqBand, EqKind, Family, GEQ_HZ, InputMode, Layout};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

pub const OUTPUT_NAMES: [&str; 6] = ["H-L", "H-R", "M-L", "M-R", "L-L", "L-R"];
#[derive(Clone, Copy, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}
impl Biquad {
    fn normalized(b: [f64; 3], a: [f64; 3]) -> Self {
        Self {
            b: b.map(|v| v / a[0]),
            a: [a[1] / a[0], a[2] / a[0]],
            z: [0.; 2],
        }
    }
    fn edge(hz: f64, rate: u32, high: bool) -> Self {
        Self::second_order(hz, rate, high, FRAC_1_SQRT_2)
    }
    fn second_order(hz: f64, rate: u32, high: bool, q: f64) -> Self {
        let w = 2. * PI * hz / rate as f64;
        let c = w.cos();
        let alpha = w.sin() / (2. * q);
        let b = if high {
            [(1. + c) / 2., -(1. + c), (1. + c) / 2.]
        } else {
            [(1. - c) / 2., 1. - c, (1. - c) / 2.]
        };
        Self::normalized(b, [1. + alpha, -2. * c, 1. - alpha])
    }
    fn identity() -> Self {
        Self::normalized([1., 0., 0.], [1., 0., 0.])
    }
    /// Bilinear transform of an order-N Butterworth, or two identical
    /// Butterworth prototypes for LR. Four second-order slots per edge.
    fn sections(edge: Edge, rate: u32, high: bool) -> [Self; 4] {
        let mut sections = [Self::identity(); 4];
        if edge.bypass {
            return sections;
        }
        let repeat = if edge.family == Family::LinkwitzRiley {
            2
        } else {
            1
        };
        let order = edge.slope as usize / 6 / repeat;
        let mut slot = 0;
        for _ in 0..repeat {
            if order % 2 == 1 {
                let k = (PI * edge.hz / rate as f64).tan();
                let b = if high { [1., -1., 0.] } else { [k, k, 0.] };
                sections[slot] = Self::normalized(b, [1. + k, k - 1., 0.]);
                slot += 1;
            }
            for j in 0..order / 2 {
                let q = 1. / (2. * ((2 * j + 1) as f64 * PI / (2 * order) as f64).sin());
                sections[slot] = Self::second_order(edge.hz, rate, high, q);
                slot += 1;
            }
        }
        sections
    }
    fn valid(&self) -> bool {
        self.a.iter().chain(self.b.iter()).all(|x| x.is_finite())
            && self.a[1].abs() < 1.
            && 1. + self.a[0] + self.a[1] > 0.
            && 1. - self.a[0] + self.a[1] > 0.
    }
    fn allpass(hz: f64, rate: u32) -> Self {
        let w = 2. * PI * hz / rate as f64;
        let alpha = w.sin() / (2. * FRAC_1_SQRT_2);
        Self::normalized(
            [1. - alpha, -2. * w.cos(), 1. + alpha],
            [1. + alpha, -2. * w.cos(), 1. - alpha],
        )
    }
    fn eq(e: EqBand, rate: u32) -> Self {
        let a = 10_f64.powf(e.db / 40.);
        let w = 2. * PI * e.hz / rate as f64;
        let alpha = w.sin() / (2. * e.q);
        if e.kind != EqKind::Bell {
            let c = w.cos();
            let alpha = w.sin() / 2. * ((a + 1. / a) * (1. / e.slope - 1.) + 2.).sqrt();
            let t = 2. * a.sqrt() * alpha;
            return if e.kind == EqKind::LowShelf {
                Self::normalized(
                    [
                        a * ((a + 1.) - (a - 1.) * c + t),
                        2. * a * ((a - 1.) - (a + 1.) * c),
                        a * ((a + 1.) - (a - 1.) * c - t),
                    ],
                    [
                        (a + 1.) + (a - 1.) * c + t,
                        -2. * ((a - 1.) + (a + 1.) * c),
                        (a + 1.) + (a - 1.) * c - t,
                    ],
                )
            } else {
                Self::normalized(
                    [
                        a * ((a + 1.) + (a - 1.) * c + t),
                        -2. * a * ((a - 1.) + (a + 1.) * c),
                        a * ((a + 1.) + (a - 1.) * c - t),
                    ],
                    [
                        (a + 1.) - (a - 1.) * c + t,
                        2. * ((a - 1.) - (a + 1.) * c),
                        (a + 1.) - (a - 1.) * c - t,
                    ],
                )
            };
        }
        Self::normalized(
            [1. + alpha * a, -2. * w.cos(), 1. - alpha * a],
            [1. + alpha / a, -2. * w.cos(), 1. - alpha / a],
        )
    }
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z = [
            self.b[1] * x - self.a[0] * y + self.z[1],
            self.b[2] * x - self.a[1] * y,
        ];
        for z in &mut self.z {
            if z.abs() < 1e-30 {
                *z = 0.;
            }
        }
        y
    }
}
#[derive(Clone, Copy)]
struct Filter {
    old: Biquad,
    new: Biquad,
    left: usize,
    length: usize,
}
impl Filter {
    fn new(f: Biquad) -> Self {
        Self {
            old: f,
            new: f,
            left: 0,
            length: 1,
        }
    }
    fn update(&mut self, f: Biquad, length: usize) {
        if self.new.a == f.a && self.new.b == f.b {
            return;
        }
        self.old = self.new;
        self.new = f;
        self.left = length;
        self.length = length;
    }
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.new.tick(x);
        if self.left == 0 {
            return y;
        }
        let old = self.old.tick(x);
        self.left -= 1;
        let mix = 1. - self.left as f64 / self.length as f64;
        old + (y - old) * mix
    }
}
#[derive(Clone, Copy)]
struct Smooth {
    value: f64,
    target: f64,
    step: f64,
    left: usize,
}
impl Smooth {
    fn new(value: f64) -> Self {
        Self {
            value,
            target: value,
            step: 0.,
            left: 0,
        }
    }
    fn update(&mut self, target: f64, n: usize) {
        self.target = target;
        self.step = (target - self.value) / n as f64;
        self.left = n;
    }
    fn tick(&mut self) -> f64 {
        if self.left > 0 {
            self.value += self.step;
            self.left -= 1;
            if self.left == 0 {
                self.value = self.target;
            }
        }
        self.value
    }
}
struct Delay {
    data: Vec<f64>,
    pos: usize,
    tap: usize,
    old: usize,
    left: usize,
    length: usize,
}
impl Delay {
    fn new(ms: f64, max_ms: f64, rate: u32) -> Self {
        let mut data = vec![0.; (max_ms * rate as f64 / 1000.).round() as usize + 1];
        for sample in &mut data {
            *std::hint::black_box(sample) = 0.;
        }
        let tap = (ms * rate as f64 / 1000.).round() as usize;
        Self {
            data,
            pos: 0,
            tap,
            old: tap,
            left: 0,
            length: 1,
        }
    }
    fn update(&mut self, tap: usize, n: usize) {
        if tap != self.tap {
            self.old = self.tap;
            self.tap = tap;
            self.left = n;
            self.length = n;
        }
    }
    fn tick(&mut self, x: f64) -> f64 {
        self.data[self.pos] = x;
        let get = |tap| self.data[(self.pos + self.data.len() - tap) % self.data.len()];
        let mut y = get(self.tap);
        if self.left > 0 {
            self.left -= 1;
            let mix = 1. - self.left as f64 / self.length as f64;
            y = get(self.old) * (1. - mix) + y * mix;
        }
        self.pos = (self.pos + 1) % self.data.len();
        y
    }
}
/// Validated, fixed-size transaction. Build on the controller, copy at a block boundary.
#[derive(Clone, Copy)]
pub struct Prepared {
    config: Config,
    input_eq: [[Biquad; 8]; 2],
    output_eq: [[Biquad; 8]; 6],
    geq: [[Biquad; 31]; 2],
    edges: [[Biquad; 8]; 3],
    low: Biquad,
    rest: Biquad,
    mid: Biquad,
    high: Biquad,
    compensation: Biquad,
    gains: [f64; 3],
    input_gain: f64,
    thresholds: [f64; 3],
    release: [f64; 3],
    input_tap: usize,
    output_tap: [usize; 3],
    attack: f64,
    comp_release: f64,
    pub recall: bool,
}
impl Prepared {
    pub fn new(c: Config, recall: bool) -> Result<Self, &'static str> {
        c.validate()?;
        let rate = c.sample_rate;
        let eq = |mut e: EqBand, enabled: bool| {
            if !enabled {
                e.db = 0.;
            }
            Biquad::eq(e, rate)
        };
        let mut edges = [[Biquad::identity(); 8]; 3];
        if let Crossover::Independent(pairs) = c.crossover {
            for (filters, pair) in edges.iter_mut().zip(pairs) {
                filters[..4].copy_from_slice(&Biquad::sections(pair.hp, rate, true));
                filters[4..].copy_from_slice(&Biquad::sections(pair.lp, rate, false));
            }
        }
        if !edges.iter().flatten().all(Biquad::valid) {
            return Err("unstable/nonfinite crossover coefficients");
        }
        Ok(Self {
            edges,
            input_eq: c.input_eq.map(|es| es.map(|e| eq(e, c.input_eq_enabled))),
            output_eq: std::array::from_fn(|ch| {
                c.bands[ch / 2]
                    .eq
                    .map(|e| eq(e, c.bands[ch / 2].eq_enabled))
            }),
            geq: std::array::from_fn(|ch| {
                std::array::from_fn(|band| {
                    let hz = GEQ_HZ[band];
                    // Bands above the usable range are identity; stored gains are retained.
                    eq(
                        EqBand {
                            hz: hz.min(rate as f64 * 0.45),
                            q: 4.318,
                            db: c.geq.db[if c.geq.linked { 0 } else { ch }][band],
                            ..EqBand::default()
                        },
                        c.geq.enabled && hz <= rate as f64 * 0.45,
                    )
                })
            }),
            low: Biquad::edge(c.low_hz, rate, false),
            rest: Biquad::edge(c.low_hz, rate, true),
            mid: Biquad::edge(c.high_hz, rate, false),
            high: Biquad::edge(c.high_hz, rate, true),
            compensation: Biquad::allpass(c.high_hz, rate),
            gains: std::array::from_fn(|i| {
                10_f64.powf(c.bands[i].gain_db / 20.) * if c.bands[i].inverted { -1. } else { 1. }
            }),
            input_gain: 10_f64.powf(c.input_gain_db / 20.),
            thresholds: std::array::from_fn(|i| 10_f64.powf(c.bands[i].limiter_db / 20.)),
            release: std::array::from_fn(|i| {
                (-1. / (c.bands[i].release_ms * 0.001 * rate as f64)).exp()
            }),
            input_tap: (c.input_delay_ms * rate as f64 / 1000.).round() as usize,
            output_tap: std::array::from_fn(|i| {
                (c.bands[i].delay_ms * rate as f64 / 1000.).round() as usize
            }),
            attack: (-1. / (c.compressor.attack_ms * 0.001 * rate as f64)).exp(),
            comp_release: (-1. / (c.compressor.release_ms * 0.001 * rate as f64)).exp(),
            config: c,
            recall,
        })
    }
}
/// Static compression curve: output minus input in dB, before makeup.
pub fn compression_db(level: f64, threshold: f64, ratio: f64, knee: f64) -> f64 {
    let x = level - threshold;
    let slope = 1. / ratio - 1.;
    if knee > 0. && x.abs() < knee / 2. {
        slope * (x + knee / 2.).powi(2) / (2. * knee)
    } else if x > 0. {
        slope * x
    } else {
        0.
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Meters {
    pub input_peak: [f32; 2],
    pub output_peak: [f32; 6],
    pub compressor_db: f32,
    pub reduction_db: [f32; 3],
    pub clips: [bool; 2],
    pub fault: bool,
}
pub struct Engine {
    config: Config,
    input_eq: [[Filter; 8]; 2],
    geq: [[Filter; 31]; 2],
    output_eq: [[Filter; 8]; 6],
    edges: [[Filter; 8]; 6],
    low: [[Filter; 2]; 2],
    rest: [[Filter; 2]; 2],
    mid: [[Filter; 2]; 2],
    high: [[Filter; 2]; 2],
    compensation: [Filter; 2],
    input_delay: [Delay; 2],
    output_delay: [Delay; 6],
    gains: [Smooth; 3],
    input_gain: Smooth,
    limiter_gain: [f64; 3],
    thresholds: [Smooth; 3],
    release: [Smooth; 3],
    muted: [bool; 6],
    ramps: [f64; 6],
    ramp_step: f64,
    pub meters: Meters,
    pending: Option<Prepared>,
    transition: usize,
    hold: bool,
    comp_gain_db: f64,
    comp_mix: Smooth,
    comp_threshold: Smooth,
    comp_ratio: Smooth,
    comp_knee: Smooth,
    comp_makeup: Smooth,
    comp_attack: Smooth,
    comp_release: Smooth,
}
fn cascade(filters: &mut [Filter], mut x: f64) -> f64 {
    for f in filters {
        x = f.tick(x);
    }
    x
}
// Both public sample formats share the same processing and fault path. Conversion
// happens only at the boundary; f64 hosts never pass through an f32 audio buffer.
trait Sample: Copy {
    const ZERO: Self;
    fn to_f64(self) -> f64;
    fn from_f64(value: f64) -> Self;
}
impl Sample for f32 {
    const ZERO: Self = 0.;
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    fn from_f64(value: f64) -> Self {
        value as f32
    }
}
impl Sample for f64 {
    const ZERO: Self = 0.;
    fn to_f64(self) -> f64 {
        self
    }
    fn from_f64(value: f64) -> Self {
        value
    }
}
impl Engine {
    pub fn new(c: Config) -> Result<Self, &'static str> {
        let p = Prepared::new(c, false)?;
        let rate = c.sample_rate;
        Ok(Self {
            input_eq: p.input_eq.map(|es| es.map(Filter::new)),
            output_eq: p.output_eq.map(|es| es.map(Filter::new)),
            geq: p.geq.map(|es| es.map(Filter::new)),
            edges: std::array::from_fn(|ch| p.edges[ch / 2].map(Filter::new)),
            low: [[Filter::new(p.low); 2]; 2],
            rest: [[Filter::new(p.rest); 2]; 2],
            mid: [[Filter::new(p.mid); 2]; 2],
            high: [[Filter::new(p.high); 2]; 2],
            compensation: [Filter::new(p.compensation); 2],
            input_delay: std::array::from_fn(|_| Delay::new(c.input_delay_ms, 100., rate)),
            output_delay: std::array::from_fn(|i| Delay::new(c.bands[i / 2].delay_ms, 10., rate)),
            gains: p.gains.map(Smooth::new),
            input_gain: Smooth::new(p.input_gain),
            limiter_gain: [1.; 3],
            thresholds: p.thresholds.map(Smooth::new),
            release: p.release.map(Smooth::new),
            muted: [true; 6],
            ramps: [0.; 6],
            ramp_step: 1. / (0.005 * rate as f64),
            meters: Meters::default(),
            config: c,
            pending: None,
            transition: 0,
            hold: false,
            comp_gain_db: 0.,
            comp_mix: Smooth::new(if c.compressor.enabled { 1. } else { 0. }),
            comp_threshold: Smooth::new(c.compressor.threshold_db),
            comp_ratio: Smooth::new(c.compressor.ratio),
            comp_knee: Smooth::new(c.compressor.knee_db),
            comp_makeup: Smooth::new(c.compressor.makeup_db),
            comp_attack: Smooth::new(p.attack),
            comp_release: Smooth::new(p.comp_release),
        })
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.transition > 0
    }
    pub fn reconfiguring(&self) -> bool {
        self.pending.is_some() || self.hold
    }
    /// Called once before render. Rejected transactions leave all state intact.
    pub fn apply(&mut self, p: Prepared) -> Result<(), &'static str> {
        if self.faulted() {
            return Err("fault latched; restart required");
        }
        if self.busy() {
            return Err("transition busy; retry transaction");
        }
        if p.config.sample_rate != self.config.sample_rate
            || p.config.max_block != self.config.max_block
        {
            return Err("rate/block changes require restart");
        }
        if p.recall
            || p.config.layout != self.config.layout
            || p.config.input_mode != self.config.input_mode
            || p.config.mono_bass != self.config.mono_bass
            || p.config.crossover != self.config.crossover
            || p.config.low_hz != self.config.low_hz
            || p.config.high_hz != self.config.high_hz
        {
            self.pending = Some(p);
            self.hold = true;
        } else {
            self.install(p);
        }
        Ok(())
    }
    fn install(&mut self, p: Prepared) {
        let n = (self.config.sample_rate as usize / 50).max(1);
        if std::mem::discriminant(&self.config.crossover)
            != std::mem::discriminant(&p.config.crossover)
        {
            // Mode switches are installed under mute. Only crossover state resets.
            self.low = [[Filter::new(p.low); 2]; 2];
            self.rest = [[Filter::new(p.rest); 2]; 2];
            self.mid = [[Filter::new(p.mid); 2]; 2];
            self.high = [[Filter::new(p.high); 2]; 2];
            self.compensation = [Filter::new(p.compensation); 2];
            self.edges = std::array::from_fn(|ch| p.edges[ch / 2].map(Filter::new));
        }

        for ch in 0..2 {
            for i in 0..8 {
                self.input_eq[ch][i].update(p.input_eq[ch][i], n);
            }
            for i in 0..31 {
                self.geq[ch][i].update(p.geq[ch][i], n);
            }
            for i in 0..2 {
                self.low[ch][i].update(p.low, n);
                self.rest[ch][i].update(p.rest, n);
                self.mid[ch][i].update(p.mid, n);
                self.high[ch][i].update(p.high, n);
            }
            self.compensation[ch].update(p.compensation, n);
            self.input_delay[ch].update(p.input_tap, n);
        }
        for ch in 0..6 {
            for i in 0..8 {
                self.output_eq[ch][i].update(p.output_eq[ch][i], n);
                self.edges[ch][i].update(p.edges[ch / 2][i], n);
            }
            self.output_delay[ch].update(p.output_tap[ch / 2], n);
        }
        for i in 0..3 {
            self.gains[i].update(p.gains[i], n);
            self.thresholds[i].update(p.thresholds[i], n);
            self.release[i].update(p.release[i], n);
        }
        self.input_gain.update(p.input_gain, n);
        let c = p.config.compressor;
        self.comp_mix.update(if c.enabled { 1. } else { 0. }, n);
        self.comp_threshold.update(c.threshold_db, n);
        self.comp_ratio.update(c.ratio, n);
        self.comp_knee.update(c.knee_db, n);
        self.comp_makeup.update(c.makeup_db, n);
        self.comp_attack.update(p.attack, n);
        self.comp_release.update(p.comp_release, n);
        self.config = p.config;
        self.transition = n;
    }
    pub fn set_mutes(&mut self, mutes: [bool; 6]) {
        self.muted = mutes;
    }
    pub fn faulted(&self) -> bool {
        self.meters.fault
    }
    /// Clears output on shape errors, never allocates. Faults latch silence until reconstruction.
    pub fn render(
        &mut self,
        input: &[[f32; 2]],
        output: &mut [[f32; 6]],
    ) -> Result<(), &'static str> {
        self.render_samples(input, output)
    }
    /// Native f64 boundary, with the same processing, bounds and latched faults as render.
    pub fn render_f64(
        &mut self,
        input: &[[f64; 2]],
        output: &mut [[f64; 6]],
    ) -> Result<(), &'static str> {
        self.render_samples(input, output)
    }
    fn render_samples<S: Sample>(
        &mut self,
        input: &[[S; 2]],
        output: &mut [[S; 6]],
    ) -> Result<(), &'static str> {
        output.fill([S::ZERO; 6]);
        if input.len() != output.len() || input.len() > self.config.max_block {
            return Err("render shape exceeds prepared block or differs");
        }
        if !self.faulted() && self.pending.is_some() && self.ramps.iter().all(|&r| r == 0.) {
            let p = self.pending.take().unwrap();
            self.install(p);
        }
        self.meters.compressor_db = 0.;
        self.meters.input_peak = [0.; 2];
        self.meters.output_peak = [0.; 6];
        self.meters.reduction_db = [0.; 3];
        for (frame, out) in input.iter().zip(output.iter_mut()) {
            if self.meters.fault {
                continue;
            }
            let mut x = frame.map(S::to_f64);
            if x.iter().any(|x| !x.is_finite()) {
                self.meters.fault = true;
                continue;
            }
            for (ch, &x) in x.iter().enumerate() {
                self.meters.input_peak[ch] = self.meters.input_peak[ch].max(x.abs() as f32);
                self.meters.clips[ch] |= x.abs() >= 1.;
            }
            if self.config.input_mode == InputMode::MonoLeft {
                x[1] = x[0];
            }
            let input_gain = self.input_gain.tick();
            for (ch, x) in x.iter_mut().enumerate() {
                *x = cascade(
                    &mut self.input_eq[ch],
                    cascade(&mut self.geq[ch], *x * input_gain),
                );
            }
            let peak = x[0].abs().max(x[1].abs()).max(1e-30);
            let target = compression_db(
                20. * peak.log10(),
                self.comp_threshold.tick(),
                self.comp_ratio.tick(),
                self.comp_knee.tick(),
            );
            let attack = self.comp_attack.tick();
            let release = self.comp_release.tick();
            let coef = if target < self.comp_gain_db {
                attack
            } else {
                release
            };
            self.comp_gain_db = target + coef * (self.comp_gain_db - target);
            let mix = self.comp_mix.tick();
            let cg =
                1. + (10_f64.powf((self.comp_gain_db + self.comp_makeup.tick()) / 20.) - 1.) * mix;
            self.meters.compressor_db = self
                .meters
                .compressor_db
                .max((-self.comp_gain_db * mix) as f32);
            for (ch, x) in x.iter_mut().enumerate() {
                *x = self.input_delay[ch].tick(*x * cg);
            }
            let gains = self.gains.each_mut().map(Smooth::tick);
            let bass = if self.config.mono_bass {
                [(x[0] + x[1]) * 0.5; 2]
            } else {
                x
            };
            let mut y = [0.; 6];
            for ch in 0..2 {
                if matches!(self.config.crossover, Crossover::Independent(_)) {
                    for pair in 0..3 {
                        let source = if pair == 2
                            && matches!(
                                self.config.layout,
                                Layout::TwoWay | Layout::ThreeWay | Layout::FourPlusSubs
                            ) {
                            bass[ch]
                        } else {
                            x[ch]
                        };
                        y[pair * 2 + ch] = cascade(&mut self.edges[pair * 2 + ch], source);
                    }
                    continue;
                }
                match self.config.layout {
                    Layout::FullRange | Layout::External => y[ch] = x[ch],
                    Layout::SixFullRange => {
                        y[ch] = x[ch];
                        y[ch + 2] = x[ch];
                        y[ch + 4] = x[ch];
                    }
                    Layout::TwoWay | Layout::FourPlusSubs | Layout::ThreeWay => {
                        let low = cascade(&mut self.low[ch], bass[ch]);
                        let rest = cascade(&mut self.rest[ch], x[ch]);
                        if self.config.layout == Layout::ThreeWay {
                            y[ch + 4] = self.compensation[ch].tick(low);
                            y[ch + 2] = cascade(&mut self.mid[ch], rest);
                            y[ch] = cascade(&mut self.high[ch], rest);
                        } else {
                            y[ch + 4] = low;
                            y[ch] = rest;
                            if self.config.layout == Layout::FourPlusSubs {
                                y[ch + 2] = rest;
                            }
                        }
                    }
                }
            }
            for (ch, y) in y.iter_mut().enumerate() {
                *y = cascade(&mut self.output_eq[ch], *y * gains[ch / 2]);
            }
            if y.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
                self.meters.fault = true;
                continue;
            }
            for pair in 0..3 {
                let threshold = self.thresholds[pair].tick();
                let release = self.release[pair].tick();
                let peak = y[pair * 2].abs().max(y[pair * 2 + 1].abs());
                let required = if peak > threshold {
                    threshold / peak
                } else {
                    1.
                };
                self.limiter_gain[pair] =
                    required.min(1. - (1. - self.limiter_gain[pair]) * release);
                self.meters.reduction_db[pair] = self.meters.reduction_db[pair]
                    .max((-20. * self.limiter_gain[pair].log10()) as f32);
                let delayed: [f64; 2] = std::array::from_fn(|side| {
                    let ch = pair * 2 + side;
                    self.output_delay[ch].tick(y[ch] * self.limiter_gain[pair])
                });
                // A lowered ceiling must also protect samples already in delay history.
                let peak = delayed[0].abs().max(delayed[1].abs());
                let guard = if peak > threshold {
                    threshold / peak
                } else {
                    1.
                };
                self.meters.reduction_db[pair] =
                    self.meters.reduction_db[pair].max((-20. * guard.log10()) as f32);
                for (side, delayed) in delayed.iter().enumerate() {
                    let ch = pair * 2 + side;
                    let target = if self.muted[ch] || self.hold { 0. } else { 1. };
                    self.ramps[ch] +=
                        (target - self.ramps[ch]).clamp(-self.ramp_step, self.ramp_step);
                    let sample = delayed * guard * self.ramps[ch];
                    let active = self.config.active_pair(pair);
                    out[ch] = if active { S::from_f64(sample) } else { S::ZERO };
                    self.meters.output_peak[ch] =
                        self.meters.output_peak[ch].max(out[ch].to_f64().abs() as f32);
                }
            }
            if self.transition > 0 {
                self.transition -= 1;
                if self.transition == 0 {
                    self.hold = false;
                }
            }
        }
        if self.meters.fault {
            output.fill([S::ZERO; 6]);
            self.meters.output_peak = [0.; 6];
        }
        Ok(())
    }
}
