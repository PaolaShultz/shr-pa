use shr_pa::offline::{Generator, GeneratorLevel, Signal};

#[test]
fn pink_is_bounded_stereo_and_independent_of_blocks_and_rate() {
    assert!(matches!(Signal::parse("pink").unwrap(), Signal::Pink));
    assert!(Signal::parse("pink:bad").is_err());
    assert!(Generator::new(Signal::Pink, 0, 1).is_err());
    assert!(Generator::new(Signal::Pink, 48000, 0).is_err());
    // Cross four row-counter wraps, including short and empty fills.
    let mut reference = vec![[0.; 2]; 262_147];
    Generator::new(Signal::Pink, 48000, reference.len() as u64)
        .unwrap()
        .fill(&mut reference);
    for rate in [8000, 44100, 48000, 96000, 192000] {
        let mut g = Generator::new(Signal::Pink, rate, reference.len() as u64).unwrap();
        let mut actual = vec![[0.; 2]; reference.len()];
        g.fill(&mut []);
        for chunk in actual.chunks_mut(127) {
            g.fill(chunk);
        }
        assert_eq!(reference, actual);
    }
    let mut power = 0.;
    let mut mean = 0.;
    for [l, r] in &reference {
        assert_eq!(l, r);
        assert!(l.is_finite() && l.abs() <= 0.1);
        power += (*l as f64).powi(2);
        mean += *l as f64;
    }
    let rms = (power / reference.len() as f64).sqrt();
    assert!((0.008..0.025).contains(&rms), "rms={rms}");
    // Finite pink sequences have a fluctuating mean; no DC-removal claim.
    assert!((mean / reference.len() as f64).abs() < 0.01);
}

// Independent Hann-windowed Goertzel measurements, using no production filters.
// Average eight bins in each octave over eight disjoint windows. Multiplying
// mean bin power by octave width tests equal power/octave, not equal power/Hz.
fn octave_powers(signal: Signal) -> Vec<f64> {
    const N: usize = 32768;
    let mut g = Generator::new(signal, 48000, (N * 8) as u64).unwrap();
    let mut samples = vec![[0.; 2]; N];
    let window: Vec<_> = (0..N)
        .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / N as f64).cos())
        .collect();
    let mut powers = vec![0.; 9];
    for _ in 0..8 {
        g.fill(&mut samples);
        for (octave, power) in powers.iter_mut().enumerate() {
            let low = 16 << octave;
            for probe in 0..8 {
                let bin = low + low * (2 * probe + 1) / 16;
                let coeff = 2. * (std::f64::consts::TAU * bin as f64 / N as f64).cos();
                let (mut a, mut b) = (0., 0.);
                for (frame, w) in samples.iter().zip(&window) {
                    let next = frame[0] as f64 * w + coeff * a - b;
                    b = a;
                    a = next;
                }
                *power += (a * a + b * b - coeff * a * b) * low as f64;
            }
        }
    }
    powers
}

#[test]
fn pink_has_approximately_equal_octave_power_and_white_does_not() {
    let pink = octave_powers(Signal::Pink);
    let min = pink.iter().copied().fold(f64::INFINITY, f64::min);
    let max = pink.iter().copied().fold(0., f64::max);
    let spread_db = 10. * (max / min).log10();
    assert!(spread_db < 3., "octave spread {spread_db:.3} dB: {pink:?}");
    // Control: equal-width white spectral density gives ~3 dB more per octave.
    let white = octave_powers(Signal::Noise);
    let rise_db = 10. * (white[8] / white[0]).log10();
    assert!((21. ..27.).contains(&rise_db), "white rise={rise_db}");
}

#[test]
fn generator_levels_validate_and_scale_all_sources_without_changing_sequence() {
    use shr_pa::offline::GeneratorLevel;
    for db in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -60.01, 0.01] {
        assert!(GeneratorLevel::new(db).is_err());
    }
    assert_eq!(GeneratorLevel::default().dbfs(), -20.);
    for signal in [
        Signal::Silence,
        Signal::Impulse,
        Signal::Sine(1000.),
        Signal::Sweep,
        Signal::Noise,
        Signal::Pink,
    ] {
        let mut reference = vec![[0.; 2]; 65539];
        Generator::new(signal, 48000, reference.len() as u64)
            .unwrap()
            .fill(&mut reference);
        for db in [-60., -37.5, -20., 0.] {
            let level = GeneratorLevel::new(db).unwrap();
            let mut g =
                Generator::with_level(signal, 48000, reference.len() as u64, level).unwrap();
            let mut actual = vec![[0.; 2]; reference.len()];
            g.fill(&mut []);
            for chunk in actual.chunks_mut(127) {
                g.fill(chunk);
            }
            let scale = 10_f64.powf((db + 20.) / 20.);
            let bound = 10_f64.powf(db / 20.) as f32;
            for (got, base) in actual.iter().zip(&reference) {
                assert_eq!(got[0], got[1]);
                assert!(got[0].is_finite() && got[0].abs() <= bound);
                let expected = (base[0] as f64 * scale) as f32;
                assert!((got[0] - expected).abs() <= bound * 2e-7);
                if db == -20. {
                    assert_eq!(got, base);
                }
            }
        }
    }
    // Peak definition has an independent exact reference: a quarter-cycle sine.
    for db in [-60., -20., 0.] {
        let mut frames = [[0.; 2]; 2];
        Generator::with_level(
            Signal::Sine(12000.),
            48000,
            2,
            GeneratorLevel::new(db).unwrap(),
        )
        .unwrap()
        .fill(&mut frames);
        assert_eq!(frames[1][0], 10_f64.powf(db / 20.) as f32);
    }
}

#[test]
fn runtime_level_ramps_retarget_without_restarting_source() {
    use shr_pa::control::GeneratorControl;
    for rate in [8000, 44100, 48000, 192000] {
        for signal in [
            Signal::Noise,
            Signal::Pink,
            Signal::Sine(1000.),
            Signal::Sweep,
            Signal::Impulse,
            Signal::Silence,
        ] {
            let control = GeneratorControl::default();
            let mut g = Generator::new(signal, rate, 100000).unwrap();
            let mut reference = Generator::new(signal, rate, 100000).unwrap();
            let mut current = 1.;
            // Retarget before the first ramp finishes, then settle at lower/default levels.
            for (db, frames) in [
                (0., rate as usize / 400),
                (-60., rate as usize / 200 + 3),
                (-20., rate as usize / 200 + 3),
            ] {
                control.request(GeneratorLevel::new(db).unwrap());
                let target = 10_f64.powf((db + 20.) / 20.);
                let ramp = (rate / 200).max(1) as usize;
                let start = current;
                for i in 0..frames {
                    // Repeated service must not restart the ramp.
                    control.service(&mut g);
                    let mut actual = [[0.; 2]; 1];
                    let mut base = [[0.; 2]; 1];
                    g.fill(&mut actual);
                    reference.fill(&mut base);
                    current = if i + 1 >= ramp {
                        target
                    } else {
                        start + (target - start) * (i + 1) as f64 / ramp as f64
                    };
                    assert_eq!(actual[0][0], actual[0][1]);
                    assert!((actual[0][0] as f64 - base[0][0] as f64 * current).abs() < 1e-7);
                    assert!(actual[0][0].abs() <= 1.);
                }
            }
        }
    }
}

#[test]
fn runtime_latest_level_and_block_partition_are_deterministic() {
    use shr_pa::control::GeneratorControl;
    let control = GeneratorControl::default();
    let mut whole = Generator::new(Signal::Pink, 48000, 100000).unwrap();
    let mut split = Generator::new(Signal::Pink, 48000, 100000).unwrap();
    for db in [-60., 0., -37.5, -20.] {
        // Multiple publications before consumption retain the last desired value.
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    control.request(GeneratorLevel::new(-10.).unwrap());
                    control.request(GeneratorLevel::new(db).unwrap());
                })
                .join()
                .unwrap();
        });
        control.service(&mut whole);
        let mut expected = [[0.; 2]; 1024];
        whole.fill(&mut expected);
        let mut actual = [[0.; 2]; 1024];
        for block in actual.chunks_mut(73) {
            control.service(&mut split);
            split.fill(&mut []);
            split.fill(block);
        }
        assert_eq!(actual, expected);
    }
    // Returning to default is exact once the ramp ends, at the continued sequence.
    let mut reference = Generator::new(Signal::Pink, 48000, 100000).unwrap();
    reference.fill(&mut [[0.; 2]; 4096]);
    let mut expected = [[0.; 2]; 128];
    let mut actual = expected;
    reference.fill(&mut expected);
    whole.fill(&mut actual);
    assert_eq!(actual, expected);
}

#[test]
fn live_null_applies_runtime_level_before_meters_and_all_six_outputs() {
    use shr_pa::{
        config::{Config, Layout},
        transport::{self, LiveOptions, Mapping, Shared},
    };
    use std::{
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    let shared = Shared::default();
    let c = Config {
        layout: Layout::SixFullRange,
        ..Config::default()
    };
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            transport::run(
                c,
                LiveOptions {
                    capture: "null",
                    playback: "null",
                    map: Mapping::parse(2, 2, "0,1", "0,1,-,-,-,-").unwrap(),
                    seconds: 86400.,
                    signal: Some(Signal::Sine(12000.)),
                    generator_level: Some(GeneratorLevel::new(-60.).unwrap()),
                },
                &shared,
            )
            .unwrap()
        });
        let deadline = Instant::now() + Duration::from_secs(8);
        while shared.blocks.load(Ordering::Relaxed) < 10 && Instant::now() < deadline {
            std::thread::yield_now();
        }
        let started = shared.blocks.load(Ordering::Relaxed) >= 10;
        let initial = f32::from_bits(shared.input[0].load(Ordering::Relaxed));
        let muted = shared.output.iter().all(|x| x.load(Ordering::Relaxed) == 0);
        shared.generator.request(GeneratorLevel::new(0.).unwrap());
        shared.mutes.store(0, Ordering::Relaxed);
        let mut reached = false;
        while Instant::now() < deadline {
            reached = f32::from_bits(shared.input[0].load(Ordering::Relaxed)) > 0.99
                && shared
                    .output
                    .iter()
                    .all(|x| f32::from_bits(x.load(Ordering::Relaxed)) > 0.8);
            if reached {
                break;
            }
            std::thread::yield_now();
        }
        let mut restored = false;
        let mut resumed = false;
        shared.generator.request_enabled(false);
        while Instant::now() < deadline {
            restored = shared
                .input
                .iter()
                .chain(shared.output.iter())
                .all(|x| f32::from_bits(x.load(Ordering::Relaxed)) == 0.);
            if restored {
                break;
            }
            std::thread::yield_now();
        }
        shared.generator.request_enabled(true);
        while Instant::now() < deadline {
            resumed = f32::from_bits(shared.input[0].load(Ordering::Relaxed)) > 0.99
                && shared
                    .output
                    .iter()
                    .all(|x| f32::from_bits(x.load(Ordering::Relaxed)) > 0.8);
            if resumed {
                break;
            }
            std::thread::yield_now();
        }
        shared.stop.store(true, Ordering::Relaxed);
        let report = worker.join().unwrap();
        assert!(started && muted && (initial - 0.001).abs() < 1e-7);
        assert!(reached, "runtime level did not reach every logical output");
        assert!(
            restored && resumed,
            "capture restoration or source resume failed"
        );
        assert_eq!(shared.controls.accepted.load(Ordering::Relaxed), 0);
        assert_eq!(shared.controls.rejected.load(Ordering::Relaxed), 0);
        assert!(report.fault.is_none());
        assert!(
            report
                .output_peaks
                .iter()
                .all(|&x| x <= 10_f32.powf(-1. / 20.) + 1e-6)
        );
        assert_eq!(shared.mutes.load(Ordering::Relaxed), 63);
    });
}

#[test]
fn capture_restore_ramps_retarget_and_preserve_source_clock() {
    use shr_pa::{control::GeneratorControl, offline::LiveGenerator};
    for rate in [1, 8000, 44100, 48000, 192000] {
        let mut signals = vec![
            Signal::Noise,
            Signal::Pink,
            Signal::Silence,
            Signal::Impulse,
        ];
        if rate > 1 {
            signals.extend([Signal::Sine(1000.), Signal::Sweep]);
        }
        for signal in signals {
            let control = GeneratorControl::default();
            let mut live = LiveGenerator::new(Generator::new(signal, rate, 100000).unwrap());
            let mut reference = Generator::new(signal, rate, 100000).unwrap();
            let ramp = (rate / 200).max(1) as usize;
            let mut current = 1.;
            let mut position = 0;
            for (enabled, frames) in [
                (true, 17),
                (false, ramp / 2),
                (true, ramp / 3),
                (false, ramp + 23),
                (true, ramp + 23),
            ] {
                control.request_enabled(enabled);
                let target = if enabled { 1. } else { 0. };
                let start = current;
                for i in 0..frames {
                    // Repeated requests/service must not restart the transition.
                    control.request_enabled(enabled);
                    control.service_live(&mut live);
                    let capture = [0.25 + (position % 11) as f32 / 100., -0.375];
                    let mut actual = [capture];
                    let mut source = [[0.; 2]];
                    live.mix_capture(&mut actual);
                    reference.fill(&mut source);
                    current = if i + 1 >= ramp {
                        target
                    } else {
                        start + (target - start) * (i + 1) as f64 / ramp as f64
                    };
                    for (ch, captured) in capture.into_iter().enumerate() {
                        let expected = ((1. - current) * captured as f64
                            + current * source[0][ch] as f64)
                            as f32;
                        assert!((actual[0][ch] - expected).abs() < 1e-7);
                        if current == 0. {
                            assert_eq!(actual[0][ch].to_bits(), captured.to_bits());
                        } else if current == 1. {
                            assert_eq!(actual[0][ch], source[0][ch]);
                        }
                    }
                    position += 1;
                }
            }
        }
    }
}

#[test]
fn capture_selection_coalesces_and_is_block_independent_with_level_edits() {
    use shr_pa::{control::GeneratorControl, offline::LiveGenerator};
    let control = GeneratorControl::default();
    let mut whole = LiveGenerator::new(Generator::new(Signal::Pink, 48000, 100000).unwrap());
    let mut split = LiveGenerator::new(Generator::new(Signal::Pink, 48000, 100000).unwrap());
    for (enabled, db, count) in [
        (false, -60., 71),
        (true, 0., 53),
        (false, -37., 1024),
        (true, -20., 1024),
    ] {
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    control.request_enabled(!enabled);
                    control.request_enabled(enabled);
                    control.request(GeneratorLevel::new(db).unwrap());
                })
                .join()
                .unwrap();
        });
        let capture: Vec<_> = (0..count).map(|i| [i as f32 / 2048., -0.25]).collect();
        let mut expected = capture.clone();
        let mut actual = capture;
        control.service_live(&mut whole);
        whole.mix_capture(&mut expected);
        for block in actual.chunks_mut(37) {
            control.service_live(&mut split);
            split.mix_capture(&mut []);
            split.mix_capture(block);
        }
        assert_eq!(actual, expected);
    }
}

#[test]
fn capture_restore_services_during_processing_and_cannot_clear_faults() {
    use shr_pa::{
        config::Config,
        control::{GeneratorControl, Handoff},
        dsp::{Engine, Prepared},
        offline::LiveGenerator,
    };
    let control = GeneratorControl::default();
    let mut source = LiveGenerator::new(Generator::new(Signal::Pink, 48000, 100000).unwrap());
    let mut engine = Engine::new(Config::default()).unwrap();
    let handoff = Handoff::default();
    handoff
        .publish(Prepared::new(Config::default(), true).unwrap())
        .unwrap();
    handoff.service(&mut engine);
    assert!(engine.busy());
    control.request_enabled(false);
    let mut capture = [[0.25, -0.5]; 128];
    let mut output = [[0.; 6]; 128];
    for _ in 0..2 {
        capture.fill([0.25, -0.5]);
        control.service_live(&mut source);
        source.mix_capture(&mut capture);
        engine.render(&capture, &mut output).unwrap();
    }
    assert!(engine.busy());
    assert_eq!(capture[127], [0.25, -0.5]);
    // A selected capture fault must reach the engine; toggling back cannot reset it.
    capture.fill([f32::NAN, 0.]);
    source.mix_capture(&mut capture);
    engine.render(&capture, &mut output).unwrap();
    assert!(engine.faulted());
    control.request_enabled(true);
    control.service_live(&mut source);
    capture.fill([0.; 2]);
    source.mix_capture(&mut capture);
    engine.set_mutes([false; 6]);
    engine.render(&capture, &mut output).unwrap();
    assert_eq!(output, [[0.; 6]; 128]);

    // Fully selected generation still replaces capture exactly, even nonfinite capture.
    let mut source = LiveGenerator::new(Generator::new(Signal::Silence, 48000, 1000).unwrap());
    let mut capture = [[f32::NAN, f32::INFINITY]; 1];
    source.mix_capture(&mut capture);
    assert_eq!(capture, [[0.; 2]]);
    // Silence is a source; off must restore capture rather than produce silence.
    control.request_enabled(false);
    control.service_live(&mut source);
    let mut capture = [[-0.0, 0.5]; 240];
    source.mix_capture(&mut capture);
    assert_eq!(capture[239][0].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(capture[239][1], 0.5);
}
