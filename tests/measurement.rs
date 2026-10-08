use shr_pa::{
    graph::{Graph, GraphConfig},
    measurement::*,
};
use std::f64::consts::PI;
fn meta(id: &str, output: usize) -> CaptureMeta {
    CaptureMeta {
        id: id.into(),
        source_epoch: "1".into(),
        map_revision: "1".into(),
        clock_domain: "synthetic-common-clock".into(),
        reference_id: "actual-input".into(),
        reference_tap: "program-pre-pa".into(),
        reference_offset_frames: "0".into(),
        first_frame: "10000".into(),
        sample_rate: 48000,
        output_index: output,
        position_id: "position-1".into(),
        configuration_revision: "1".into(),
        dropped_frames: "0".into(),
        clipped_reference: false,
        clipped_mic: false,
        timing_verified: true,
    }
}
fn noise() -> Vec<f64> {
    let mut state = 17_u64;
    (0..MIN_SAMPLES)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as i64 as f64 / i64::MAX as f64) * 0.03
        })
        .collect()
}
fn shifted(x: &[f64], delay: usize, sign: f64) -> Vec<f64> {
    (0..x.len())
        .map(|i| if i >= delay { x[i - delay] * sign } else { 0. })
        .collect()
}
fn measure(id: &str, index: usize, x: &[f64], y: &[f64]) -> Measurement {
    analyze(&meta(id, index), x, y, &Options::default()).unwrap()
}
fn config() -> GraphConfig {
    let mut c = GraphConfig::stereo(vec![]);
    // Both independently protected outputs receive the same actual excitation.
    c.outputs[1].source = c.outputs[0].source;
    c
}
fn render(config: GraphConfig, x: &[f64]) -> Vec<Vec<f64>> {
    let mut graph = Graph::prepare(config.clone()).unwrap();
    graph.rearm().unwrap();
    graph
        .process(
            &vec![0.; config.inputs.len() * 256],
            &mut vec![0.; config.outputs.len() * 256],
        )
        .unwrap();
    let mut out = vec![Vec::new(); config.outputs.len()];
    for block in x.chunks(256) {
        let input: Vec<_> = block
            .iter()
            .flat_map(|&v| vec![v; config.inputs.len()])
            .collect();
        let mut output = vec![0.; block.len() * config.outputs.len()];
        graph.process(&input, &mut output).unwrap();
        for frame in output.chunks(config.outputs.len()) {
            for (c, &v) in frame.iter().enumerate() {
                out[c].push(v);
            }
        }
    }
    out
}
#[test]
fn production_graph_arrival_polarity_proposal_and_independent_sum() {
    let x = noise();
    let mut c = config();
    c.outputs[1].processing.delay_ms = 0.5;
    c.outputs[1].processing.inverted = true;
    let before = render(c.clone(), &x);
    let a = measure("a", 0, &x, &before[0]);
    let b = measure("b", 1, &x, &before[1]);
    assert_eq!(a.arrival_samples, 0);
    assert_eq!(b.arrival_samples, 24);
    for bin in &b.spectrum {
        let expected = PI - 2. * PI * bin.frequency_hz * 24. / 48000.;
        assert!((bin.magnitude - 1.).abs() < 1e-6);
        let error = bin.phase_radians - expected;
        assert!(error.sin().atan2(error.cos()).abs() < 1e-6);
        assert!(bin.coherence > 0.999999);
    }
    assert!(a.proposal_eligible, "{:?}", a.reasons);
    assert!(b.proposal_eligible, "{:?}", b.reasons);
    let proposal = propose(
        &c,
        "1",
        &[PositionPair {
            anchor: a,
            target: b,
        }],
    )
    .unwrap();
    assert_eq!(proposal.status, "proposed", "{}", proposal.reason);
    let next = proposal.candidate_configuration(&c, "1").unwrap();
    assert_eq!(next.outputs[0].processing.delay_ms, 0.5);
    assert!(!next.outputs[1].processing.inverted);
    let mut expected = c.clone();
    expected.outputs[0].processing.delay_ms = 0.5;
    expected.outputs[1].processing.inverted = false;
    assert_eq!(next, expected);
    let after = render(next, &x);
    let ideal = shifted(&x, 24, 2.);
    let mse = after[0]
        .iter()
        .zip(&after[1])
        .zip(&ideal)
        .map(|((&a, &b), &v)| (a + b - v).powi(2))
        .sum::<f64>()
        / x.len() as f64;
    assert!(mse < 1e-20, "{mse}");
    let before_energy = before[0]
        .iter()
        .zip(&before[1])
        .map(|(&a, &b)| (a + b).powi(2))
        .sum::<f64>();
    let after_energy = after[0]
        .iter()
        .zip(&after[1])
        .map(|(&a, &b)| (a + b).powi(2))
        .sum::<f64>();
    assert!(after_energy > before_energy * 1.8);
    assert!(proposal.candidate_configuration(&c, "2").is_err());
}
#[test]
fn zero_one_maximum_and_out_of_range_proposals() {
    let x = noise();
    let a = measure("a", 0, &x, &x);
    for delay in [0, 1, 480, 481] {
        let b = measure("b", 1, &x, &shifted(&x, delay, 1.));
        assert_eq!(b.arrival_samples, delay as i32);
        let p = propose(
            &config(),
            "1",
            &[PositionPair {
                anchor: a.clone(),
                target: b,
            }],
        )
        .unwrap();
        assert_eq!(
            p.status,
            if delay == 0 {
                "no_change"
            } else if delay <= 480 {
                "proposed"
            } else {
                "refused"
            },
            "{}",
            p.reason
        );
        if delay == 0 {
            assert_eq!(p.candidate_configuration(&config(), "1").unwrap(), config());
        }
    }
}
#[test]
fn timing_clipping_missing_and_schema_refusals() {
    let x = noise();
    for field in 0..4 {
        let mut m = meta("x", 0);
        match field {
            0 => m.dropped_frames = "1".into(),
            1 => m.clipped_mic = true,
            2 => m.timing_verified = false,
            _ => m.source_epoch = "01".into(),
        };
        assert!(analyze(&m, &x, &x, &Options::default()).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, 1., -1.] {
        let mut y = x.clone();
        y[100] = bad;
        assert!(analyze(&meta("x", 0), &x, &y, &Options::default()).is_err());
    }
    assert!(analyze(&meta("x", 0), &x[..100], &x[..100], &Options::default()).is_err());
    assert!(
        analyze(
            &meta("x", 0),
            &vec![0.; MIN_SAMPLES],
            &x,
            &Options::default()
        )
        .is_err()
    );
    let mut overflow = meta("overflow", 0);
    overflow.first_frame = u64::MAX.to_string();
    assert!(analyze(&overflow, &x, &x, &Options::default()).is_err());
    assert!(decode_request(br#"{"operation":"analyze","operation":"propose"}"#).is_err());
    assert!(decode_request(&vec![b' '; MAX_JSON_BYTES + 1]).is_err());
}
#[test]
fn noise_periodicity_and_reflections_withhold_proposals() {
    let x = noise();
    let periodic: Vec<_> = (0..MIN_SAMPLES)
        .map(|i| (2. * PI * i as f64 / 48.).sin() * 0.03)
        .collect();
    let p = measure("periodic", 0, &periodic, &periodic);
    assert!(!p.proposal_eligible, "{:?}", p.reasons);
    let reflected: Vec<_> = x
        .iter()
        .zip(shifted(&x, 100, 0.8))
        .map(|(&a, b)| a + b)
        .collect();
    let r = measure("reflection", 0, &x, &reflected);
    assert!(!r.proposal_eligible);
    let unrelated: Vec<_> = x.iter().rev().copied().collect();
    let n = measure("noise", 0, &x, &unrelated);
    assert!(!n.proposal_eligible);
}
#[test]
fn multiple_positions_require_exact_consensus_and_basis() {
    let x = noise();
    let a = measure("a", 0, &x, &x);
    let b = measure("b", 1, &x, &shifted(&x, 24, 1.));
    let first = PositionPair {
        anchor: a.clone(),
        target: b.clone(),
    };
    let mut second = first.clone();
    second.anchor.capture.id = "c".into();
    second.target.capture.id = "d".into();
    second.anchor.capture.position_id = "position-2".into();
    second.target.capture.position_id = "position-2".into();
    assert_eq!(
        propose(&config(), "1", &[first.clone(), second.clone()])
            .unwrap()
            .status,
        "proposed"
    );
    second.target = measure("d", 1, &x, &shifted(&x, 25, 1.));
    second.target.capture.position_id = "position-2".into();
    assert_eq!(
        propose(&config(), "1", &[first.clone(), second])
            .unwrap()
            .reason,
        "conflicting_positions"
    );
    let mut mismatch = first;
    mismatch.target.capture.source_epoch = "2".into();
    assert_eq!(
        propose(&config(), "1", &[mismatch]).unwrap().status,
        "refused"
    );
}
#[test]
fn production_crossover_transfer_is_complex_and_not_forced_into_delay() {
    let x = noise();
    let c = GraphConfig::stereo(vec![1000.]);
    let output = render(c, &x);
    let m = measure("high", 0, &x, &output[0]);
    assert!(m.spectrum.len() >= 8);
    assert!(m.spectrum.iter().any(|b| b.phase_radians.abs() > 0.1));
    // Independent bilinear analog LR24 high-pass reference, no owner coefficient reuse.
    for b in m.spectrum.iter().filter(|b| b.frequency_hz > 400.) {
        let k = (PI * b.frequency_hz / 48000.).tan() / (PI * 1000. / 48000.).tan();
        let expected_magnitude = k.powi(4) / ((1. - k * k).powi(2) + 2. * k * k);
        let expected_phase = 2. * (PI - (2_f64.sqrt() * k).atan2(1. - k * k));
        assert!(
            (b.magnitude - expected_magnitude).abs() < 0.015,
            "{}",
            b.frequency_hz
        );
        let error = b.phase_radians - expected_phase;
        assert!(
            error.sin().atan2(error.cos()).abs() < 0.03,
            "{}",
            b.frequency_hz
        );
    }
    // Independently measured production low/high responses have designed phase;
    // a dominant correlation peak is not permission to erase crossover behavior.
    let low = measure("low", 2, &x, &output[2]);
    let p = propose(
        &GraphConfig::stereo(vec![1000.]),
        "1",
        &[PositionPair {
            anchor: m,
            target: low,
        }],
    )
    .unwrap();
    assert_eq!(p.status, "refused");
}
#[test]
fn standalone_json_refusal_never_applies_or_opens_devices() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_shr-pa-measure"))
        .args(["unexpected", "extra"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "refused");
}
#[test]
fn clipping_range_rounding_and_proposal_tampering_preserve_base() {
    let x = noise();
    let a = measure("a", 0, &x, &x);
    let b = measure("b", 1, &x, &shifted(&x, 1, 1.));
    let mut c = config();
    c.outputs[0].processing.delay_ms = 9.999;
    let p = propose(
        &c,
        "1",
        &[PositionPair {
            anchor: a.clone(),
            target: b.clone(),
        }],
    )
    .unwrap();
    assert_eq!(p.reason, "output_delay_range");
    assert!(p.changes.is_empty());
    let mut p = propose(
        &config(),
        "1",
        &[PositionPair {
            anchor: a,
            target: b,
        }],
    )
    .unwrap();
    p.status = "no_change".into();
    assert!(p.candidate_configuration(&config(), "1").is_err());
    p.status = "proposed".into();
    p.added_latency_samples = 123;
    assert!(p.candidate_configuration(&config(), "1").is_err());
}
#[test]
fn negative_arrival_and_boundary_are_explicit() {
    let x = noise();
    let reference = shifted(&x, 24, 1.);
    let m = measure("early", 0, &reference, &x);
    assert_eq!(m.arrival_samples, -24);
    assert!(m.proposal_eligible);
    let options = Options {
        max_arrival_samples: 24,
        ..Options::default()
    };
    let boundary = analyze(&meta("edge", 0), &x, &shifted(&x, 24, 1.), &options).unwrap();
    assert!(!boundary.proposal_eligible);
    assert!(
        boundary
            .reasons
            .iter()
            .any(|r| r == "arrival_at_search_boundary")
    );
}
#[test]
fn bounded_json_accepts_real_request_and_rejects_unknown_and_nested_duplicates() {
    let request = serde_json::json!({"contract":"C-PA-MEASUREMENT","version":1,"operation":"analyze","capture":meta("x",0),"options":Options::default(),"reference":[],"mic":[]});
    let encoded = serde_json::to_vec(&request).unwrap();
    assert!(decode_request(&encoded).is_ok());
    let mut extra = request.clone();
    extra["surprise"] = true.into();
    assert!(decode_request(&serde_json::to_vec(&extra).unwrap()).is_err());
    let text = String::from_utf8(encoded).unwrap().replace(
        "\"sample_rate\":48000",
        "\"sample_rate\":48000,\"sample_rate\":48000",
    );
    assert!(decode_request(text.as_bytes()).is_err());
    let mut larger = request;
    larger["reference"] = serde_json::json!(vec![0; MAX_SAMPLES + 1]);
    assert!(decode_request(&serde_json::to_vec(&larger).unwrap()).is_err());
}
#[test]
fn target_earlier_inverted_and_output_map_drift() {
    let x = noise();
    let mut c = config();
    c.outputs[0].processing.delay_ms = 0.5;
    c.outputs[1].processing.inverted = true;
    let output = render(c.clone(), &x);
    let a = measure("a", 0, &x, &output[0]);
    let b = measure("b", 1, &x, &output[1]);
    let pair = PositionPair {
        anchor: a,
        target: b,
    };
    let p = propose(&c, "1", std::slice::from_ref(&pair)).unwrap();
    let next = p.candidate_configuration(&c, "1").unwrap();
    assert_eq!(next.outputs[1].processing.delay_ms, 0.5);
    assert!(!next.outputs[1].processing.inverted);
    let mut drift = c.clone();
    drift.outputs[1].source = Some(shr_pa::graph::Source::Input(1));
    assert!(p.candidate_configuration(&drift, "1").is_err());
    let mut mismatch = pair.clone();
    mismatch.target.capture.map_revision = "2".into();
    assert_eq!(
        propose(&c, "1", &[mismatch]).unwrap().reason,
        "incompatible_measurement_basis"
    );
    let mut offset_mismatch = pair.clone();
    offset_mismatch.target.capture.reference_offset_frames = "1".into();
    assert_eq!(
        propose(&c, "1", &[offset_mismatch]).unwrap().reason,
        "incompatible_measurement_basis"
    );
    let mut mismatch = pair;
    mismatch.target.capture.configuration_revision = "2".into();
    assert_eq!(
        propose(&c, "1", &[mismatch]).unwrap().reason,
        "incompatible_measurement_basis"
    );
}
#[test]
fn golden_owner_results_and_proposal_are_strictly_interoperable() {
    let a: Measurement =
        serde_json::from_str(include_str!("fixtures/measurement/v1/anchor-result.json")).unwrap();
    let b: Measurement =
        serde_json::from_str(include_str!("fixtures/measurement/v1/target-result.json")).unwrap();
    let golden: Proposal =
        serde_json::from_str(include_str!("fixtures/measurement/v1/proposal.json")).unwrap();
    let proposal = propose(
        &golden.basis_configuration,
        "1",
        &[PositionPair {
            anchor: a,
            target: b,
        }],
    )
    .unwrap();
    assert_eq!(proposal.changes, golden.changes);
    assert_eq!(proposal.status, "proposed");
    assert_eq!(proposal.basis_capture, golden.basis_capture);
    assert_eq!(
        proposal
            .candidate_configuration(&golden.basis_configuration, "1")
            .unwrap(),
        golden
            .candidate_configuration(&golden.basis_configuration, "1")
            .unwrap()
    );
    let refusal: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/measurement/v1/refusal.json")).unwrap();
    assert_eq!(refusal["contract"], "C-PA-MEASUREMENT-REFUSAL");
}
#[test]
fn owner_candidate_operation_rejects_forged_refused_or_mismatched_basis() {
    let proposal: Proposal =
        serde_json::from_str(include_str!("fixtures/measurement/v1/proposal.json")).unwrap();
    let request = serde_json::json!({"contract":"C-PA-MEASUREMENT","version":1,"operation":"candidate","configuration_revision":"1","configuration":proposal.basis_configuration,"proposal":proposal});
    let run = |value: &serde_json::Value| {
        execute(decode_request(&serde_json::to_vec(value).unwrap()).unwrap())
    };
    let candidate = run(&request).unwrap();
    assert_eq!(candidate["contract"], "C-PA-ALIGNMENT-CANDIDATE");
    let graph: GraphConfig =
        serde_json::from_str(candidate["configuration_json"].as_str().unwrap()).unwrap();
    assert_eq!(
        graph,
        proposal
            .candidate_configuration(&proposal.basis_configuration, "1")
            .unwrap()
    );
    let mut bad = request.clone();
    bad["proposal"]["changes"][0]["after_delay_samples"] = 481.into();
    assert!(run(&bad).is_err());
    let mut bad = request.clone();
    bad["proposal"]["status"] = "refused".into();
    assert!(run(&bad).is_err());
    let mut bad = request.clone();
    bad["configuration_revision"] = "2".into();
    assert!(run(&bad).is_err());
    let mut bad = request;
    bad["configuration"]["outputs"][0]["processing"]["limiter_db"] = (-20).into();
    assert!(run(&bad).is_err());
}
