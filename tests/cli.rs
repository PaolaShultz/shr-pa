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
