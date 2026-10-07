use shr_pa::{
    config::EqKind,
    graph::{Graph, GraphConfig, Input, Source},
    live_eq::{EqPatch, EqSettings},
};
fn patch(c: &GraphConfig, a: usize, b: usize, db: f64) -> EqPatch {
    let mut inputs = [
        EqSettings::from_input(a, &c.inputs[a]),
        EqSettings::from_input(b, &c.inputs[b]),
    ];
    for e in &mut inputs {
        e.eq[0].db = db;
    }
    EqPatch { version: 1, inputs }
}
#[test]
fn strict_patch_admission_and_owner_lifetime() {
    let c = GraphConfig::stereo(vec![]);
    let mut a = Graph::prepare(c.clone()).unwrap();
    let mut b = Graph::prepare(c.clone()).unwrap();
    let p = patch(&c, 1, 0, 6.);
    let mut prepared = Some(a.prepare_eq(p.clone()).unwrap());
    assert!(b.apply_eq(&mut prepared).is_err());
    assert_eq!(b.eq_progress().generation, 0);
    assert!(a.apply_eq(&mut prepared).is_ok());
    assert!(prepared.is_none());
    assert!(a.prepare_eq(p.clone()).is_err());
    assert!(a.retire_eq().is_none());
    let text = serde_json::to_string(&p).unwrap();
    for bad in [
        text.replace("\"version\":1", "\"version\":1,\"version\":1"),
        text.replace("\"version\":1", "\"version\":1,\"gain_db\":0"),
        text.replace("\"input_index\":1", "\"input_index\":0"),
        text.replace("\"db\":6.0", "\"db\":1e999"),
    ] {
        assert!(EqPatch::parse(bad.as_bytes(), 48000, 2).is_err(), "{bad}");
    }
}
fn process(g: &mut Graph, input: &[f64], chunks: &[usize]) -> Vec<f64> {
    let mut out = vec![0.; input.len() / g.config().inputs.len() * g.config().outputs.len()];
    let ni = g.config().inputs.len();
    let no = g.config().outputs.len();
    let mut start = 0;
    let mut n = 0;
    while start < input.len() / ni {
        let frames = chunks[n % chunks.len()].min(input.len() / ni - start);
        g.process(
            &input[start * ni..(start + frames) * ni],
            &mut out[start * no..(start + frames) * no],
        )
        .unwrap();
        start += frames;
        n += 1;
    }
    out
}
#[test]
fn stereo_endpoints_partition_unselected_and_identical_history() {
    let mut c = GraphConfig::stereo(vec![]);
    c.max_block = 256;
    c.inputs.push(Input::default());
    c.outputs.push(c.outputs[0].clone());
    c.outputs[2].source = Some(Source::Input(2));
    c.inputs[0].eq[0].q = 12.;
    c.inputs[0].eq[0].db = 10.;
    c.inputs[1] = c.inputs[0].clone();
    let mut a = Graph::prepare(c.clone()).unwrap();
    let mut b = Graph::prepare(c.clone()).unwrap();
    let mut untouched = Graph::prepare(c.clone()).unwrap();
    for g in [&mut a, &mut b, &mut untouched] {
        g.rearm().unwrap();
    }
    let seed: Vec<_> = (0..900).map(|i| if i < 3 { 0.2 } else { 0. }).collect();
    for g in [&mut a, &mut b, &mut untouched] {
        process(g, &seed, &[128]);
    }
    let identical = patch(&c, 1, 0, 10.);
    let mut p = Some(a.prepare_eq(identical).unwrap());
    a.apply_eq(&mut p).unwrap();
    let silent = vec![0.; 450];
    assert_eq!(
        process(&mut a, &silent, &[7]),
        process(&mut b, &silent, &[150])
    );
    process(&mut untouched, &silent, &[33]);
    assert_eq!(a.eq_progress().remaining, 0);
    assert!(a.retire_eq().is_some());
    let p = patch(&c, 1, 0, -6.);
    let mut pa = Some(a.prepare_eq(p.clone()).unwrap());
    let mut pb = Some(b.prepare_eq(p).unwrap());
    a.apply_eq(&mut pa).unwrap();
    b.apply_eq(&mut pb).unwrap();
    let signal: Vec<_> = (0..1800)
        .map(|i| ((i / 3) as f64 * 0.21).sin() * 0.1)
        .collect();
    let x = process(&mut a, &signal, &[256]);
    let y = process(&mut b, &signal, &[1, 17, 99]);
    assert_eq!(x, y);
    let reference = process(&mut untouched, &signal, &[256]);
    assert_eq!(&x[..3], &reference[..3]); // exact old endpoint
    for (i, row) in x.chunks_exact(3).enumerate() {
        assert_eq!(row[0], row[1]);
        assert_eq!(row[2], reference[i * 3 + 2]);
    }
    assert_eq!(a.eq_progress().remaining, 0);
    assert!(a.eq_progress().retirement_occupied);
    assert!(a.retire_eq().is_some());
    assert!(!a.eq_progress().retirement_occupied);
}
#[test]
fn owner_coefficients_independent_rbj_bell_and_rate_identity() {
    let mut c = GraphConfig::stereo(vec![]);
    c.sample_rate = 8000;
    c.inputs[0].geq_enabled = true;
    c.inputs[0].geq_db = [12.; 31];
    c.inputs[0].eq[0].db = 6.;
    c.inputs[0].eq[0].q = 4.318;
    let settings = EqSettings::from_input(0, &c.inputs[0]);
    let bank = settings.coefficients(8000).unwrap();
    let e = settings.eq[0];
    let w = std::f64::consts::TAU * e.hz / 8000.;
    let a = 10f64.powf(e.db / 40.);
    let alpha = w.sin() / (2. * e.q);
    let norm = 1. + alpha / a;
    let expected = [
        (1. + alpha * a) / norm,
        -2. * w.cos() / norm,
        (1. - alpha * a) / norm,
        -2. * w.cos() / norm,
        (1. - alpha / a) / norm,
    ];
    for (x, y) in bank[0].iter().zip(expected) {
        assert!((x - y).abs() < 1e-14);
    }
    for (i, hz) in shr_pa::config::GEQ_HZ.iter().enumerate() {
        if *hz > 3600. {
            assert_eq!(bank[i + 8], [1., 0., 0., 0., 0.]);
        }
    }
    for kind in [EqKind::Bell, EqKind::LowShelf, EqKind::HighShelf] {
        c.inputs[0].eq[0].kind = kind;
        c.inputs[0].eq[0].db = -12.;
        c.inputs[0].eq[0].slope = 0.1;
        assert!(
            EqSettings::from_input(0, &c.inputs[0])
                .coefficients(8000)
                .unwrap()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
}

#[derive(Clone)]
struct ReferenceBank {
    coefficients: Vec<[f64; 5]>,
    histories: Vec<[f64; 4]>,
}
impl ReferenceBank {
    fn new(e: &EqSettings) -> Self {
        Self {
            coefficients: e.coefficients(48000).unwrap(),
            histories: vec![[0.; 4]; 39],
        }
    }
    fn tick(&mut self, mut x: f64) -> f64 {
        for (c, h) in self.coefficients.iter().zip(&mut self.histories) {
            let y = c[0] * x + c[1] * h[0] + c[2] * h[1] - c[3] * h[2] - c[4] * h[3];
            *h = [x, h[0], y, h[2]];
            x = y;
        }
        x
    }
}
#[test]
fn independent_blend_preserves_compressor_delay_crossover_limiter_and_output_delay() {
    let mut c = GraphConfig::stereo(vec![400., 2500.]);
    c.max_block = 256;
    c.inputs.resize(4, Input::default());
    for o in &mut c.outputs {
        if let Some(Source::Input(i)) = o.source {
            o.source = Some(Source::Input(if i == 0 { 3 } else { 1 }));
        }
        o.processing.limiter_db = -25.;
        o.processing.release_ms = 500.;
        o.processing.delay_ms = 3.3;
        o.processing.eq[0].db = 3.;
        o.processing.eq[0].hz = 1700.;
    }
    for i in &mut c.inputs {
        i.eq[0].db = 7.;
        i.eq[0].q = 10.;
        i.gain_db = 2.;
        i.delay_ms = 1.7;
        i.compressor.enabled = true;
        i.compressor.threshold_db = -35.;
        i.compressor.ratio = 7.;
        i.compressor.attack_ms = 1.;
        i.compressor.release_ms = 300.;
    }
    let mut downstream = c.clone();
    for i in &mut downstream.inputs {
        i.eq_enabled = false;
        i.geq_enabled = false;
        i.gain_db = 0.;
    }
    let mut actual = Graph::prepare(c.clone()).unwrap();
    let mut reference = Graph::prepare(downstream).unwrap();
    actual.rearm().unwrap();
    reference.rearm().unwrap();
    let mut old: Vec<_> = c
        .inputs
        .iter()
        .enumerate()
        .map(|(i, e)| ReferenceBank::new(&EqSettings::from_input(i, e)))
        .collect();
    let gain = 10f64.powf(2. / 20.);
    let mut p = patch(&c, 3, 1, -8.);
    p.inputs[1] = EqSettings::from_input(1, &c.inputs[1]);
    let mut target = ReferenceBank::new(&p.inputs[0]);
    for frame in 0..2600 {
        if frame == 1100 {
            let mut prepared = Some(actual.prepare_eq(p.clone()).unwrap());
            actual.apply_eq(&mut prepared).unwrap();
        }
        let source: Vec<_> = (0..4)
            .map(|i| {
                if frame < 1150 {
                    ((frame as f64) * 0.08 + i as f64).sin() * 0.2
                } else {
                    0.
                }
            })
            .collect();
        let mut upstream: Vec<_> = old
            .iter_mut()
            .zip(&source)
            .map(|(b, x)| b.tick(*x * gain))
            .collect();
        if frame >= 1100 {
            let new = target.tick(source[3] * gain);
            let w = ((frame - 1100) as f64 / 239.).min(1.);
            upstream[3] = upstream[3] * (1. - w) + new * w;
        }
        let mut a = [0.; 6];
        let mut b = [0.; 6];
        actual.process(&source, &mut a).unwrap();
        reference.process(&upstream, &mut b).unwrap();
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 2e-10, "frame {frame}: {x} {y}");
            assert!(x.is_finite() && x.abs() <= 10f64.powf(-25. / 20.));
        }
        if frame == 1339 {
            assert_eq!(actual.eq_progress().remaining, 0);
            assert!(actual.retire_eq().is_some());
        }
    }
}
#[test]
fn independent_one_side_preserves_other_selected_ringing_exactly() {
    let mut c = GraphConfig::stereo(vec![]);
    c.inputs.resize(4, Input::default());
    c.outputs[0].source = Some(Source::Input(3));
    c.outputs[1].source = Some(Source::Input(1));
    c.inputs[1].eq[0].db = 12.;
    c.inputs[1].eq[0].q = 15.;
    let mut a = Graph::prepare(c.clone()).unwrap();
    let mut b = Graph::prepare(c.clone()).unwrap();
    a.rearm().unwrap();
    b.rearm().unwrap();
    let seed: Vec<_> = (0..1200).map(|i| if i == 1 { 0.3 } else { 0. }).collect();
    process(&mut a, &seed, &[100]);
    process(&mut b, &seed, &[100]);
    let mut p = patch(&c, 3, 1, 6.);
    p.inputs[1] = EqSettings::from_input(1, &c.inputs[1]);
    let mut prepared = Some(a.prepare_eq(p).unwrap());
    a.apply_eq(&mut prepared).unwrap();
    let silent = vec![0.; 2400];
    let x = process(&mut a, &silent, &[13]);
    let y = process(&mut b, &silent, &[113]);
    for (a, b) in x.chunks_exact(2).zip(y.chunks_exact(2)) {
        assert_eq!(a[1], b[1]);
    }
    assert!(a.retire_eq().is_some());
}

#[test]
fn bounded_extreme_banks_bypass_and_persistent_mute_remain_finite_and_protected() {
    for rate in [8000, 48000, 192000] {
        let mut c = GraphConfig::stereo(vec![]);
        c.sample_rate = rate;
        let mut g = Graph::prepare(c.clone()).unwrap();
        let mut p = patch(&c, 1, 0, 12.);
        for e in &mut p.inputs {
            e.geq_enabled = true;
            e.geq_db = [12.; 31];
            for (i, band) in e.eq.iter_mut().enumerate() {
                band.kind = [EqKind::Bell, EqKind::LowShelf, EqKind::HighShelf][i % 3];
                band.hz = if i % 2 == 0 {
                    20.
                } else {
                    (rate as f64 * 0.45).min(20000.)
                };
                band.q = if i % 2 == 0 { 0.1 } else { 15.909 };
                band.slope = 0.1;
                band.db = if i % 2 == 0 { 12. } else { -12. };
            }
        }
        let mut prepared = Some(g.prepare_eq(p).unwrap());
        g.apply_eq(&mut prepared).unwrap();
        let input = vec![0.2; 512];
        let mut output = vec![1.; 512];
        for _ in 0..5 {
            g.process(&input, &mut output).unwrap();
            assert!(output.iter().all(|x| *x == 0.));
        }
        assert!(g.retire_eq().is_some());
        g.rearm().unwrap();
        for _ in 0..5 {
            g.process(&input, &mut output).unwrap();
            assert!(output.iter().all(|x| x.is_finite() && x.abs() <= 1.));
        }
        let mut bypass = patch(g.config(), 1, 0, 0.);
        for e in &mut bypass.inputs {
            e.eq_enabled = false;
            e.geq_enabled = false;
        }
        let mut prepared = Some(g.prepare_eq(bypass).unwrap());
        g.apply_eq(&mut prepared).unwrap();
        for _ in 0..5 {
            g.process(&input, &mut output).unwrap();
            assert!(output.iter().all(|x| x.is_finite() && x.abs() <= 1.));
        }
    }
}
