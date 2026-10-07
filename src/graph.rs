//! Configurable owner-native PA graph. Preparation allocates; rendering does not.
use crate::config::{Band, Compressor, Config, EqBand, GEQ_HZ};
use crate::dsp::{Biquad, Delay, compression_db};
use crate::live_eq::{EqPatch, EqProgress, EqSettings, PreparedEq};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Implementation resource budgets, not product channel limits.
pub const MAX_PORTS: usize = 4096;
pub const MAX_OPERATIONS: usize = 65536;
pub const MAX_DELAY_SAMPLES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub gain_db: f64,
    pub delay_ms: f64,
    pub eq_enabled: bool,
    pub eq: [EqBand; 8],
    pub geq_enabled: bool,
    pub geq_db: [f64; 31],
    pub compressor: Compressor,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            gain_db: 0.,
            delay_ms: 0.,
            eq_enabled: true,
            eq: [EqBand::default(); 8],
            geq_enabled: false,
            geq_db: [0.; 31],
            compressor: Compressor::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    Input(usize),
    Node(usize),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub source: Source,
    pub weight: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub routes: Vec<Route>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Output {
    /// None is an explicitly silent output; outputs have one unique vector index.
    pub source: Option<Source>,
    /// Ascending LR24 split frequencies. Band 0 is lowest; len is highest.
    pub splits_hz: Vec<f64>,
    pub band: usize,
    pub processing: Band,
    pub muted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphConfig {
    pub version: u32,
    pub sample_rate: u32,
    pub max_block: usize,
    pub inputs: Vec<Input>,
    pub nodes: Vec<Node>,
    pub outputs: Vec<Output>,
}
impl GraphConfig {
    pub fn stereo(splits_hz: Vec<f64>) -> Self {
        let outputs = (0..=splits_hz.len())
            .rev()
            .flat_map(|band| (0..2).map(move |ch| (band, ch)))
            .map(|(band, ch)| Output {
                source: Some(Source::Input(ch)),
                splits_hz: splits_hz.clone(),
                band,
                processing: Band::default(),
                muted: false,
            })
            .collect();
        Self {
            version: 2,
            sample_rate: 48000,
            max_block: 256,
            inputs: vec![Input::default(); 2],
            nodes: vec![],
            outputs,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 2 {
            return Err("graph schema version must be 2");
        }
        if self.inputs.is_empty()
            || self.outputs.is_empty()
            || self.inputs.len() > MAX_PORTS
            || self.outputs.len() > MAX_PORTS
            || self.nodes.len() > MAX_PORTS
        {
            return Err("port/node descriptor budget 4096 exceeded or empty ports");
        }
        let mut base = Config {
            sample_rate: self.sample_rate,
            max_block: self.max_block,
            low_hz: 100.,
            high_hz: 1000.,
            ..Config::default()
        };
        base.validate()?;
        for input in &self.inputs {
            base.input_gain_db = input.gain_db;
            base.input_delay_ms = input.delay_ms;
            base.input_eq = [input.eq; 2];
            base.geq.db = [input.geq_db; 2];
            base.compressor = input.compressor;
            base.validate()?;
        }
        let valid_source = |s: Source, count: usize| match s {
            Source::Input(i) => i < self.inputs.len(),
            Source::Node(i) => i < count,
        };
        let mut operations = self.inputs.len() * 40;
        for (index, node) in self.nodes.iter().enumerate() {
            if node.routes.is_empty() {
                return Err("empty sum node");
            }
            for route in &node.routes {
                if !valid_source(route.source, index) {
                    return Err(
                        "invalid or forward/cyclic node reference; nodes must be topological",
                    );
                }
                if !route.weight.is_finite() || route.weight.abs() > 16. {
                    return Err("route weight must be finite -16..16; no normalization");
                }
            }
            operations = operations
                .checked_add(node.routes.len())
                .ok_or("operation overflow")?;
        }
        for output in &self.outputs {
            if output
                .source
                .is_some_and(|s| !valid_source(s, self.nodes.len()))
            {
                return Err("invalid output source");
            }
            if output.band > output.splits_hz.len() {
                return Err("invalid crossover band");
            }
            let mut previous = 0.;
            for &hz in &output.splits_hz {
                if !hz.is_finite()
                    || hz < 16.
                    || hz > (self.sample_rate as f64 * 0.45).min(20000.)
                    || hz <= previous
                {
                    return Err("splits must increase within 16..min(20000,0.45*rate)");
                }
                previous = hz;
            }
            base.bands = [output.processing; 3];
            base.validate()?;
            operations = operations
                .checked_add(
                    output
                        .splits_hz
                        .len()
                        .checked_mul(2)
                        .ok_or("operation overflow")?
                        + 12,
                )
                .ok_or("operation overflow")?;
        }
        if operations > MAX_OPERATIONS {
            return Err("65536 per-frame DSP operation admission budget exceeded");
        }
        let delays = self
            .inputs
            .len()
            .checked_mul((self.sample_rate as usize).div_ceil(10) + 1)
            .and_then(|n| {
                self.outputs
                    .len()
                    .checked_mul((self.sample_rate as usize).div_ceil(100) + 1)
                    .and_then(|o| n.checked_add(o))
            })
            .ok_or("delay budget overflow")?;
        if delays > MAX_DELAY_SAMPLES {
            return Err("64 MiB delay-storage admission budget exceeded");
        }
        Ok(())
    }
}
struct InputState {
    eq: Vec<Biquad>,
    delay: Delay,
    gain: f64,
    compressor: Compressor,
    attack: f64,
    release: f64,
    reduction: f64,
}
struct OutputState {
    filters: Vec<Biquad>,
    eq: Vec<Biquad>,
    delay: Delay,
    gain: f64,
    ceiling: f64,
    release: f64,
    limiter: f64,
}
/// State is owned by one worker, never concurrently queried or mutated.
pub struct Graph {
    pub(crate) config: GraphConfig,
    inputs: Vec<InputState>,
    outputs: Vec<OutputState>,
    values: Vec<f64>,
    pub(crate) fault: bool,
    pub(crate) muted: bool,
    pub(crate) ramp: f64,
    pub(crate) instance: u64,
    pub(crate) eq_generation: u64,
    pub(crate) live_eq: Option<Box<PreparedEq>>,
    pub(crate) eq_aborted: Option<[EqSettings; 2]>,
}
impl Graph {
    pub fn config(&self) -> &GraphConfig {
        &self.config
    }
    pub fn faulted(&self) -> bool {
        self.fault
    }
    pub fn prepare(config: GraphConfig) -> Result<Self, &'static str> {
        config.validate()?;
        let rate = config.sample_rate;
        let inputs = config
            .inputs
            .iter()
            .map(|c| {
                let mut eq = Vec::with_capacity(39);
                for e in c.eq {
                    eq.push(if c.eq_enabled {
                        Biquad::eq(e, rate)
                    } else {
                        Biquad::identity()
                    });
                }
                for (i, hz) in GEQ_HZ.iter().enumerate() {
                    eq.push(if c.geq_enabled && *hz <= rate as f64 * 0.45 {
                        Biquad::eq(
                            EqBand {
                                hz: *hz,
                                q: 4.318,
                                db: c.geq_db[i],
                                ..EqBand::default()
                            },
                            rate,
                        )
                    } else {
                        Biquad::identity()
                    });
                }
                InputState {
                    eq,
                    delay: Delay::new(c.delay_ms, 100., rate),
                    gain: 10_f64.powf(c.gain_db / 20.),
                    compressor: c.compressor,
                    attack: (-1. / (c.compressor.attack_ms * 0.001 * rate as f64)).exp(),
                    release: (-1. / (c.compressor.release_ms * 0.001 * rate as f64)).exp(),
                    reduction: 0.,
                }
            })
            .collect();
        let mut outputs = Vec::with_capacity(config.outputs.len());
        for c in &config.outputs {
            let mut filters = Vec::new();
            for (index, &hz) in c.splits_hz.iter().enumerate() {
                if index < c.band {
                    filters.extend([Biquad::edge(hz, rate, true); 2]);
                } else if index == c.band {
                    filters.extend([Biquad::edge(hz, rate, false); 2]);
                } else {
                    filters.push(Biquad::allpass(hz, rate));
                }
            }
            if !filters.iter().all(Biquad::valid) {
                return Err("unstable crossover");
            }
            let p = c.processing;
            outputs.push(OutputState {
                filters,
                eq: p
                    .eq
                    .iter()
                    .map(|e| {
                        if p.eq_enabled {
                            Biquad::eq(*e, rate)
                        } else {
                            Biquad::identity()
                        }
                    })
                    .collect(),
                delay: Delay::new(p.delay_ms, 10., rate),
                gain: 10_f64.powf(p.gain_db / 20.) * if p.inverted { -1. } else { 1. },
                ceiling: 10_f64.powf(p.limiter_db / 20.),
                release: (-1. / (p.release_ms * 0.001 * rate as f64)).exp(),
                limiter: 1.,
            });
        }
        Ok(Self {
            values: vec![0.; config.inputs.len() + config.nodes.len()],
            config,
            inputs,
            outputs,
            fault: false,
            muted: true,
            ramp: 0.,
            instance: NEXT_INSTANCE
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| "owner instance counter exhausted")?,
            eq_generation: 0,
            live_eq: None,
            eq_aborted: None,
        })
    }
    pub(crate) fn overlaps(&self, span: (usize, usize)) -> bool {
        fn vector<T>(v: &Vec<T>, span: (usize, usize)) -> bool {
            let a = v.as_ptr() as usize;
            a < span.1 && span.0 < a + v.capacity() * std::mem::size_of::<T>()
        }
        let overlap = |a: (usize, usize)| a.0 < span.1 && span.0 < a.1;
        self.live_eq.as_ref().is_some_and(|p| p.overlaps(span))
            || vector(&self.config.inputs, span)
            || vector(&self.config.nodes, span)
            || vector(&self.config.outputs, span)
            || self.config.nodes.iter().any(|n| vector(&n.routes, span))
            || self
                .config
                .outputs
                .iter()
                .any(|o| vector(&o.splits_hz, span))
            || vector(&self.inputs, span)
            || vector(&self.outputs, span)
            || vector(&self.values, span)
            || self
                .inputs
                .iter()
                .any(|i| vector(&i.eq, span) || overlap(i.delay.owned_span()))
            || self.outputs.iter().any(|o| {
                vector(&o.filters, span) || vector(&o.eq, span) || overlap(o.delay.owned_span())
            })
    }
    /// Separate optional admission; legacy graph admission remains unchanged.
    pub fn live_eq_eligible(&self) -> bool {
        let work = self.config.inputs.len() * 40
            + self
                .config
                .nodes
                .iter()
                .map(|n| n.routes.len())
                .sum::<usize>()
            + self
                .config
                .outputs
                .iter()
                .map(|o| o.splits_hz.len() * 2 + 12)
                .sum::<usize>();
        work.checked_add(80).is_some_and(|n| n <= MAX_OPERATIONS)
    }
    pub fn prepare_eq(&self, patch: EqPatch) -> Result<Box<PreparedEq>, &'static str> {
        if self.fault {
            return Err("fault latched");
        }
        if self.live_eq.is_some() {
            return Err("EQ retirement occupied");
        }
        if !self.live_eq_eligible() {
            return Err("live EQ resource budget");
        }
        patch.validate(self.config.sample_rate, self.inputs.len())?;
        let next = self
            .eq_generation
            .checked_add(1)
            .ok_or("EQ generation exhausted")?;
        let banks = [
            patch.inputs[0].bank(self.config.sample_rate)?,
            patch.inputs[1].bank(self.config.sample_rate)?,
        ];
        let changed = patch
            .inputs
            .each_ref()
            .map(|e| !e.matches(&self.config.inputs[e.input_index]));
        let noop = patch
            .inputs
            .iter()
            .all(|e| e.matches(&self.config.inputs[e.input_index]));
        Ok(Box::new(PreparedEq {
            previous: [
                EqSettings::from_input(
                    patch.inputs[0].input_index,
                    &self.config.inputs[patch.inputs[0].input_index],
                ),
                EqSettings::from_input(
                    patch.inputs[1].input_index,
                    &self.config.inputs[patch.inputs[1].input_index],
                ),
            ],
            graph_generation: 0,
            epoch: 0,
            frame: 0,
            patch,
            banks,
            instance: self.instance,
            base_generation: self.eq_generation,
            generation: next,
            elapsed: 0,
            duration: (self.config.sample_rate as u64).div_ceil(200).max(2),
            finished: noop,
            noop,
            changed,
        }))
    }
    /// On failure ownership stays with caller. No allocation or destruction.
    pub fn apply_eq(&mut self, prepared: &mut Option<Box<PreparedEq>>) -> Result<(), &'static str> {
        if self.fault {
            return Err("fault latched");
        }
        if self.live_eq.is_some() {
            return Err("EQ retirement occupied");
        }
        let p = prepared.as_ref().ok_or("missing prepared EQ")?;
        if p.instance != self.instance || p.base_generation != self.eq_generation {
            return Err("stale EQ owner/generation");
        }
        for e in &p.patch.inputs {
            e.install(&mut self.config.inputs[e.input_index]);
        }
        self.eq_generation = p.generation;
        self.live_eq = prepared.take();
        Ok(())
    }
    /// Move reserved storage to the controller for offRT destruction.
    pub fn retire_eq(&mut self) -> Option<Box<PreparedEq>> {
        if self
            .live_eq
            .as_ref()
            .is_some_and(|p| p.finished || self.fault)
        {
            let p = self.live_eq.take().unwrap();
            if self.fault && !p.finished {
                self.eq_aborted = Some(p.previous.clone());
            }
            Some(p)
        } else {
            None
        }
    }
    pub fn eq_progress(&self) -> EqProgress {
        EqProgress {
            generation: self.eq_generation,
            remaining: self
                .live_eq
                .as_ref()
                .filter(|p| !p.finished)
                .map_or(0, |p| p.duration - p.elapsed),
            retirement_occupied: self.live_eq.is_some(),
            eligible: self.live_eq_eligible(),
        }
    }
    pub fn quiesced(&self) -> bool {
        self.muted && self.ramp == 0.
    }
    pub fn mute(&mut self) {
        self.muted = true;
    }
    pub fn rearm(&mut self) -> Result<(), &'static str> {
        if self.fault || !self.quiesced() {
            return Err("fault or not quiesced");
        }
        self.muted = false;
        Ok(())
    }
    pub fn process(&mut self, input: &[f64], output: &mut [f64]) -> Result<(), &'static str> {
        let ni = self.inputs.len();
        let no = self.outputs.len();
        if input.is_empty()
            || !input.len().is_multiple_of(ni)
            || input.len() / ni > self.config.max_block
            || output.len() != input.len() / ni * no
        {
            return Err("buffer dimensions");
        }
        if self.fault {
            output.fill(0.);
            return Err("fault latched");
        }
        for (src, dst) in input.chunks_exact(ni).zip(output.chunks_exact_mut(no)) {
            let weight = self
                .live_eq
                .as_ref()
                .filter(|p| !p.finished && !p.noop)
                .map(|p| p.elapsed as f64 / (p.duration - 1) as f64);
            for (index, (state, &x)) in self.inputs.iter_mut().zip(src).enumerate() {
                let mut y = x * state.gain;
                for eq in &mut state.eq {
                    y = eq.tick(y);
                }
                if let Some(w) = weight {
                    let p = self.live_eq.as_mut().unwrap();
                    if let Some(slot) = p.patch.inputs.iter().position(|e| e.input_index == index)
                        && p.changed[slot]
                    {
                        let mut target = x * state.gain;
                        for eq in &mut p.banks[slot] {
                            target = eq.tick(target);
                        }
                        y = y * (1. - w) + target * w;
                    }
                }
                if state.compressor.enabled {
                    let c = state.compressor;
                    let desired = compression_db(
                        20. * y.abs().max(1e-30).log10(),
                        c.threshold_db,
                        c.ratio,
                        c.knee_db,
                    );
                    let coefficient = if desired < state.reduction {
                        state.attack
                    } else {
                        state.release
                    };
                    state.reduction = desired + (state.reduction - desired) * coefficient;
                    y *= 10_f64.powf((state.reduction + c.makeup_db) / 20.);
                }
                if !y.is_finite() || y.abs() > 1e12 {
                    self.fault = true;
                }
                self.values[index] = state.delay.tick(y);
            }
            if weight.is_some() {
                let p = self.live_eq.as_mut().unwrap();
                p.elapsed += 1;
                if p.elapsed == p.duration {
                    for slot in 0..2 {
                        if p.changed[slot] {
                            std::mem::swap(
                                &mut self.inputs[p.patch.inputs[slot].input_index].eq,
                                &mut p.banks[slot],
                            );
                        }
                    }
                    p.finished = true;
                }
            }
            for (index, node) in self.config.nodes.iter().enumerate() {
                let mut value = 0.;
                for route in &node.routes {
                    value += self.values[match route.source {
                        Source::Input(i) => i,
                        Source::Node(i) => ni + i,
                    }] * route.weight;
                }
                self.values[ni + index] = value;
            }
            let target = if self.muted { 0. } else { 1. };
            self.ramp += (target - self.ramp).clamp(
                -1. / (0.005 * self.config.sample_rate as f64),
                1. / (0.005 * self.config.sample_rate as f64),
            );
            for (index, state) in self.outputs.iter_mut().enumerate() {
                let c = &self.config.outputs[index];
                let mut y = c.source.map_or(0., |s| {
                    self.values[match s {
                        Source::Input(i) => i,
                        Source::Node(i) => ni + i,
                    }]
                });
                for filter in &mut state.filters {
                    y = filter.tick(y);
                }
                y *= state.gain;
                for eq in &mut state.eq {
                    y = eq.tick(y);
                }
                if !y.is_finite() || y.abs() > 1e12 {
                    self.fault = true;
                }
                let required = if y.abs() > state.ceiling {
                    state.ceiling / y.abs()
                } else {
                    1.
                };
                state.limiter = required.min(1. - (1. - state.limiter) * state.release);
                y = state.delay.tick(y * state.limiter);
                y = y.clamp(-state.ceiling, state.ceiling) * self.ramp;
                dst[index] = if c.muted { 0. } else { y };
            }
            if src
                .iter()
                .chain(&self.values)
                .any(|x| !x.is_finite() || x.abs() > 1e12)
            {
                self.fault = true;
            }
        }
        if self.fault {
            self.muted = true;
            self.ramp = 0.;
            output.fill(0.);
            return Err("numeric fault");
        }
        Ok(())
    }
}

#[cfg(test)]
mod live_eq_safety_tests {
    use super::*;
    fn patch(g: &Graph) -> EqPatch {
        EqPatch {
            version: 1,
            inputs: [
                EqSettings::from_input(0, &g.config.inputs[0]),
                EqSettings::from_input(1, &g.config.inputs[1]),
            ],
        }
    }
    #[test]
    fn optional_budget_does_not_change_legacy_admission_and_counter_exhaustion_refuses() {
        let mut c = GraphConfig::stereo(vec![]);
        c.nodes.push(Node {
            routes: vec![
                Route {
                    source: Source::Input(0),
                    weight: 1.
                };
                MAX_OPERATIONS - 104
            ],
        });
        assert!(c.validate().is_ok());
        let g = Graph::prepare(c).unwrap();
        assert!(!g.live_eq_eligible());
        assert!(g.prepare_eq(patch(&g)).is_err());
        let mut g = Graph::prepare(GraphConfig::stereo(vec![])).unwrap();
        g.eq_generation = u64::MAX;
        assert!(g.prepare_eq(patch(&g)).is_err());
        assert_eq!(g.eq_generation, u64::MAX);
    }
}
