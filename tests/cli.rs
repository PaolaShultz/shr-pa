use std::process::{Command, Stdio};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_shr-pa"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn headless_commands_never_emit_terminal_control_sequences() {
    for args in [
        &["--help"][..],
        &["--version"],
        &["--snapshot"],
        &["--snapshot", "features"],
        &["--snapshot", "hardware"],
    ] {
        let output = run(args);
        assert!(output.status.success());
        assert!(!output.stdout.contains(&0x1b));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn unsupported_live_mode_and_invalid_pages_fail_explicitly() {
    for args in [
        &["--audio"][..],
        &["--snapshot", "live"],
        &["--version", "extra"],
        &[],
    ] {
        let output = run(args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn preset_render_commands_and_invalid_mapping_fail_before_hardware_open() {
    let dir = std::env::temp_dir().join(format!("shr-pa-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let preset = dir.join("preset.json");
    let wav = dir.join("out.wav");
    let p = preset.to_str().unwrap();
    let w = wav.to_str().unwrap();
    assert!(run(&["init", p]).status.success());
    assert!(run(&["check", p]).status.success());
    assert!(
        !run(&["render", p, "impulse", w, "0.03", "bad"])
            .status
            .success()
    );
    assert!(!wav.exists());
    assert!(
        run(&["render", p, "impulse", w, "0.03", "--unmute"])
            .status
            .success()
    );
    assert_eq!(hound::WavReader::open(&wav).unwrap().spec().channels, 6);
    let result = run(&[
        "live",
        p,
        "hw:NONEXISTENT",
        "hw:NONEXISTENT",
        "2",
        "2",
        "0,1",
        "0,1,2,3,4,5",
        "1",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("physical output unavailable"));
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&preset).unwrap()).unwrap();
    old["version"] = 2.into();
    old.as_object_mut().unwrap().remove("crossover");
    let bytes = serde_json::to_vec(&old).unwrap();
    std::fs::write(&preset, &bytes).unwrap();
    assert!(!run(&["check", p]).status.success());
    let migrated = dir.join("v3.json");
    let m = migrated.to_str().unwrap();
    assert!(run(&["migrate", p, m]).status.success());
    assert!(run(&["check", m]).status.success());
    assert_eq!(std::fs::read(&preset).unwrap(), bytes);
    assert!(!run(&["migrate", p, m]).status.success());
    let link = dir.join("dangling.json");
    std::os::unix::fs::symlink(dir.join("missing.json"), &link).unwrap();
    assert!(
        !run(&["migrate", p, link.to_str().unwrap()])
            .status
            .success()
    );
    assert!(link.is_symlink());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pink_cli_render_and_explicit_null_streaming_preserve_startup_mutes() {
    let dir = std::env::temp_dir().join(format!("shr-pa-pink-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let preset = dir.join("preset.json");
    let wav = dir.join("pink.wav");
    let p = preset.to_str().unwrap();
    assert!(run(&["init", p]).status.success());
    assert!(
        run(&[
            "render",
            p,
            "pink",
            wav.to_str().unwrap(),
            "0.031",
            "--unmute"
        ])
        .status
        .success()
    );
    let mut reader = hound::WavReader::open(&wav).unwrap();
    assert_eq!(reader.duration(), 1488);
    assert_eq!(reader.spec().channels, 6);
    // Compare the CLI artifact with direct generator -> full engine processing,
    // including a short last block, EQ, crossover, startup ramp and all outputs.
    let c = shr_pa::config::Config::load(&preset).unwrap();
    let mut engine = shr_pa::dsp::Engine::new(c).unwrap();
    engine.set_mutes([false; 6]);
    let mut g =
        shr_pa::offline::Generator::new(shr_pa::offline::Signal::Pink, 48000, 1488).unwrap();
    let mut input = vec![[0.; 2]; 1488];
    g.fill(&mut input);
    let mut expected = vec![[0.; 6]; 1488];
    for (input, output) in input.chunks(128).zip(expected.chunks_mut(128)) {
        engine.render(input, output).unwrap();
    }
    let actual: Vec<_> = reader.samples::<f32>().map(Result::unwrap).collect();
    assert_eq!(actual, expected.into_iter().flatten().collect::<Vec<_>>());
    for unmute in [false, true] {
        let mut args = vec![
            "live",
            p,
            "null",
            "null",
            "2",
            "2",
            "0,1",
            "0,1,-,-,-,-",
            "0.03",
            "--signal=pink",
        ];
        if unmute {
            args.push("--unmute");
        }
        let output = run(&args);
        assert!(output.status.success(), "{:?}", output);
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(report.contains("fault: None"), "{report}");
        let silent = report.contains("output_peaks: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]");
        assert_eq!(silent, !unmute, "{report}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explicit_generator_level_reaches_all_six_outputs_offline_and_live() {
    let dir = std::env::temp_dir().join(format!("shr-pa-level-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let preset = dir.join("preset.json");
    let wav = dir.join("level.wav");
    let p = preset.to_str().unwrap();
    let c = shr_pa::config::Config {
        layout: shr_pa::config::Layout::SixFullRange,
        ..Default::default()
    };
    c.save(&preset).unwrap();
    let bytes = std::fs::read(&preset).unwrap();
    for db in ["-60", "-20", "0"] {
        let option = format!("--level={db}");
        let result = run(&[
            "render",
            p,
            "sine:12000",
            wav.to_str().unwrap(),
            "0.031",
            &option,
            "--unmute",
        ]);
        assert!(result.status.success(), "{result:?}");
        let samples: Vec<_> = hound::WavReader::open(&wav)
            .unwrap()
            .samples::<f32>()
            .map(Result::unwrap)
            .collect();
        assert_eq!(samples.len(), 1488 * 6);
        let expected = (10_f64.powf(db.parse::<f64>().unwrap() / 20.) as f32)
            .min(10_f64.powf(-1. / 20.) as f32);
        for ch in 0..6 {
            let peak = samples
                .iter()
                .skip(ch)
                .step_by(6)
                .copied()
                .map(f32::abs)
                .fold(0., f32::max);
            assert!((peak - expected).abs() < 1e-6, "{db}: {peak} != {expected}");
        }
        for unmute in [false, true] {
            let mut args = vec![
                "live",
                p,
                "null",
                "null",
                "2",
                "2",
                "0,1",
                "0,1,-,-,-,-",
                "0.03",
                "--signal=sine:12000",
                &option,
            ];
            if unmute {
                args.push("--unmute");
            }
            let result = run(&args);
            assert!(result.status.success(), "{result:?}");
            let report = String::from_utf8(result.stdout).unwrap();
            assert!(report.contains("fault: None"), "{report}");
            let peaks = report
                .split("output_peaks: [")
                .nth(1)
                .unwrap()
                .split(']')
                .next()
                .unwrap();
            for peak in peaks.split(',') {
                let peak: f32 = peak.trim().parse().unwrap();
                assert!(
                    (peak - if unmute { expected } else { 0. }).abs() < 1e-6,
                    "{report}"
                );
            }
        }
    }
    assert_eq!(std::fs::read(&preset).unwrap(), bytes);
    assert!(!dir.join(".shr-pa").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bad_or_inapplicable_levels_reject_before_output_or_device_open() {
    let dir = std::env::temp_dir().join(format!("shr-pa-bad-level-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let preset = dir.join("preset.json");
    let wav = dir.join("preserved.wav");
    let p = preset.to_str().unwrap();
    shr_pa::config::Config::default().save(&preset).unwrap();
    std::fs::write(&wav, b"existing destination").unwrap();
    for option in [
        "--level=NaN",
        "--level=inf",
        "--level=-inf",
        "--level=-60.1",
        "--level=0.1",
        "--level=",
        "--level=bad",
    ] {
        assert!(
            !run(&[
                "render",
                p,
                "pink",
                wav.to_str().unwrap(),
                "0.03",
                "--unmute",
                option
            ])
            .status
            .success()
        );
        let result = run(&[
            "live",
            p,
            "hw:NONEXISTENT",
            "hw:NONEXISTENT",
            "2",
            "2",
            "0,1",
            "0,1,-,-,-,-",
            "0.03",
            "--signal=pink",
            option,
        ]);
        assert!(!result.status.success());
        assert!(
            !String::from_utf8_lossy(&result.stderr).contains("ALSA"),
            "{result:?}"
        );
    }
    let result = run(&[
        "live",
        p,
        "hw:NONEXISTENT",
        "hw:NONEXISTENT",
        "2",
        "2",
        "0,1",
        "0,1,-,-,-,-",
        "0.03",
        "--level=-20",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("requires --signal"));
    let result = run(&[
        "render",
        p,
        "missing.wav",
        wav.to_str().unwrap(),
        "0.03",
        "--unmute",
        "--level=-20",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("not a WAV path"));
    assert!(
        !run(&[
            "render",
            p,
            "pink",
            wav.to_str().unwrap(),
            "0.03",
            "--level=-20",
            "--level=-30"
        ])
        .status
        .success()
    );
    let result = run(&[
        "live",
        p,
        "hw:NONEXISTENT",
        "hw:NONEXISTENT",
        "2",
        "2",
        "0,1",
        "0,1,-,-,-,-",
        "0.03",
        "--signal=pink",
        "--level=-20",
        "--level=-30",
    ]);
    assert!(String::from_utf8_lossy(&result.stderr).contains("duplicate generator level"));
    assert_eq!(std::fs::read(&wav).unwrap(), b"existing destination");
    std::fs::remove_dir_all(dir).unwrap();
}
