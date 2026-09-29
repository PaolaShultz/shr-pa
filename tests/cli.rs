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
