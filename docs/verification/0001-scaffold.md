# Scaffold verification — 2026-09-29

> Historical evidence for the dated slice below. Test counts, pending features and
> commit/push statements describe that moment. See the [evidence index](README.md)
> for subsequent commits and the [0.2 alpha record](0007-0.2-alpha.md) for release checks.

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

A subsequent CI run exposed a startup resize race: SIGWINCH could arrive after
first paint but before the terminal event source was initialized. Event handling
now initializes before the first frame. The full normal local suite passed again,
plus ten immediate-resize repetitions. The existing resize case remains in CI.
