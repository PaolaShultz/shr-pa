# Scaffold verification — 2026-09-29

Executed locally on the Raspberry Pi 5 described in [hardware](../HARDWARE.md),
using Rust 1.97.1. Scope: offline scaffold, not audio performance.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | Pass: 3 layout/navigation unit tests, 2 CLI integration tests |
| `cargo build --release --locked` | Pass, native aarch64 |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Pass: 7 terminal lifecycle cases |
| Documentation illustrations | Generated; rendered preview visually inspected |

The terminal cases cover keyboard navigation, terminal mouse navigation, Ctrl+C,
SIGINT, SIGTERM, SIGHUP and resize recovery. Each checks clean exit, restored
terminal attributes, cursor visibility and mouse-capture release.

All normal scaffold tests ran. Hardware streaming/latency, long soaks, acoustic
measurement and DSP tests were not run: their implementations are pending and
the UMC1820 was not attached. No historical or exhaustive test suite exists yet.
The documentation renderer was run explicitly because the artwork was created
in this change.

This record does not claim physical touch, live audio, speaker protection or
low-buffer acceptance. GitHub CI results are reported by the repository badge.
