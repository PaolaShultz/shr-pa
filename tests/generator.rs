use shr_pa::offline::{Generator, Signal};

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
