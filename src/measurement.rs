//! Bounded offline reference/microphone analysis. Never opens devices or applies settings.
use crate::graph::GraphConfig;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const MIN_SAMPLES: usize = 32768;
pub const MAX_SAMPLES: usize = 65536;
pub const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_POSITIONS: usize = 8;
const SEGMENT: usize = 4096;
const MAX_LAG: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureMeta {
    pub id: String,
    pub source_epoch: String,
    pub map_revision: String,
    pub clock_domain: String,
    pub reference_id: String,
    pub reference_tap: String,
    pub reference_offset_frames: String,
    pub first_frame: String,
    pub sample_rate: u32,
    pub output_index: usize,
    pub position_id: String,
    pub configuration_revision: String,
    pub dropped_frames: String,
    pub clipped_reference: bool,
    pub clipped_mic: bool,
    pub timing_verified: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub max_arrival_samples: usize,
    pub band_hz: [f64; 2],
}
impl Default for Options {
    fn default() -> Self {
        Self {
            max_arrival_samples: MAX_LAG,
            band_hz: [100., 10000.],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectralBin {
    pub frequency_hz: f64,
    pub magnitude: f64,
    pub phase_radians: f64,
    pub coherence: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub contract: String,
    pub version: u32,
    pub algorithm: String,
    pub capture: CaptureMeta,
    pub options: Options,
    pub samples: usize,
    pub arrival_samples: i32,
    pub signed_correlation: f64,
    pub competing_peak_ratio: f64,
    pub segments: usize,
    pub spectrum: Vec<SpectralBin>,
    pub proposal_eligible: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionPair {
    pub anchor: Measurement,
    pub target: Measurement,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputChange {
    pub output_index: usize,
    pub before_delay_samples: u32,
    pub after_delay_samples: u32,
    pub before_inverted: bool,
    pub after_inverted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub contract: String,
    pub version: u32,
    pub status: String,
    pub basis_configuration_revision: String,
    pub basis_capture: CaptureMeta,
    pub basis_configuration: GraphConfig,
    pub measurement_ids: Vec<String>,
    pub changes: Vec<OutputChange>,
    pub added_latency_samples: u32,
    pub verification_required: bool,
    pub reason: String,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Analyze {
        contract: String,
        version: u32,
        capture: Box<CaptureMeta>,
        options: Options,
        reference: Vec<f64>,
        mic: Vec<f64>,
    },
    Candidate {
        contract: String,
        version: u32,
        configuration_revision: String,
        configuration: GraphConfig,
        proposal: Box<Proposal>,
    },
    Propose {
        contract: String,
        version: u32,
        configuration_revision: String,
        configuration: GraphConfig,
        positions: Vec<PositionPair>,
    },
}
pub fn counter(s: &str) -> Result<u64, &'static str> {
    if s.is_empty()
        || s.len() > 20
        || (s.len() > 1 && s.starts_with('0'))
        || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("noncanonical_counter");
    }
    s.parse().map_err(|_| "counter_overflow")
}
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_graphic())
}
fn validate_meta(m: &CaptureMeta, samples: usize) -> Result<(), &'static str> {
    if ![
        &m.id,
        &m.clock_domain,
        &m.reference_id,
        &m.reference_tap,
        &m.position_id,
    ]
    .into_iter()
    .all(|s| identifier(s))
        || m.sample_rate != 48000
        || m.output_index >= crate::graph::MAX_PORTS
        || counter(&m.source_epoch)? == 0
        || counter(&m.map_revision)? == 0
        || counter(&m.configuration_revision)? == 0
    {
        return Err("capture_identity");
    }
    counter(&m.reference_offset_frames)?;
    counter(&m.first_frame)?
        .checked_add(samples as u64)
        .ok_or("frame_overflow")?;
    if counter(&m.dropped_frames)? != 0
        || m.clipped_reference
        || m.clipped_mic
        || !m.timing_verified
    {
        return Err("capture_quality_or_timing");
    }
    Ok(())
}
fn validate_options(o: &Options) -> Result<(), &'static str> {
    if !(1..=MAX_LAG).contains(&o.max_arrival_samples)
        || !o.band_hz.iter().all(|v| v.is_finite())
        || o.band_hz[0] < 20.
        || o.band_hz[1] > 20000.
        || o.band_hz[1] <= o.band_hz[0]
    {
        return Err("analysis_limits");
    }
    Ok(())
}
fn correlation(x: &[f64], y: &[f64], lag: i32, stride: usize) -> f64 {
    let (mut xy, mut xx, mut yy) = (0., 0., 0.);
    for i in (MAX_LAG..MAX_LAG + SEGMENT).step_by(stride) {
        let a = x[i];
        let b = y[(i as i32 + lag) as usize];
        xy += a * b;
        xx += a * a;
        yy += b * b;
    }
    if xx <= 1e-16 || yy <= 1e-16 {
        0.
    } else {
        xy / (xx * yy).sqrt()
    }
}
fn dft(samples: &[f64], bin: usize) -> (f64, f64) {
    let step = -2. * PI * bin as f64 / SEGMENT as f64;
    let (sin, cos) = step.sin_cos();
    let (mut cr, mut ci, mut re, mut im) = (1., 0., 0., 0.);
    for (i, &sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (2. * PI * i as f64 / (SEGMENT - 1) as f64).cos();
        re += sample * window * cr;
        im += sample * window * ci;
        (cr, ci) = (cr * cos - ci * sin, ci * cos + cr * sin);
    }
    (re, im)
}
pub fn analyze(
    meta: &CaptureMeta,
    reference: &[f64],
    mic: &[f64],
    options: &Options,
) -> Result<Measurement, &'static str> {
    let n = reference.len();
    if !(MIN_SAMPLES..=MAX_SAMPLES).contains(&n) || mic.len() != n {
        return Err("sample_count");
    }
    validate_meta(meta, n)?;
    validate_options(options)?;
    if reference
        .iter()
        .chain(mic)
        .any(|v| !v.is_finite() || v.abs() >= 1.)
    {
        return Err("nonfinite_or_clipped_samples");
    }
    let centered = |v: &[f64]| {
        let mean = v.iter().sum::<f64>() / n as f64;
        v.iter().map(|x| x - mean).collect::<Vec<_>>()
    };
    let x = centered(reference);
    let y = centered(mic);
    if [&x, &y]
        .iter()
        .any(|v| v.iter().map(|x| x * x).sum::<f64>() / (n as f64) < 1e-10)
    {
        return Err("insufficient_signal");
    }
    let max = options.max_arrival_samples as i32;
    let mut scores = Vec::with_capacity((max * 2 + 1) as usize);
    for lag in -max..=max {
        scores.push(correlation(&x, &y, lag, 4));
    }
    let peak = scores
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .unwrap()
        .0;
    let mut lag = peak as i32 - max;
    let mut signed = 0_f64;
    for candidate in (lag - 4).max(-max)..=(lag + 4).min(max) {
        let c = correlation(&x, &y, candidate, 1);
        if c.abs() > signed.abs() {
            signed = c;
            lag = candidate;
        }
    }
    let competitor = scores
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as i32 - max - lag).abs() > 4)
        .map(|(_, v)| v.abs())
        .fold(0_f64, f64::max)
        / signed.abs().max(1e-12);
    let xs = if lag < 0 { -lag as usize } else { 0 };
    let ys = if lag > 0 { lag as usize } else { 0 };
    let available = n - xs.max(ys);
    // 50% overlap gives >=14 windows even at the largest admitted arrival.
    let starts: Vec<_> = (0..=available - SEGMENT).step_by(SEGMENT / 2).collect();
    let lo = (options.band_hz[0] * SEGMENT as f64 / 48000.).ceil() as usize;
    let hi = (options.band_hz[1] * SEGMENT as f64 / 48000.).floor() as usize;
    if hi < lo || hi - lo + 1 < 8 {
        return Err("insufficient_bandwidth");
    }
    let count = (hi - lo + 1).min(64);
    let bins: Vec<_> = (0..count)
        .map(|i| lo + i * (hi - lo) / (count - 1))
        .collect();
    let mut raw = Vec::with_capacity(count);
    for bin in bins {
        let (mut xx, mut yy, mut re, mut im) = (0., 0., 0., 0.);
        for &start in &starts {
            let (xr, xi) = dft(&x[xs + start..xs + start + SEGMENT], bin);
            let (yr, yi) = dft(&y[ys + start..ys + start + SEGMENT], bin);
            xx += xr * xr + xi * xi;
            yy += yr * yr + yi * yi;
            re += yr * xr + yi * xi;
            im += yi * xr - yr * xi;
        }
        raw.push((bin, xx, yy, re, im));
    }
    let peak_power = raw.iter().map(|v| v.1).fold(0_f64, f64::max);
    let spectrum: Vec<_> = raw
        .into_iter()
        .filter(|v| v.1 > peak_power * 1e-3 && v.2 > 1e-14)
        .map(|(bin, xx, yy, re, im)| {
            let hz = bin as f64 * 48000. / SEGMENT as f64;
            let phase = im.atan2(re) - 2. * PI * hz * lag as f64 / 48000.;
            SpectralBin {
                frequency_hz: hz,
                magnitude: re.hypot(im) / xx,
                phase_radians: phase.sin().atan2(phase.cos()),
                coherence: ((re * re + im * im) / (xx * yy)).clamp(0., 1.),
            }
        })
        .collect();
    let mut reasons = Vec::new();
    if lag.abs() == max {
        reasons.push("arrival_at_search_boundary".into());
    }
    if signed.abs() < 0.9 {
        reasons.push("weak_direct_correlation".into());
    }
    if competitor > 0.5 {
        reasons.push("ambiguous_or_reflected_arrival".into());
    }
    if spectrum.len() < 8 || spectrum.len() < count / 2 {
        reasons.push("insufficient_broadband_excitation".into());
    }
    if spectrum.iter().any(|b| b.coherence < 0.95) {
        reasons.push("low_coherence".into());
    }
    Ok(Measurement {
        contract: "C-PA-MEASUREMENT-RESULT".into(),
        version: 1,
        algorithm: "offline-h1-v1".into(),
        capture: meta.clone(),
        options: options.clone(),
        samples: n,
        arrival_samples: lag,
        signed_correlation: signed,
        competing_peak_ratio: competitor,
        segments: starts.len(),
        spectrum,
        proposal_eligible: reasons.is_empty(),
        reasons,
    })
}
fn validate_measurement(m: &Measurement) -> Result<(), &'static str> {
    validate_meta(&m.capture, m.samples)?;
    validate_options(&m.options)?;
    if m.contract != "C-PA-MEASUREMENT-RESULT"
        || m.version != 1
        || m.algorithm != "offline-h1-v1"
        || !(MIN_SAMPLES..=MAX_SAMPLES).contains(&m.samples)
        || m.segments < 8
        || m.segments > 32
        || m.arrival_samples.unsigned_abs() as usize >= m.options.max_arrival_samples
        || !m.signed_correlation.is_finite()
        || !(0.9..=1.000001).contains(&m.signed_correlation.abs())
        || !m.competing_peak_ratio.is_finite()
        || !(0.0..=0.5).contains(&m.competing_peak_ratio)
        || !(8..=64).contains(&m.spectrum.len())
        || !m.proposal_eligible
        || !m.reasons.is_empty()
    {
        return Err("measurement_not_qualified");
    }
    let lo = (m.options.band_hz[0] * SEGMENT as f64 / 48000.).ceil() as usize;
    let hi = (m.options.band_hz[1] * SEGMENT as f64 / 48000.).floor() as usize;
    if hi < lo || hi - lo + 1 < 8 {
        return Err("insufficient_bandwidth");
    }
    let count = (hi - lo + 1).min(64);
    let expected: Vec<_> = (0..count)
        .map(|i| (lo + i * (hi - lo) / (count - 1)) as f64 * 48000. / SEGMENT as f64)
        .collect();
    let expected_segments =
        (m.samples - m.arrival_samples.unsigned_abs() as usize - SEGMENT) / (SEGMENT / 2) + 1;
    if m.spectrum.len() < count / 2 || m.segments != expected_segments {
        return Err("incomplete_spectral_evidence");
    }
    let mut previous = 0.;
    for b in &m.spectrum {
        if ![b.frequency_hz, b.magnitude, b.phase_radians, b.coherence]
            .iter()
            .all(|v| v.is_finite())
            || !expected.contains(&b.frequency_hz)
            || b.frequency_hz <= previous
            || b.frequency_hz < m.options.band_hz[0]
            || b.frequency_hz > m.options.band_hz[1]
            || b.magnitude <= 0.
            || !(-PI..=PI).contains(&b.phase_radians)
            || !(0.95..=1.).contains(&b.coherence)
        {
            return Err("invalid_spectral_evidence");
        }
        previous = b.frequency_hz;
    }
    Ok(())
}
fn same_basis(a: &CaptureMeta, b: &CaptureMeta) -> bool {
    a.source_epoch == b.source_epoch
        && a.map_revision == b.map_revision
        && a.clock_domain == b.clock_domain
        && a.reference_id == b.reference_id
        && a.reference_tap == b.reference_tap
        && a.reference_offset_frames == b.reference_offset_frames
        && a.sample_rate == b.sample_rate
        && a.configuration_revision == b.configuration_revision
}
/// Produces a review object only. No graph instance or mute/rearm state is touched.
pub fn propose(
    base: &GraphConfig,
    revision: &str,
    positions: &[PositionPair],
) -> Result<Proposal, &'static str> {
    base.validate()?;
    if base.sample_rate != 48000
        || counter(revision)? == 0
        || positions.is_empty()
        || positions.len() > MAX_POSITIONS
    {
        return Err("proposal_limits");
    }
    let mut proposal = Proposal {
        contract: "C-PA-ALIGNMENT-PROPOSAL".into(),
        version: 1,
        status: "refused".into(),
        basis_configuration_revision: revision.into(),
        basis_capture: positions[0].anchor.capture.clone(),
        basis_configuration: base.clone(),
        measurement_ids: Vec::new(),
        changes: Vec::new(),
        added_latency_samples: 0,
        verification_required: true,
        reason: String::new(),
    };
    let decision = (|| -> Result<(i32, bool), &'static str> {
        let origin = &positions[0].anchor.capture;
        let anchor_index = origin.output_index;
        let target_index = positions[0].target.capture.output_index;
        if anchor_index == target_index
            || anchor_index >= base.outputs.len()
            || target_index >= base.outputs.len()
            || base.outputs[anchor_index].source.is_none()
            || base.outputs[target_index].source.is_none()
        {
            return Err("output_identity");
        }
        let mut seen_positions = std::collections::BTreeSet::new();
        let mut seen_ids = std::collections::BTreeSet::new();
        let mut consensus = None;
        for p in positions {
            validate_measurement(&p.anchor)?;
            validate_measurement(&p.target)?;
            let (a, b) = (&p.anchor, &p.target);
            if !same_basis(&a.capture, &b.capture)
                || !same_basis(origin, &a.capture)
                || a.capture.configuration_revision != revision
                || a.capture.position_id != b.capture.position_id
                || !seen_positions.insert(&a.capture.position_id)
                || a.capture.output_index != anchor_index
                || b.capture.output_index != target_index
                || a.options != b.options
                || !seen_ids.insert(&a.capture.id)
                || !seen_ids.insert(&b.capture.id)
            {
                return Err("incompatible_measurement_basis");
            }
            proposal
                .measurement_ids
                .extend([a.capture.id.clone(), b.capture.id.clone()]);
            let delta = b.arrival_samples - a.arrival_samples;
            let mut errors = [0., 0.];
            let mut used = 0;
            for x in &a.spectrum {
                let Some(y) = b.spectrum.iter().find(|y| y.frequency_hz == x.frequency_hz) else {
                    continue;
                };
                let residual = y.phase_radians - x.phase_radians
                    + 2. * PI * x.frequency_hz * delta as f64 / 48000.;
                for (invert, sum) in errors.iter_mut().enumerate() {
                    let phase = residual + invert as f64 * PI;
                    *sum += phase.sin().atan2(phase.cos()).powi(2);
                }
                used += 1;
            }
            if used < 8 {
                return Err("insufficient_common_band");
            }
            let invert = errors[1] < errors[0];
            if (errors[usize::from(invert)] / used as f64).sqrt() > PI / 12. {
                return Err("nonconstant_relative_phase");
            }
            let candidate = (delta, invert);
            if consensus.is_some_and(|v| v != candidate) {
                return Err("conflicting_positions");
            }
            consensus = Some(candidate);
        }
        Ok(consensus.unwrap())
    })();
    let (delta, invert) = match decision {
        Ok(v) => v,
        Err(reason) => {
            proposal.reason = reason.into();
            return Ok(proposal);
        }
    };
    let anchor = positions[0].anchor.capture.output_index;
    let target = positions[0].target.capture.output_index;
    let delayed = if delta > 0 { anchor } else { target };
    for index in [anchor, target] {
        let before = &base.outputs[index].processing;
        let before_samples = (before.delay_ms * 48.).round() as u32;
        let added = if index == delayed {
            delta.unsigned_abs()
        } else {
            0
        };
        let after = before_samples + added;
        if after > 480 {
            proposal.changes.clear();
            proposal.reason = "output_delay_range".into();
            return Ok(proposal);
        }
        let after_inverted = before.inverted ^ (index == target && invert);
        if added != 0 || after_inverted != before.inverted {
            proposal.changes.push(OutputChange {
                output_index: index,
                before_delay_samples: before_samples,
                after_delay_samples: after,
                before_inverted: before.inverted,
                after_inverted,
            });
        }
    }
    proposal.added_latency_samples = delta.unsigned_abs();
    proposal.status = if proposal.changes.is_empty() {
        "no_change"
    } else {
        "proposed"
    }
    .into();
    proposal.reason = if proposal.changes.is_empty() {
        "already_aligned"
    } else {
        "offline_candidate_requires_combined_and_acoustic_verification"
    }
    .into();
    Ok(proposal)
}
impl Proposal {
    /// Makes a candidate config for the existing muted review path, never applies it.
    pub fn candidate_configuration(
        &self,
        current: &GraphConfig,
        revision: &str,
    ) -> Result<GraphConfig, &'static str> {
        if self.contract != "C-PA-ALIGNMENT-PROPOSAL"
            || self.version != 1
            || !matches!(self.status.as_str(), "proposed" | "no_change")
            || !self.verification_required
            || self.basis_configuration_revision != revision
            || &self.basis_configuration != current
            || self.changes.len() > 2
            || current.sample_rate != 48000
            || counter(revision)? == 0
            || self.reason.len() > 256
            || self.measurement_ids.is_empty()
            || self.measurement_ids.len() > MAX_POSITIONS * 2
            || self.measurement_ids.iter().any(|s| !identifier(s))
            || (self.status == "no_change"
                && (!self.changes.is_empty() || self.added_latency_samples != 0))
            || (self.status == "proposed" && self.changes.is_empty())
        {
            return Err("stale_or_invalid_proposal");
        }
        validate_meta(&self.basis_capture, 0)?;
        if self.basis_capture.configuration_revision != revision {
            return Err("proposal_basis_revision");
        }
        current.validate()?;
        let mut next = current.clone();
        let mut seen = std::collections::BTreeSet::new();
        let mut added = 0_u32;
        for c in &self.changes {
            let Some(output) = next.outputs.get_mut(c.output_index) else {
                return Err("output_identity");
            };
            if !seen.insert(c.output_index)
                || output.source.is_none()
                || (output.processing.delay_ms * 48.).round() as u32 != c.before_delay_samples
                || output.processing.inverted != c.before_inverted
                || c.after_delay_samples > 480
                || c.after_delay_samples < c.before_delay_samples
                || (c.after_delay_samples == c.before_delay_samples
                    && c.after_inverted == c.before_inverted)
            {
                return Err("invalid_proposal_change");
            }
            added = added.max(c.after_delay_samples - c.before_delay_samples);
            // Preserve exact existing nonintegral delay value for polarity-only edits.
            if c.after_delay_samples != c.before_delay_samples {
                output.processing.delay_ms = c.after_delay_samples as f64 / 48.;
            }
            output.processing.inverted = c.after_inverted;
        }
        if added != self.added_latency_samples {
            return Err("proposal_latency_mismatch");
        }
        next.validate()?;
        Ok(next)
    }
}
/// Strict recursive JSON decoding rejects duplicate keys before typed admission.
/// The byte bound caps total parse memory; each array and object also has a bound.
pub fn decode_request(bytes: &[u8]) -> Result<Request, String> {
    if bytes.len() > MAX_JSON_BYTES {
        return Err("request_byte_capacity".into());
    }
    struct Strict(serde_json::Value);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Strict;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("bounded JSON without duplicate keys")
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Strict, E> {
                    serde_json::Number::from_f64(v)
                        .map(|n| Strict(n.into()))
                        .ok_or_else(|| E::custom("nonfinite"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                    self.visit_string(v.into())
                }
                fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Strict, E> {
                    if v.len() > 256 {
                        return Err(E::custom("string_capacity"));
                    }
                    Ok(Strict(v.into()))
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                    Ok(Strict(serde_json::Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Strict, A::Error> {
                    let mut out = Vec::new();
                    while let Some(Strict(v)) = a.next_element()? {
                        if out.len() == MAX_SAMPLES {
                            return Err(serde::de::Error::custom("array_capacity"));
                        }
                        out.push(v);
                    }
                    Ok(Strict(out.into()))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Strict, A::Error> {
                    let mut out = serde_json::Map::new();
                    while let Some(key) = a.next_key::<String>()? {
                        if key.len() > 128 || out.len() == 64 || out.contains_key(&key) {
                            return Err(serde::de::Error::custom(
                                "object_capacity_or_duplicate_key",
                            ));
                        }
                        let Strict(v) = a.next_value()?;
                        out.insert(key, v);
                    }
                    Ok(Strict(out.into()))
                }
            }
            d.deserialize_any(Visitor)
        }
    }
    let Strict(value) = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}
pub fn execute(request: Request) -> Result<serde_json::Value, String> {
    match request {
        Request::Analyze {
            contract,
            version,
            capture,
            options,
            reference,
            mic,
        } => {
            if contract != "C-PA-MEASUREMENT" || version != 1 {
                return Err("request_identity".into());
            }
            let result = analyze(&capture, &reference, &mic, &options)?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        Request::Candidate {
            contract,
            version,
            configuration_revision,
            configuration,
            proposal,
        } => {
            if contract != "C-PA-MEASUREMENT" || version != 1 {
                return Err("request_identity".into());
            }
            let candidate =
                proposal.candidate_configuration(&configuration, &configuration_revision)?;
            Ok(
                serde_json::json!({"contract":"C-PA-ALIGNMENT-CANDIDATE", "version":1,
                "basis_capture":proposal.basis_capture,
                "basis_configuration_revision":configuration_revision,
                "configuration_json":serde_json::to_string(&candidate).map_err(|e|e.to_string())?}),
            )
        }
        Request::Propose {
            contract,
            version,
            configuration_revision,
            configuration,
            positions,
        } => {
            if contract != "C-PA-MEASUREMENT" || version != 1 {
                return Err("request_identity".into());
            }
            let result = propose(&configuration, &configuration_revision, &positions)?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
    }
}
