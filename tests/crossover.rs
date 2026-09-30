use shr_pa::{
    config::{Config, Crossover, Edge, Family, Layout, PairEdges},
    dsp::{Engine, Prepared},
    ui::{Editor, Page},
};
use std::f64::consts::PI;
#[derive(Clone, Copy, Debug)]
struct C(f64, f64);
impl C {
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
    }
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
    fn scale(self, b: f64) -> Self {
        Self(self.0 * b, self.1 * b)
    }
    fn div(self, b: Self) -> Self {
        self.mul(Self(b.0, -b.1))
            .scale(1. / (b.0 * b.0 + b.1 * b.1))
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
}
// Independent analog reference: multiply all left-half-plane unit-circle poles.
// No production coefficients, biquad factoring or Q values are used here.
fn reference(e: Edge, rate: u32, hz: f64, high: bool) -> C {
    if e.bypass {
        return C(1., 0.);
    }
    let repeat = if e.family == Family::LinkwitzRiley {
        2
    } else {
        1
    };
    let order = e.slope as usize / 6 / repeat;
    let r = (PI * hz / rate as f64).tan() / (PI * e.hz / rate as f64).tan();
    let s = C(0., r);
    let mut den = C(1., 0.);
    let mut num = C(1., 0.);
    for k in 0..order {
        let theta = PI * (2 * k + 1 + order) as f64 / (2 * order) as f64;
        den = den.mul(s.add(C(-theta.cos(), -theta.sin())));
        if high {
            num = num.mul(s);
        }
    }
    let h = num.div(den);
    if repeat == 2 { h.mul(h) } else { h }
}
fn close(a: C, b: C, tol: f64) {
    assert!(a.add(b.scale(-1.)).abs() < tol, "{a:?} != {b:?}");
}
fn impulse(c: Config, frames: usize) -> Vec<[f32; 6]> {
    let mut engine = Engine::new(c).unwrap();
    engine.set_mutes([false; 6]);
    let mut block = [[0.; 6]; 128];
    for _ in 0..10 {
        engine.render(&[[0.; 2]; 128], &mut block).unwrap();
    }
    let mut out = vec![[0.; 6]; frames];
    for (i, b) in out.chunks_mut(128).enumerate() {
        let mut input = [[0.; 2]; 128];
        if i == 0 {
            input[0] = [0.01, 0.005];
        }
        engine.render(&input[..b.len()], b).unwrap();
    }
    assert!(!engine.faulted());
    out
}
fn dft(out: &[[f32; 6]], ch: usize, hz: f64, rate: u32) -> C {
    out.iter().enumerate().fold(C(0., 0.), |sum, (n, y)| {
        let a = -2. * PI * hz * n as f64 / rate as f64;
        sum.add(C(a.cos(), a.sin()).scale(y[ch] as f64 / 0.01))
    })
}
fn config(edge: Edge, high: bool, rate: u32) -> Config {
    let bypass = Edge::lr24(1000., true);
    let pair = PairEdges {
        hp: if high { edge } else { bypass },
        lp: if high { bypass } else { edge },
    };
    Config {
        sample_rate: rate,
        layout: Layout::SixFullRange,
        crossover: Crossover::Independent([pair; 3]),
        ..Config::default()
    }
}
#[test]
fn all_orders_complex_cutoff_stopband_and_stereo_reference() {
    for rate in [44100, 48000, 96000] {
        for family in [Family::Butterworth, Family::LinkwitzRiley] {
            for slope in (6..=48)
                .step_by(6)
                .filter(|s| family == Family::Butterworth || s % 12 == 0)
            {
                let e = Edge {
                    bypass: false,
                    hz: 1000.,
                    family,
                    slope,
                };
                for high in [false, true] {
                    let out = impulse(config(e, high, rate), 8192);
                    for hz in [125., 250., 500., 1000., 2000., 4000., 8000.] {
                        close(dft(&out, 0, hz, rate), reference(e, rate, hz, high), 3e-6);
                    }
                    let cutoff = dft(&out, 0, 1000., rate).abs();
                    assert!(
                        (cutoff
                            - if family == Family::Butterworth {
                                0.5_f64.sqrt()
                            } else {
                                0.5
                            })
                        .abs()
                            < 3e-6
                    );
                    for y in &out {
                        for pair in 0..3 {
                            assert_eq!(y[pair * 2], y[0]);
                            assert!((y[pair * 2 + 1] * 2. - y[0]).abs() < 1e-28);
                        }
                    }
                    // Stopband slope measured per octave in bilinear-warped frequency.
                    let stop = if slope <= 24 { [10., 20.] } else { [2., 4.] };
                    let ratios = if high { stop.map(|r| 1. / r) } else { stop };
                    let magnitudes = ratios.map(|r| {
                        let hz = ((PI * 1000. / rate as f64).tan() * r).atan() * rate as f64 / PI;
                        dft(&out, 0, hz, rate).abs()
                    });
                    {
                        assert!(
                            (20. * (magnitudes[1] / magnitudes[0]).log10() + slope as f64).abs()
                                < 0.3
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn extremes_bypass_and_independent_frequency_validation() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for family in [Family::Butterworth, Family::LinkwitzRiley] {
            for slope in (6..=48)
                .step_by(6)
                .filter(|s| family == Family::Butterworth || s % 12 == 0)
            {
                for hz in [16., (rate as f64 * 0.45).min(20000.)] {
                    let e = Edge {
                        bypass: false,
                        hz,
                        family,
                        slope,
                    };
                    Prepared::new(config(e, true, rate), false).unwrap();
                    Prepared::new(config(e, false, rate), false).unwrap();
                }
            }
        }
    }
    for (rate, hz) in [(8000, 16.), (8000, 3600.), (192000, 16.), (192000, 20000.)] {
        for family in [Family::Butterworth, Family::LinkwitzRiley] {
            let edge = Edge {
                bypass: false,
                hz,
                family,
                slope: 48,
            };
            for high in [false, true] {
                let out = impulse(config(edge, high, rate), rate as usize);
                close(
                    dft(&out, 0, hz, rate),
                    reference(edge, rate, hz, high),
                    1e-5,
                );
            }
        }
    }
    let edge = Edge {
        bypass: true,
        hz: 1000.,
        family: Family::Butterworth,
        slope: 48,
    };
    let out = impulse(config(edge, true, 48000), 1024);
    assert_eq!(out[0], [0.01, 0.005, 0.01, 0.005, 0.01, 0.005]);
    assert!(out[1..].iter().all(|x| *x == [0.; 6]));
    for hz in [15., 20001., f64::NAN, f64::INFINITY] {
        assert!(config(Edge { hz, ..edge }, true, 48000).validate().is_err());
    }
    for slope in [0, 7, 54, 255] {
        assert!(
            config(Edge { slope, ..edge }, true, 48000)
                .validate()
                .is_err()
        );
    }
    assert!(
        config(
            Edge {
                family: Family::LinkwitzRiley,
                slope: 18,
                ..edge
            },
            true,
            48000
        )
        .validate()
        .is_err()
    );
}
#[test]
fn matched_sums_polarity_and_arbitrary_edges_have_no_flat_sum_assumption() {
    for (family, slopes) in [
        (Family::Butterworth, vec![6, 18, 30, 42]),
        (Family::LinkwitzRiley, vec![12, 24, 36, 48]),
    ] {
        for slope in slopes {
            let edge = Edge {
                bypass: false,
                hz: 1000.,
                family,
                slope,
            };
            let mut c = config(edge, true, 48000);
            c.layout = Layout::TwoWay;
            let Crossover::Independent(ref mut pairs) = c.crossover else {
                unreachable!()
            };
            pairs[2] = PairEdges {
                hp: Edge::lr24(1000., true),
                lp: edge,
            };
            let sign = if family == Family::LinkwitzRiley && slope % 24 != 0 {
                -1.
            } else {
                1.
            };
            c.bands[0].inverted = sign < 0.;
            let out = impulse(c, 8192);
            for hz in [100., 500., 1000., 2000., 10000.] {
                let actual = dft(&out, 0, hz, 48000).add(dft(&out, 4, hz, 48000));
                let expected = reference(edge, 48000, hz, true)
                    .scale(sign)
                    .add(reference(edge, 48000, hz, false));
                close(actual, expected, 3e-6);
                assert!((actual.abs() - 1.).abs() < 3e-6);
            }
            assert!(out.iter().all(|y| y[2] == 0. && y[3] == 0.));
        }
    }
    // Even-order BW pairs have either cancellation or +3.0103 dB at
    // crossover, never a flat sum. Do not disguise that with automatic gain.
    for slope in [12, 24, 36, 48] {
        for inverted in [false, true] {
            let edge = Edge {
                bypass: false,
                hz: 1000.,
                family: Family::Butterworth,
                slope,
            };
            let mut c = config(edge, true, 48000);
            c.layout = Layout::TwoWay;
            c.bands[0].inverted = inverted;
            let Crossover::Independent(ref mut pairs) = c.crossover else {
                unreachable!()
            };
            pairs[2] = PairEdges {
                hp: Edge::lr24(1000., true),
                lp: edge,
            };
            let out = impulse(c, 8192);
            let sign = if inverted { -1. } else { 1. };
            let sum = dft(&out, 0, 1000., 48000).add(dft(&out, 4, 1000., 48000));
            close(
                sum,
                reference(edge, 48000, 1000., true)
                    .scale(sign)
                    .add(reference(edge, 48000, 1000., false)),
                3e-6,
            );
            assert!((sum.abs() - 1.).abs() > 0.4);
        }
    }
    // An intentional gap and overlap both remain exactly as requested.
    for (hp, lp) in [(2000., 500.), (500., 2000.)] {
        let mut c = config(Edge::lr24(hp, false), true, 48000);
        let Crossover::Independent(ref mut pairs) = c.crossover else {
            unreachable!()
        };
        pairs[0].lp = Edge::lr24(lp, false);
        c.validate().unwrap();
        let out = impulse(c, 8192);
        close(
            dft(&out, 0, 1000., 48000),
            reference(Edge::lr24(hp, false), 48000, 1000., true).mul(reference(
                Edge::lr24(lp, false),
                48000,
                1000.,
                false,
            )),
            3e-6,
        );
    }
}
#[test]
fn independent_layout_roles_bass_and_ui_controls() {
    for layout in [
        Layout::FullRange,
        Layout::External,
        Layout::TwoWay,
        Layout::ThreeWay,
        Layout::SixFullRange,
        Layout::FourPlusSubs,
    ] {
        let mut c = config(Edge::lr24(1000., true), true, 48000);
        c.layout = layout;
        c.mono_bass = true;
        let out = impulse(c, 1024);
        for pair in 0..3 {
            if !c.active_pair(pair) {
                assert!(
                    out.iter()
                        .all(|y| y[pair * 2] == 0. && y[pair * 2 + 1] == 0.)
                );
            }
        }
        if matches!(
            layout,
            Layout::TwoWay | Layout::ThreeWay | Layout::FourPlusSubs
        ) {
            assert!((out[0][4] - 0.0075).abs() < 1e-8);
            assert_eq!(out[0][4], out[0][5]);
        } else if layout == Layout::SixFullRange {
            assert_eq!(out[0][4], 0.01);
        }
    }
    let mut e = Editor::new().unwrap();
    e.config.bands[0].gain_db = -7.;
    e.config.bands[0].inverted = true;
    for _ in 0..6 {
        e.key('v');
    }
    for _ in 0..6 {
        e.key('n');
    }
    e.key('x'); // locked until explicit mode change
    assert_eq!(e.config.crossover, Crossover::LayoutLr24);
    e.key('N');
    e.key('x');
    assert!(matches!(e.config.crossover, Crossover::Independent(_)));
    let before = e.config;
    e.key(']');
    assert_eq!(e.config, before); // split cannot discard independent edits
    for _ in 0..8 {
        e.key('n');
        e.key('x');
    }
    let Crossover::Independent(pairs) = e.config.crossover else {
        panic!()
    };
    assert!(pairs[0].hp.bypass);
    assert!(!pairs[0].lp.bypass);
    assert_eq!(pairs[0].hp.family, Family::Butterworth);
    assert_eq!(pairs[0].hp.slope, 30);
    assert!(pairs[0].hp.hz > 1800.);
    assert!(pairs[0].lp.hz > 1800.);
    assert_eq!(e.config.bands, before.bands);
    for _ in 0..14 {
        let screen = e.screen(Page::Features, 40, 13);
        assert_eq!(screen.len(), 13);
        assert!(screen.iter().all(|s| s.chars().count() <= 40));
        e.key('n');
    }
}

#[test]
fn crossover_transactions_preserve_history_mutes_backpressure_and_fault_precedence() {
    use shr_pa::control::Handoff;
    use std::sync::atomic::Ordering;
    let mut c = config(Edge::lr24(800., false), true, 48000);
    c.input_delay_ms = 17.;
    c.bands[1].delay_ms = 9.;
    c.bands[1].eq[0].db = 8.;
    c.bands[1].eq[0].q = 15.;
    c.compressor.enabled = true;
    c.compressor.threshold_db = -35.;
    c.bands[1].limiter_db = -50.;
    let mut e = Engine::new(c).unwrap();
    let mut reference = Engine::new(c).unwrap();
    let mutes = [false, false, false, false, true, true];
    e.set_mutes(mutes);
    reference.set_mutes(mutes);
    let mut out = [[0.; 6]; 128];
    let mut expected = out;
    let signal = |b: usize| {
        std::array::from_fn::<_, 128, _>(|n| {
            let x = (2. * PI * 937. * (128 * b + n) as f64 / 48000.).sin() as f32 * 0.1;
            [x, x * 0.5]
        })
    };
    for b in 0..40 {
        let x = signal(b);
        e.render(&x, &mut out).unwrap();
        reference.render(&x, &mut expected).unwrap();
    }
    let slot = Handoff::default();
    let Crossover::Independent(ref mut pairs) = c.crossover else {
        unreachable!()
    };
    pairs[0].hp = Edge {
        bypass: false,
        hz: 1200.,
        family: Family::Butterworth,
        slope: 42,
    };
    let first = Prepared::new(c, false).unwrap();
    slot.publish(first).unwrap();
    let Crossover::Independent(ref mut pairs) = c.crossover else {
        unreachable!()
    };
    pairs[0].hp.bypass = true;
    let latest = Prepared::new(c, false).unwrap();
    assert!(slot.publish(latest).is_err());
    slot.service(&mut e);
    assert!(e.reconfiguring());
    slot.publish(latest).unwrap();
    let mut saw_silence = false;
    for b in 40..100 {
        slot.service(&mut e);
        let x = signal(b);
        e.render(&x, &mut out).unwrap();
        reference.render(&x, &mut expected).unwrap();
        saw_silence |= out.iter().all(|f| *f == [0.; 6]);
        assert!(out.iter().all(|f| f[4] == 0. && f[5] == 0.));
        if b >= 80 {
            assert!(!e.busy());
            // Includes compressor envelope, delay history, PEQ, untouched edge
            // history and limiter release. Mute/reconfigure may not reset these.
            for (y, r) in out.iter().zip(expected) {
                assert_eq!(&y[2..4], &r[2..4]);
            }
        }
    }
    assert!(saw_silence);
    assert_eq!(slot.accepted.load(Ordering::Acquire), 2);
    assert!(!slot.pending());
    // Fault wins over mode changes and recall, even with fresh unmute intent.
    e.render(&[[f32::NAN; 2]; 128], &mut out).unwrap();
    c.crossover = Crossover::LayoutLr24;
    slot.publish(Prepared::new(c, true).unwrap()).unwrap();
    slot.service(&mut e);
    e.set_mutes([false; 6]);
    e.render(&signal(101), &mut out).unwrap();
    assert_eq!(out, [[0.; 6]; 128]);
    assert_eq!(slot.rejected.load(Ordering::Acquire), 1);
}
