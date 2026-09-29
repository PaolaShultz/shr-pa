use shr_pa::{
    config::{Config, EqBand, EqKind, GEQ_HZ, Layout},
    control::Handoff,
    dsp::{Engine, Prepared},
};
use std::{
    f64::consts::PI,
    sync::{Arc, atomic::Ordering},
};
fn full() -> Config {
    Config {
        layout: Layout::SixFullRange,
        ..Config::default()
    }
}
fn block(e: &mut Engine, x: [f32; 2]) -> [[f32; 6]; 128] {
    let mut y = [[0.; 6]; 128];
    e.render(&[x; 128], &mut y).unwrap();
    y
}
fn settle(e: &mut Engine, x: [f32; 2]) {
    for _ in 0..100 {
        block(e, x);
    }
}
// Complex impulse response, including phase; independent analog prototypes below.
fn response(c: Config, hz: f64, ch: usize) -> (f64, f64) {
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.; 2]);
    let mut result = (0., 0.);
    for b in 0..64 {
        let mut input = [[0.; 2]; 128];
        if b == 0 {
            input[0] = [0.01; 2];
        }
        let mut out = [[0.; 6]; 128];
        e.render(&input, &mut out).unwrap();
        for (i, y) in out.iter().enumerate() {
            let a = -2. * PI * hz * (b * 128 + i) as f64 / c.sample_rate as f64;
            result.0 += y[ch] as f64 / 0.01 * a.cos();
            result.1 += y[ch] as f64 / 0.01 * a.sin();
        }
    }
    result
}
fn div(n: (f64, f64), d: (f64, f64)) -> (f64, f64) {
    let q = d.0 * d.0 + d.1 * d.1;
    ((n.0 * d.0 + n.1 * d.1) / q, (n.1 * d.0 - n.0 * d.1) / q)
}
fn close(a: (f64, f64), b: (f64, f64)) {
    assert!((a.0 - b.0).hypot(a.1 - b.1) < 4e-5, "{a:?} != {b:?}");
}
#[test]
fn shelf_complex_responses_match_warped_analog_prototypes() {
    for kind in [EqKind::LowShelf, EqKind::HighShelf] {
        for db in [-12., 12.] {
            for slope in [0.1, 1.] {
                let mut c = full();
                c.input_eq[0][0] = EqBand {
                    kind,
                    slope,
                    hz: 1000.,
                    db,
                    ..EqBand::default()
                };
                c.bands[1].eq[0] = c.input_eq[0][0];
                let a = 10_f64.powf(db / 40.);
                for hz in [80., 1000., 12000.] {
                    let r = (PI * hz / 48000.).tan() / (PI * 1000. / 48000.).tan();
                    let t = ((a * a + 1.) * (1. / slope - 1.) + 2. * a).sqrt() * r;
                    let expected = if kind == EqKind::LowShelf {
                        div((a * (a - r * r), a * t), (1. - a * r * r, t))
                    } else {
                        div((a * (1. - a * r * r), a * t), (a - r * r, t))
                    };
                    close(response(c, hz, 0), expected);
                    close(response(c, hz, 3), expected); // independent output shelf, right input flat
                }
            }
        }
    }
}

#[test]
fn graphic_eq_center_gain_linking_separation_and_low_rate_policy() {
    let mut c = full();
    c.geq.enabled = true;
    c.geq.db[0][17] = 6.;
    c.geq.db[1][17] = -6.;
    let gain = 10_f64.powf(6. / 20.);
    close(response(c, GEQ_HZ[17], 0), (gain, 0.));
    close(response(c, GEQ_HZ[17], 1), (gain, 0.));
    c.geq.linked = false;
    close(response(c, 1000., 1), (1. / gain, 0.));
    c.geq.enabled = false;
    close(response(c, 1000., 0), (1., 0.));
    c.sample_rate = 8000;
    c.geq.enabled = true;
    c.geq.db = [[0.; 31]; 2];
    c.geq.db[0][30] = 12.;
    close(response(c, 1000., 0), (1., 0.)); // 20 kHz unavailable, not moved into audible band
}
#[test]
fn compressor_static_curve_linking_makeup_attack_release_and_bypass() {
    let mut c = full();
    for b in &mut c.bands {
        b.limiter_db = 0.;
    }
    c.compressor.enabled = true;
    c.compressor.threshold_db = -20.;
    c.compressor.ratio = 4.;
    c.compressor.knee_db = 0.;
    c.compressor.attack_ms = 10.;
    c.compressor.release_ms = 100.;
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.; 2]);
    let mut out = [[0.; 6]; 128];
    // Constant 0 dBFS detector -> -15 dB target, attack reaches 1-exp(-1) at 480 samples.
    for _ in 0..3 {
        e.render(&[[1., 0.25]; 128], &mut out).unwrap();
    }
    let mut tail = [[0.; 6]; 96];
    e.render(&[[1., 0.25]; 96], &mut tail).unwrap();
    let gr = -15. * (1. - (-1_f64).exp());
    assert!((tail[95][0] as f64 - 10_f64.powf(gr / 20.)).abs() < 2e-6);
    assert!((tail[95][1] * 4. - tail[95][0]).abs() < 1e-6);
    settle(&mut e, [1., 0.25]);
    let y = block(&mut e, [1., 0.25]);
    assert!((y[127][0] as f64 - 10_f64.powf(-15. / 20.)).abs() < 2e-6);
    let mut silence = [[0.; 6]; 128];
    for _ in 0..37 {
        e.render(&[[0.01; 2]; 128], &mut silence).unwrap();
    }
    let mut last = [[0.; 6]; 64];
    e.render(&[[0.01; 2]; 64], &mut last).unwrap();
    let expected = 0.01 * 10_f64.powf((-15. * (-1_f64).exp()) / 20.);
    assert!((last[63][0] as f64 - expected).abs() < 2e-7);
    c.compressor.makeup_db = 6.;
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.01; 2]);
    assert!((block(&mut e, [0.01; 2])[127][0] as f64 - 0.01 * 10_f64.powf(6. / 20.)).abs() < 1e-7);
    // Soft knee has the independently computed midpoint reduction: slope * width / 8.
    c.compressor.knee_db = 8.;
    c.compressor.makeup_db = 0.;
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.1; 2]);
    assert!((block(&mut e, [0.1; 2])[127][0] as f64 - 0.1 * 10_f64.powf(-0.75 / 20.)).abs() < 1e-6);
    c.compressor.enabled = false;
    e.apply(Prepared::new(c, false).unwrap()).unwrap();
    settle(&mut e, [0.1; 2]);
    assert!((block(&mut e, [0.1; 2])[127][0] - 0.1).abs() < 1e-6);
}
#[test]
fn coherent_gain_polarity_transition_and_unrelated_filter_delay_state() {
    let mut c = full();
    c.input_delay_ms = 3.;
    c.bands[1].delay_ms = 2.;
    c.bands[1].eq[0].db = 6.;
    let mut e = Engine::new(c).unwrap();
    let mut reference = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    reference.set_mutes([false; 6]);
    for n in 0..30 {
        let x = [(n as f32 * 0.07).sin() * 0.01; 2];
        block(&mut e, x);
        block(&mut reference, x);
    }
    c.bands[0].inverted = true;
    c.bands[0].gain_db = 6.;
    e.apply(Prepared::new(c, false).unwrap()).unwrap();
    let mut previous: Option<f32> = None;
    for _ in 0..16 {
        let y = block(&mut e, [0.01; 2]);
        let r = block(&mut reference, [0.01; 2]);
        for (y, r) in y.iter().zip(r) {
            assert_eq!(y[2], r[2]);
            assert_eq!(y[3], r[3]);
            if let Some(p) = previous {
                assert!((y[0] - p).abs() < 0.001);
            }
            previous = Some(y[0]);
        }
    }
    assert!(!e.busy());
    assert!((block(&mut e, [0.01; 2])[127][0] + 0.01 * 10_f32.powf(6. / 20.)).abs() < 1e-6);
}
#[test]
fn delay_edit_crossfades_existing_history_and_limiter_ceiling_survives_edits() {
    let mut c = full();
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    let mut n = 0;
    let render = |e: &mut Engine, n: &mut usize| {
        let input = std::array::from_fn::<_, 128, _>(|_| {
            let x = (*n as f64 * 2. * PI * 1000. / 48000.).sin() as f32 * 0.1;
            *n += 1;
            [x; 2]
        });
        let mut out = [[0.; 6]; 128];
        e.render(&input, &mut out).unwrap();
        (input, out)
    };
    for _ in 0..20 {
        render(&mut e, &mut n);
    }
    c.bands[0].delay_ms = 0.5;
    e.apply(Prepared::new(c, false).unwrap()).unwrap();
    let mut t = 0;
    for _ in 0..8 {
        let (x, y) = render(&mut e, &mut n);
        for i in 0..128 {
            t += 1;
            let mix = (t as f64 / 960.).min(1.);
            let expected = x[i][0] as f64 * (1. - 2. * mix);
            assert!((y[i][0] as f64 - expected).abs() < 2e-7);
        }
    }
    c.bands[0].limiter_db = -40.;
    e.apply(Prepared::new(c, false).unwrap()).unwrap();
    for _ in 0..8 {
        render(&mut e, &mut n);
    }
    let (_, y) = render(&mut e, &mut n);
    assert!(y.iter().all(|f| f[0].abs() <= 0.01000001));
}
#[test]
fn reconfigure_mutes_before_install_and_fault_overrides_recall() {
    let mut c = full();
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.1; 2]);
    c.layout = Layout::FullRange;
    e.apply(Prepared::new(c, true).unwrap()).unwrap();
    let a = block(&mut e, [0.1; 2]);
    assert!(a[0][0] > 0.09);
    block(&mut e, [0.1; 2]);
    assert!(block(&mut e, [0.1; 2]).iter().all(|f| *f == [0.; 6]));
    settle(&mut e, [0.1; 2]);
    let y = block(&mut e, [0.1; 2]);
    assert!(y.iter().all(|f| f[2..] == [0.; 4]));
    block(&mut e, [f32::NAN, 0.]);
    assert!(e.apply(Prepared::new(c, true).unwrap()).is_err());
    e.set_mutes([false; 6]);
    assert!(block(&mut e, [0.1; 2]).iter().all(|f| *f == [0.; 6]));
}
#[test]
fn handoff_bounded_backpressure_concurrent_publication_and_fault_rejection() {
    let shared = Arc::new(Handoff::default());
    let control = shared.clone();
    let thread = std::thread::spawn(move || {
        for n in 0..100 {
            let mut c = full();
            c.input_gain_db = if n % 2 == 0 { -6. } else { 0. };
            c.bands[0].gain_db = -c.input_gain_db;
            let p = Prepared::new(c, false).unwrap();
            while control.publish(p).is_err() {
                std::thread::yield_now();
            }
        }
    });
    let mut e = Engine::new(full()).unwrap();
    e.set_mutes([false; 6]);
    while shared.accepted.load(Ordering::Acquire) < 100 {
        shared.service(&mut e);
        block(&mut e, [0.01; 2]);
    }
    thread.join().unwrap();
    assert_eq!(shared.rejected.load(Ordering::Acquire), 0);
    settle(&mut e, [0.01; 2]);
    assert!((block(&mut e, [0.01; 2])[127][0] - 0.01).abs() < 1e-6);
    let p = Prepared::new(full(), false).unwrap();
    shared.publish(p).unwrap();
    assert!(shared.publish(p).is_err());
    block(&mut e, [f32::NAN, 0.]);
    shared.service(&mut e);
    assert_eq!(shared.rejected.load(Ordering::Acquire), 1);
}
#[test]
fn explicit_legacy_migration_preserves_meanings_and_rejects_unknowns() {
    let root = std::env::temp_dir().join(format!("shr-pa-migration-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let old = root.join("old.json");
    let new = root.join("new.json");
    let mut c = full();
    c.input_eq[0][2].db = 4.;
    let mut v = serde_json::to_value(c).unwrap();
    v["version"] = 1.into();
    for k in ["geq", "compressor", "input_eq_enabled"] {
        v.as_object_mut().unwrap().remove(k);
    }
    fn legacy(es: &mut serde_json::Value) {
        for e in es.as_array_mut().unwrap() {
            for k in ["kind", "slope"] {
                e.as_object_mut().unwrap().remove(k);
            }
        }
    }
    for es in v["input_eq"].as_array_mut().unwrap() {
        legacy(es);
    }
    for b in v["bands"].as_array_mut().unwrap() {
        b.as_object_mut().unwrap().remove("eq_enabled");
        legacy(&mut b["eq"]);
    }
    std::fs::write(&old, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        Config::load(&old)
            .unwrap_err()
            .to_string()
            .contains("migrate")
    );
    shr_pa::config::migrate_v1(&old, &new).unwrap();
    assert_eq!(Config::load(&new).unwrap(), c);
    v["mystery"] = true.into();
    std::fs::write(&old, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(shr_pa::config::migrate_v1(&old, &new).is_err());
    assert_eq!(Config::load(&new).unwrap(), c);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn filter_edit_is_a_bounded_crossfade_of_independent_old_and_new_responses() {
    let mut c = full();
    c.input_eq[0][0].db = 12.;
    c.input_eq[0][0].q = 15.909;
    let mut old = Engine::new(c).unwrap();
    let mut actual = Engine::new(c).unwrap();
    c.input_eq[0][0].kind = EqKind::LowShelf;
    c.input_eq[0][0].slope = 0.1;
    c.input_eq[0][0].db = -12.;
    let mut new = Engine::new(c).unwrap();
    for e in [&mut old, &mut actual, &mut new] {
        e.set_mutes([false; 6]);
    }
    for n in 0..10 {
        let x = [(n as f32 * 0.2).sin() * 0.01; 2];
        block(&mut old, x);
        block(&mut actual, x);
        block(&mut new, [0.; 2]);
    }
    actual.apply(Prepared::new(c, false).unwrap()).unwrap();
    let mut t = 0;
    for n in 0..16 {
        let x = [(n as f32 * 0.3).cos() * 0.01; 2];
        let a = block(&mut actual, x);
        let o = block(&mut old, x);
        let b = block(&mut new, x);
        for i in 0..128 {
            t += 1;
            let mix = (t as f64 / 960.).min(1.);
            let expected = o[i][0] as f64 * (1. - mix) + b[i][0] as f64 * mix;
            assert!((a[i][0] as f64 - expected).abs() < 1e-7);
            assert_eq!(a[i][1], o[i][1]);
        }
    }
}
#[test]
fn new_schema_bounds_and_live_rate_rejection_preserve_engine() {
    for mutate in [
        |c: &mut Config| c.geq.db[1][30] = f64::NAN,
        |c: &mut Config| c.compressor.ratio = 0.9,
        |c: &mut Config| c.compressor.attack_ms = 0.,
        |c: &mut Config| c.compressor.release_ms = f64::INFINITY,
        |c: &mut Config| c.compressor.knee_db = 25.,
        |c: &mut Config| c.input_eq[0][0].slope = 1.01,
        |c: &mut Config| c.bands[0].eq[0].slope = 0.,
    ] {
        let mut c = full();
        mutate(&mut c);
        assert!(Prepared::new(c, false).is_err());
    }
    let mut e = Engine::new(full()).unwrap();
    e.set_mutes([false; 6]);
    settle(&mut e, [0.01; 2]);
    let c = Config {
        sample_rate: 44100,
        ..full()
    };
    assert!(e.apply(Prepared::new(c, true).unwrap()).is_err());
    assert!(!e.busy());
    assert_eq!(block(&mut e, [0.01; 2])[127], [0.01; 6]);
}
