//! Replay actual owner-produced display goldens with independent complex math.
use serde_json::Value;
use shr_pa::live_eq::EqSettings;
#[test]
fn actual_owner_normalized_banks_and_display_response_goldens() {
    for rate in [8000, 48000, 192000] {
        for profile in ["bypass", "bell", "low_shelf", "high_shelf"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/cpa/eq-v1/{rate}-{profile}.json"));
            let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(fixture["denominator"], "1+a1*z^-1+a2*z^-2");
            let settings: [EqSettings; 2] =
                serde_json::from_value(fixture["target"].clone()).unwrap();
            for (side, e) in settings.iter().enumerate() {
                let actual = e.coefficients(rate).unwrap();
                let expected: Vec<[f64; 5]> =
                    serde_json::from_value(fixture["target_coefficients"][side].clone()).unwrap();
                assert_eq!(actual.len(), 39);
                for (a, b) in actual.iter().zip(&expected) {
                    for (x, y) in a.iter().zip(b) {
                        assert!((x - y).abs() < 5e-15);
                    }
                }
                for response in fixture["target_response"][side].as_array().unwrap() {
                    let hz = response["hz"].as_f64().unwrap();
                    let w = std::f64::consts::TAU * hz / rate as f64;
                    let mut db = 0.;
                    let mut phase = 0.;
                    for [b0, b1, b2, a1, a2] in &actual {
                        let nr = b0 + b1 * w.cos() + b2 * (2. * w).cos();
                        let ni = -b1 * w.sin() - b2 * (2. * w).sin();
                        let dr = 1. + a1 * w.cos() + a2 * (2. * w).cos();
                        let di = -a1 * w.sin() - a2 * (2. * w).sin();
                        db += 10. * ((nr * nr + ni * ni) / (dr * dr + di * di)).log10();
                        phase += ni.atan2(nr) - di.atan2(dr);
                    }
                    let diff = phase - response["phase_radians"].as_f64().unwrap();
                    let wrapped = diff.sin().atan2(diff.cos());
                    assert!(
                        (db - response["magnitude_db"].as_f64().unwrap()).abs() < 1e-8,
                        "{rate} {profile} {hz}"
                    );
                    assert!(wrapped.abs() < 1e-8);
                }
            }
        }
    }
}
