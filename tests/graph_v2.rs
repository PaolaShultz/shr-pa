use shr_pa::{ffi_v2::*, graph::*};
use std::ptr;
fn fixture(name: &str) -> GraphConfig {
    serde_json::from_str(match name {
        "3" => include_str!("fixtures/cpa/v2/stereo3way.json"),
        "4" => include_str!("fixtures/cpa/v2/stereo4way.json"),
        _ => include_str!("fixtures/cpa/v2/matrix4x8.json"),
    })
    .unwrap()
}
fn ready(config: GraphConfig) -> Graph {
    let mut graph = Graph::prepare(config).unwrap();
    graph.rearm().unwrap();
    let ni = graph.config().inputs.len();
    let no = graph.config().outputs.len();
    graph
        .process(&vec![0.; ni * 256], &mut vec![0.; no * 256])
        .unwrap();
    graph
}
// Independent analog LR24 reference after bilinear prewarping; no production
// coefficient or filter helper used. Complex arithmetic avoids magnitude-only acceptance.
#[derive(Clone, Copy, Debug)]
struct C(f64, f64);
impl C {
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
    }
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
    fn div(self, b: Self) -> Self {
        let d = b.0 * b.0 + b.1 * b.1;
        Self(
            (self.0 * b.0 + self.1 * b.1) / d,
            (self.1 * b.0 - self.0 * b.1) / d,
        )
    }
    fn norm(self) -> f64 {
        self.0.hypot(self.1)
    }
}
fn lr(hz: f64, f: f64, high: bool) -> C {
    let k = (std::f64::consts::PI * f / 48000.).tan() / (std::f64::consts::PI * hz / 48000.).tan();
    let butter = C(if high { -k * k } else { 1. }, 0.).div(C(1. - k * k, 2_f64.sqrt() * k));
    butter.mul(butter)
}
fn response(samples: &[f64], channel: usize, channels: usize, f: f64) -> C {
    samples
        .chunks_exact(channels)
        .enumerate()
        .fold(C(0., 0.), |sum, (n, x)| {
            let a = -2. * std::f64::consts::PI * f * n as f64 / 48000.;
            sum.add(C(a.cos() * x[channel], a.sin() * x[channel]))
        })
}
#[test]
fn every_three_and_four_way_branch_matches_independent_complex_reference() {
    for name in ["3", "4"] {
        let c = fixture(name);
        let ni = c.inputs.len();
        let no = c.outputs.len();
        let mut g = ready(c.clone());
        let mut rendered = Vec::new();
        for block in 0..128 {
            let mut input = vec![0.; ni * 256];
            if block == 0 {
                input[0] = 0.01;
                input[1] = -0.007;
            }
            let mut output = vec![0.; no * 256];
            g.process(&input, &mut output).unwrap();
            rendered.extend(output);
        }
        for frequency in [
            23., 80., 100., 120., 300., 600., 1800., 3000., 9000., 18000.,
        ] {
            let mut total = C(0., 0.);
            for (ch, o) in c.outputs.iter().enumerate() {
                let mut expected = C(1., 0.);
                for (j, &split) in o.splits_hz.iter().enumerate() {
                    expected = expected.mul(if j < o.band {
                        lr(split, frequency, true)
                    } else if j == o.band {
                        lr(split, frequency, false)
                    } else {
                        lr(split, frequency, false).add(lr(split, frequency, true))
                    });
                }
                let measured = response(&rendered, ch, no, frequency)
                    .div(C(if ch % 2 == 0 { 0.01 } else { -0.007 }, 0.));
                assert!(
                    C(measured.0 - expected.0, measured.1 - expected.1).norm() < 1e-8,
                    "{name} branch {ch} f={frequency}: {measured:?} != {expected:?}"
                );
                if ch % 2 == 0 {
                    total = total.add(measured);
                }
            }
            let sum_reference = c.outputs[0].splits_hz.iter().fold(C(1., 0.), |s, &split| {
                s.mul(lr(split, frequency, false).add(lr(split, frequency, true)))
            });
            assert!((total.norm() - 1.).abs() < 1e-8);
            assert!(C(total.0 - sum_reference.0, total.1 - sum_reference.1).norm() < 1e-8);
        }
    }
}
#[test]
fn weighted_matrix_has_no_hidden_normalization_and_every_input_is_real() {
    let mut c = fixture("matrix");
    for o in &mut c.outputs {
        o.splits_hz.clear();
        o.band = 0;
    }
    let mut g = ready(c.clone());
    let mut output = vec![0.; 8 * 256];
    for source in 0..4 {
        let mut input = vec![0.; 4 * 256];
        for frame in input.chunks_exact_mut(4) {
            frame[source] = 0.1;
        }
        g.process(&input, &mut output).unwrap();
        let (side, weight) = match source {
            0 => (0, 0.5),
            1 => (1, 0.75),
            2 => (0, 0.25),
            _ => (1, -0.5),
        };
        for frame in output.chunks_exact(8) {
            for (ch, &x) in frame.iter().enumerate() {
                assert!((x - if ch % 2 == side { 0.1 * weight } else { 0. }).abs() < 1e-12);
            }
        }
    }
    // Non-fixture capacities, no shared fixed arrays or ignored high channels.
    for n in [16, 32, 48, 53] {
        let mut c = GraphConfig::stereo(vec![]);
        c.inputs = vec![Input::default(); n];
        c.outputs = (0..n)
            .map(|i| {
                let mut o = c.outputs[0].clone();
                o.source = Some(Source::Input(i));
                o
            })
            .collect();
        let mut g = ready(c);
        let input: Vec<_> = (0..256)
            .flat_map(|_| (0..n).map(|i| (i + 1) as f64 / 1000.))
            .collect();
        let mut output = vec![0.; input.len()];
        g.process(&input, &mut output).unwrap();
        assert_eq!(input, output);
    }
}
#[test]
fn protection_after_sums_eq_gain_and_delay_and_unused_output_silence() {
    let mut c = fixture("matrix");
    for node in &mut c.nodes {
        for route in &mut node.routes {
            route.weight = 16.;
        }
    }
    for o in &mut c.outputs {
        o.processing.gain_db = 20.;
        o.processing.delay_ms = 10.;
        o.processing.limiter_db = -20.;
    }
    c.outputs[7].source = None;
    let mut g = ready(c);
    let input = vec![100.; 4 * 256];
    let mut output = vec![0.; 8 * 256];
    for _ in 0..12 {
        g.process(&input, &mut output).unwrap();
        for frame in output.chunks_exact(8) {
            assert!(frame.iter().all(|x| x.abs() <= 0.1 + 1e-15));
            assert_eq!(frame[7], 0.);
        }
    }
    assert!(output.iter().any(|x| x.abs() > 0.01));
}
#[test]
fn validation_rejects_cycles_invalid_ports_resource_overflow_and_unknown_schema() {
    let mut c = fixture("matrix");
    c.nodes[0].routes[0].source = Source::Node(1);
    assert!(c.validate().unwrap_err().contains("cyclic"));
    c = fixture("matrix");
    c.outputs[0].source = Some(Source::Input(4));
    assert!(c.validate().is_err());
    c = fixture("4");
    c.outputs[0].splits_hz.reverse();
    assert!(c.validate().is_err());
    c = fixture("3");
    c.inputs = vec![Input::default(); 4097];
    assert!(c.validate().unwrap_err().contains("budget"));
    c = fixture("3");
    c.inputs[0].gain_db = f64::NAN;
    assert!(c.validate().is_err());
    // Nearest-sample delay allocation exceeds 64 MiB here although floor-based
    // admission would accept 8,388,557 samples. Reserve conservatively.
    c = GraphConfig::stereo(vec![]);
    c.sample_rate = 191865;
    c.inputs = vec![Input::default(); 437];
    assert!(c.validate().unwrap_err().contains("64 MiB"));
    assert!(serde_json::from_str::<GraphConfig>("{\"version\":1,\"unexpected\":true}").is_err());
}
unsafe fn prepared(c: &GraphConfig) -> *mut ShrPaV2 {
    let j = serde_json::to_vec(c).unwrap();
    unsafe { shr_pa_v2_prepare(j.as_ptr(), j.len() as u32, 2) }
}
unsafe fn status(h: *mut ShrPaV2) -> ShrPaStatusV2 {
    let mut s = ShrPaStatusV2::default();
    assert_eq!(unsafe { shr_pa_v2_status(h, &mut s, 2, 80) }, 0);
    s
}
#[test]
fn abi_exact_shapes_persistent_mute_timeline_retirement_and_failed_atomic_swap() {
    unsafe {
        let c = fixture("4");
        let p = prepared(&c);
        assert!(!p.is_null());
        let mut active = ptr::null_mut();
        let mut retired = ptr::null_mut();
        let input = vec![0.01; 512];
        let mut output = vec![77.; 2048];
        assert_eq!(
            shr_pa_v2_process(p, input.as_ptr(), output.as_mut_ptr(), 2, 8, 256, 1, 0),
            -1
        );
        assert_eq!(output[0], 77.);
        assert_eq!(shr_pa_v2_apply(&mut active, p, &mut retired, 1, 0), 0);
        assert!(retired.is_null());
        assert_eq!(
            shr_pa_v2_process(active, input.as_ptr(), output.as_mut_ptr(), 2, 8, 256, 1, 0),
            0
        );
        assert!(output.iter().all(|x| *x == 0.));
        assert_eq!(shr_pa_v2_rearm(active, 1, 256), 0);
        assert_eq!(
            shr_pa_v2_process(
                active,
                input.as_ptr(),
                output.as_mut_ptr(),
                2,
                8,
                256,
                1,
                256
            ),
            0
        );
        assert!(output.iter().any(|x| *x != 0.));
        let next = prepared(&fixture("3"));
        let old = active;
        assert_eq!(shr_pa_v2_apply(&mut active, next, &mut retired, 1, 512), -3);
        assert_eq!(active, old);
        assert!(retired.is_null());
        assert_eq!(shr_pa_v2_mute(active), 0);
        assert_eq!(
            shr_pa_v2_process(
                active,
                input.as_ptr(),
                output.as_mut_ptr(),
                2,
                8,
                256,
                1,
                512
            ),
            0
        );
        assert_eq!(status(active).quiesced, 1);
        assert_eq!(shr_pa_v2_apply(&mut active, next, &mut retired, 1, 0), -4);
        assert_eq!(active, old);
        assert_eq!(shr_pa_v2_apply(&mut active, next, &mut retired, 1, 768), 0);
        assert_eq!(retired, old);
        assert_eq!(status(active).generation, 2);
        assert_eq!(status(active).muted, 1);
        let later = prepared(&c);
        assert_eq!(
            shr_pa_v2_apply(&mut active, later, &mut retired, 1, 768),
            -3
        );
        assert_eq!(
            shr_pa_v2_process(
                retired,
                input.as_ptr(),
                output.as_mut_ptr(),
                2,
                8,
                256,
                1,
                768
            ),
            -1
        );
        shr_pa_v2_destroy(retired);
        retired = ptr::null_mut();
        assert_eq!(shr_pa_v2_apply(&mut active, later, &mut retired, 2, 0), 0);
        shr_pa_v2_destroy(retired);
        let mut s = status(active);
        s.size = 123;
        assert_eq!(shr_pa_v2_status(active, &mut s, 2, 79), -1);
        assert_eq!(s.size, 123);
        assert_eq!(
            shr_pa_v2_process(active, input.as_ptr(), output.as_mut_ptr(), 2, 8, 256, 1, 0),
            -4
        );
        assert!(output.iter().all(|x| *x == 0.));
        assert_eq!(status(active).fault_latched, 1);
        assert_eq!(shr_pa_v2_rearm(active, 2, 0), -2);
        shr_pa_v2_destroy(active);
    }
}
#[test]
fn partition_equivalence_input_eq_output_polarity_delay_and_latched_numeric_fault() {
    let mut c = fixture("matrix");
    c.inputs[3].eq[0].db = 6.;
    c.outputs[0].processing.inverted = true;
    c.outputs[0].processing.delay_ms = 3.;
    let mut a = ready(c.clone());
    let mut b = ready(c);
    let input: Vec<_> = (0..1024 * 4)
        .map(|i| ((i * 17) as f64).sin() * 0.01)
        .collect();
    let mut oa = vec![0.; 1024 * 8];
    let mut ob = oa.clone();
    for (src, dst) in input.chunks(256 * 4).zip(oa.chunks_mut(256 * 8)) {
        a.process(src, dst).unwrap();
    }
    for (src, dst) in input.chunks(37 * 4).zip(ob.chunks_mut(37 * 8)) {
        b.process(src, dst).unwrap();
    }
    assert_eq!(oa, ob);
    let mut bad = vec![0.; 256 * 4];
    bad[100] = f64::NAN;
    let mut out = vec![42.; 256 * 8];
    assert!(a.process(&bad, &mut out).is_err());
    assert!(out.iter().all(|x| *x == 0.));
    assert!(a.rearm().is_err());
}

#[test]
fn configured_controls_change_real_samples_and_input_delay_is_exact() {
    let base = GraphConfig::stereo(vec![]);
    let mut c = base.clone();
    c.inputs[0].gain_db = -6.;
    c.inputs[0].delay_ms = 2.;
    c.outputs[0].processing.gain_db = -3.;
    c.outputs[0].processing.inverted = true;
    c.outputs[0].processing.delay_ms = 1.;
    c.outputs[1].muted = true;
    let mut g = ready(c);
    let mut input = vec![0.; 512];
    input[0] = 0.1;
    input[1] = 0.1;
    let mut output = vec![0.; 512];
    g.process(&input, &mut output).unwrap();
    for (n, frame) in output.chunks_exact(2).enumerate() {
        assert_eq!(frame[1], 0.);
        if n == 144 {
            assert!((frame[0] + 0.1 * 10_f64.powf(-9. / 20.)).abs() < 1e-15);
        } else {
            assert_eq!(frame[0], 0.);
        }
    }
    for control in 0..4 {
        let mut changed = base.clone();
        match control {
            0 => changed.inputs[0].eq[0].db = 6.,
            1 => {
                changed.inputs[0].geq_enabled = true;
                changed.inputs[0].geq_db[17] = 6.;
            }
            2 => changed.outputs[0].processing.eq[0].db = 6.,
            _ => {
                changed.inputs[0].compressor.enabled = true;
                changed.inputs[0].compressor.threshold_db = -40.;
                changed.inputs[0].compressor.attack_ms = 0.1;
            }
        }
        let mut a = ready(base.clone());
        let mut b = ready(changed);
        let mut energy_a = 0.;
        let mut energy_b = 0.;
        for block in 0..12 {
            let input: Vec<_> = (0..256)
                .flat_map(|i| {
                    [
                        0.05 * (2. * std::f64::consts::PI * 1000. * (block * 256 + i) as f64
                            / 48000.)
                            .sin(),
                        0.,
                    ]
                })
                .collect();
            let mut oa = vec![0.; 512];
            let mut ob = oa.clone();
            a.process(&input, &mut oa).unwrap();
            b.process(&input, &mut ob).unwrap();
            if block > 4 {
                energy_a += oa.iter().map(|x| x * x).sum::<f64>();
                energy_b += ob.iter().map(|x| x * x).sum::<f64>();
            }
        }
        if control < 3 {
            assert!(
                energy_b > energy_a * 3.5,
                "EQ control {control} did not reach samples"
            );
        } else {
            assert!(energy_b < energy_a * 0.5);
        }
    }
}

#[test]
fn abi_alias_shapes_and_versions_refuse_without_writes() {
    unsafe {
        let c = fixture("3");
        let p = prepared(&c);
        let mut active = ptr::null_mut();
        let mut retired = ptr::null_mut();
        assert_eq!(shr_pa_v2_apply(&mut active, p, &mut active, 1, 0), -1);
        assert!(active.is_null());
        assert_eq!(shr_pa_v2_apply(&mut active, p, &mut retired, 1, 0), 0);
        assert_eq!(shr_pa_v2_apply(&mut active, active, &mut retired, 1, 0), -1);
        let mut output = vec![31.; 6 * 256];
        let input = vec![0.; 2 * 256];
        assert_eq!(
            shr_pa_v2_process(
                active,
                output.as_ptr(),
                output.as_mut_ptr(),
                2,
                6,
                256,
                1,
                0
            ),
            -1
        );
        assert_eq!(output[0], 31.);
        assert_eq!(
            shr_pa_v2_process(active, input.as_ptr(), output.as_mut_ptr(), 2, 5, 256, 1, 0),
            -1
        );
        assert_eq!(output[0], 31.);
        assert_eq!(status(active).next_frame, 0);
        assert_eq!(shr_pa_v2_status(active, active.cast(), 2, 80), -1);
        let mut cap = ShrPaCapabilitiesV2 {
            size: 777,
            ..Default::default()
        };
        assert_eq!(shr_pa_v2_capabilities(&mut cap, 1, 64), -1);
        assert_eq!(cap.size, 777);
        let json = serde_json::to_vec(&c).unwrap();
        assert!(shr_pa_v2_prepare(json.as_ptr(), json.len() as u32, 1).is_null());
        shr_pa_v2_destroy(active);
    }
}
