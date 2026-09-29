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
    std::fs::remove_dir_all(dir).unwrap();
}
