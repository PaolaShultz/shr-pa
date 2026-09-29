use shr_pa::{
    config::{Config, EqBand, InputMode, Layout},
    dsp::Engine,
};
use std::f64::consts::PI;
fn process(c: Config, input: &[[f32; 2]], block: usize) -> Vec<[f32; 6]> {
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    let mut out = vec![[0.; 6]; input.len()];
    for (i, o) in input.chunks(block).zip(out.chunks_mut(block)) {
        e.render(i, o).unwrap();
    }
    out
}
fn full() -> Config {
    Config {
        layout: Layout::SixFullRange,
        ..Config::default()
    }
}
#[derive(Clone, Copy, Debug)]
struct C(f64, f64);
impl C {
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
    }
    fn scale(self, x: f64) -> Self {
        Self(self.0 * x, self.1 * x)
    }
    fn div(self, b: Self) -> Self {
        self.mul(Self(b.0, -b.1))
            .scale(1. / (b.0 * b.0 + b.1 * b.1))
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
}
// Independent reference: analog Butterworth prototypes evaluated using the
// bilinear frequency transform, rather than copying digital biquad coefficients.
fn lr(hz: f64, cut: f64, rate: f64) -> (C, C, C) {
    let r = (PI * hz / rate).tan() / (PI * cut / rate).tan();
    let d = C(1. - r * r, 2_f64.sqrt() * r);
    let lp = C(1., 0.).div(d);
    let hp = C(-r * r, 0.).div(d);
    (lp.mul(lp), hp.mul(hp), C(d.0, -d.1).div(d))
}
#[test]
fn complete_crossover_complex_response_matches_independent_reference() {
    for rate in [44100, 48000, 96000] {
        for layout in [Layout::TwoWay, Layout::ThreeWay] {
            let c = Config {
                sample_rate: rate,
                layout,
                low_hz: 180.,
                high_hz: 2400.,
                ..Config::default()
            };
            let mut input = vec![[0.; 2]; 16384];
            input[512] = [0.1, 0.];
            let out = process(c, &input, 128);
            for hz in [30., 90., 180., 360., 1000., 2400., 4800., 10000., 16000.] {
                let (lo, rest, ap1) = lr(hz, c.low_hz, rate as f64);
                let (mid, hi, ap2) = lr(hz, c.high_hz, rate as f64);
                let expected = if layout == Layout::ThreeWay {
                    [rest.mul(hi), rest.mul(mid), lo.mul(ap2)]
                } else {
                    [rest, C(0., 0.), lo]
                };
                let mut actual = [C(0., 0.); 3];
                for (n, frame) in out[512..].iter().enumerate() {
                    let a = -2. * PI * hz * n as f64 / rate as f64;
                    for band in 0..3 {
                        actual[band] = actual[band]
                            .add(C(a.cos(), a.sin()).scale(frame[band * 2] as f64 / 0.1));
                        assert_eq!(frame[band * 2 + 1], 0.);
                    }
                }
                for band in 0..3 {
                    assert!(
                        actual[band].add(expected[band].scale(-1.)).abs() < 3e-5,
                        "{rate} {layout:?} {hz} band {band}: {:?} vs {:?}",
                        actual[band],
                        expected[band]
                    );
                }
                let sum = actual[0].add(actual[1]).add(actual[2]);
                let reference = if layout == Layout::ThreeWay {
                    ap1.mul(ap2)
                } else {
                    ap1
                };
                assert!((sum.abs() - 1.).abs() < 3e-5);
                assert!(sum.add(reference.scale(-1.)).abs() < 3e-5);
            }
        }
    }
}
#[test]
fn routing_layouts_mono_selection_and_bass_average() {
    for layout in [
        Layout::FullRange,
        Layout::External,
        Layout::SixFullRange,
        Layout::FourPlusSubs,
        Layout::TwoWay,
        Layout::ThreeWay,
    ] {
        let c = Config {
            layout,
            ..Config::default()
        };
        let input = vec![[0.1, -0.1]; 4096];
        let out = process(c, &input, 127);
        let last = out.last().unwrap();
        if matches!(layout, Layout::FullRange | Layout::External) {
            assert_eq!(&last[2..], &[0.; 4]);
        }
        if layout == Layout::TwoWay {
            assert_eq!(&last[2..4], &[0.; 2]);
        }
        for pair in last.chunks(2) {
            assert!((pair[0] + pair[1]).abs() < 1e-7);
        }
        if matches!(layout, Layout::FourPlusSubs) {
            assert_eq!(last[0], last[2]);
            assert_eq!(last[1], last[3]);
        }
        let mono = process(
            Config {
                input_mode: InputMode::MonoLeft,
                ..c
            },
            &input,
            128,
        );
        for frame in mono {
            for pair in frame.chunks(2) {
                assert_eq!(pair[0], pair[1]);
            }
        }
        if matches!(
            layout,
            Layout::TwoWay | Layout::ThreeWay | Layout::FourPlusSubs
        ) {
            let cancel = process(
                Config {
                    mono_bass: true,
                    ..c
                },
                &input,
                128,
            );
            assert!(cancel.iter().all(|f| f[4] == 0. && f[5] == 0.));
            let correlated = process(
                Config {
                    mono_bass: true,
                    ..c
                },
                &vec![[0.1, 0.1]; 4096],
                128,
            );
            let stereo = process(c, &vec![[0.1, 0.1]; 4096], 128);
            assert_eq!(correlated, stereo);
        }
    }
}
#[test]
fn delays_gain_polarity_and_block_boundaries() {
    let mut c = full();
    c.input_delay_ms = 13.;
    c.bands[0].delay_ms = 3.;
    c.bands[1].delay_ms = 7.;
    c.bands[1].inverted = true;
    c.bands[2].gain_db = -6.;
    let mut input = vec![[0.; 2]; 4000];
    input[512] = [0.1, 0.2];
    let a = process(c, &input, 128);
    let b = process(c, &input, 7);
    assert_eq!(a, b);
    assert_eq!(a[512 + 768][0], 0.1);
    assert_eq!(a[512 + 960][2], -0.1);
    assert!((a[512 + 624][4] - 0.1 * 10_f32.powf(-6. / 20.)).abs() < 1e-7);
    assert_eq!(a.iter().filter(|f| f[0] != 0.).count(), 1);
}
#[test]
fn peq_center_gain_input_and_output_are_independent() {
    let mut c = full();
    c.input_eq[0][0] = EqBand {
        kind: shr_pa::config::EqKind::Bell,
        slope: 1.,
        hz: 1000.,
        q: 2.,
        db: 6.,
    };
    c.bands[1].eq[0] = EqBand {
        kind: shr_pa::config::EqKind::Bell,
        slope: 1.,
        hz: 1000.,
        q: 1.,
        db: -6.,
    };
    let input: Vec<_> = (0..12000)
        .map(|n| {
            let x = (2. * PI * 1000. * n as f64 / 48000.).sin() as f32 * 0.01;
            [x, x]
        })
        .collect();
    let out = process(c, &input, 128);
    let rms = |ch: usize| {
        (out[6000..]
            .iter()
            .map(|f| (f[ch] as f64).powi(2))
            .sum::<f64>()
            / 6000.)
            .sqrt()
    };
    assert!((20. * (rms(0) / rms(1)).log10() - 6.).abs() < 0.001);
    assert!((20. * (rms(2) / rms(0)).log10() + 6.).abs() < 0.001);
}
#[test]
fn limiter_has_zero_sample_overshoot_linked_gain_and_exponential_release() {
    let mut c = full();
    c.bands[0].limiter_db = -12.;
    c.bands[0].release_ms = 100.;
    let mut input = vec![[0.; 2]; 12000];
    for f in &mut input[512..1024] {
        *f = [4., 1.];
    }
    for f in &mut input[1024..] {
        *f = [0.1, 0.025];
    }
    let out = process(c, &input, 128);
    let threshold = 10_f64.powf(-12. / 20.);
    assert!(out.iter().all(|f| f[0].abs() <= threshold as f32 + 1e-7));
    for f in &out[512..] {
        assert!((f[0] - 4. * f[1]).abs() < 1e-7);
    }
    let expected = 1. - (1. - threshold / 4.) * (-4800_f64 / 4800.).exp();
    assert!((out[1024 + 4800 - 1][0] as f64 / 0.1 - expected).abs() < 1e-6);
}
#[test]
fn startup_mutes_ramps_meters_and_faults_fail_closed() {
    let mut e = Engine::new(full()).unwrap();
    let input = [[0.1, 0.2]; 128];
    let mut out = [[0.; 6]; 128];
    e.render(&input, &mut out).unwrap();
    assert!(out.iter().all(|f| *f == [0.; 6]));
    assert_eq!(e.meters.input_peak, [0.1, 0.2]);
    e.set_mutes([false; 6]);
    e.render(&input, &mut out).unwrap();
    assert!(
        out.windows(2)
            .all(|w| w[1][0] >= w[0][0] && w[1][0] - w[0][0] <= 0.1 / 240. + 1e-7)
    );
    e.render(&input, &mut out).unwrap();
    assert_eq!(out[127][0], 0.1);
    e.set_mutes([true; 6]);
    e.render(&input, &mut out).unwrap();
    e.render(&input, &mut out).unwrap();
    assert_eq!(out[127], [0.; 6]);
    let mut bad = input;
    bad[100][1] = f32::NAN;
    e.render(&bad, &mut out).unwrap();
    assert!(e.faulted());
    assert_eq!(out, [[0.; 6]; 128]);
    e.set_mutes([false; 6]);
    e.render(&input, &mut out).unwrap();
    assert_eq!(out, [[0.; 6]; 128]);
    let mut wrong = [[1.; 6]; 129];
    assert!(e.render(&input, &mut wrong).is_err());
    assert_eq!(wrong, [[0.; 6]; 129]);
}
#[test]
fn parameter_boundaries_reject_nonfinite_and_impossible_configs() {
    let mut c = Config {
        low_hz: f64::NAN,
        ..Config::default()
    };
    assert!(c.validate().is_err());
    c = Config::default();
    c.high_hz = c.low_hz;
    assert!(c.validate().is_err());
    c = Config::default();
    c.bands[2].delay_ms = 10.1;
    assert!(c.validate().is_err());
    c = Config::default();
    c.input_eq[1][7].q = 0.;
    assert!(c.validate().is_err());
    c = Config::default();
    c.max_block = 0;
    assert!(Engine::new(c).is_err());
    for rate in [8000, 44100, 48000, 96000, 192000] {
        let mut c = Config {
            sample_rate: rate,
            low_hz: 16.,
            high_hz: (rate as f64 * 0.45).min(20000.),
            input_delay_ms: 100.,
            ..Config::default()
        };
        for b in &mut c.bands {
            b.delay_ms = 10.;
            b.gain_db = 20.;
            b.limiter_db = -60.;
            b.eq = [EqBand {
                kind: shr_pa::config::EqKind::Bell,
                slope: 1.,
                hz: 20.,
                q: 15.909,
                db: 12.,
            }; 8];
        }
        let out = process(c, &vec![[0.1, -0.1]; 4096], 128);
        assert!(
            out.iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 0.001001)
        );
    }
}
