# M03 generator level — 2026-09-30

Started from clean `main` at `1f54f7dceb40e6772b98f9b936a6fdfcedb06768`,
matching `origin/main` after fetch. Baseline CI run `36716823038` passed on both
x86-64 and ARM64. Read working agreements, current documentation, verification
records and generator/command/transport code and tests before selecting this slice.

## Implemented

- `--level=DBFS` selects a finite −60…0 dBFS source peak bound for `render` and
  explicit live generators. Default remains −20 dBFS with identical samples.
- Applies to pink/white noise, sine, sweep and impulse; silence stays zero.
  Scaling precedes input metering and the complete six-output processing path.
  The bound is subject to f32 rounding; noise RMS and finite-record peaks are
  lower. No RMS or physical calibration is implied.
- A validated runtime-only level type prepares one constant scale before use.
  Generation adds one multiply per sample, preserves fixed storage, stereo pairing
  and sequence, and performs no allocations or deallocations.
- Duplicate/invalid levels reject. Explicit level on WAV input or capture-only
  live sessions rejects before output creation/device open. Presets, library and
  working recovery do not store source or level. Existing default API entry
  points, mutes, limiter, faults, transaction ownership and mappings remain.

This is level selection at command start. It does not add in-session level edits,
source switching or stop/restore-capture controls. M03 remains partial.
Application `0.2.0-alpha.1`, processing v3 and envelope v2 are unchanged.
Development defaults remain 48 kHz / 128-frame periods / 512-frame buffers.

## Validation

Native aarch64 with pinned `rustc 1.97.1 (8bab26f4f 2026-07-14)`:

| Check | Result |
| --- | --- |
| Focused generator, CLI, contract and allocation checks | Pass |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | Pass: 52 production tests, zero failed/ignored |
| `cargo build --release --locked` | Pass |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Pass: lifecycle, navigation, compact layout and recovery |
| `python3 scripts/check-live-controls.py target/release/shr-pa` | Pass: explicit −30 dBFS source, live transactions/library/EQ/crossover, muted recovery and cleanup |
| `python3 scripts/check-docs.py` | Pass: local links/anchors, SVG XML and version consistency |
| `git diff --check` and final diff review | Pass |

New bounded production regressions cover all six source shapes at −60, −37.5,
−20 and 0 dBFS, empty/short block fills and a pink counter wrap. They compare
scaling and stereo pairing, finite bounds, exact default behavior and independent
quarter-cycle sine peaks. CLI tests check invalid/nonfinite/out-of-range and
duplicate options, inapplicable source rejection, destination preservation and
unchanged preset bytes. Six-output WAV and software-null live tests at −60/−20/0
dBFS verify every logical output, including unmapped ones, startup mutes and the
existing −1 dBFS limiter ceiling. The WAV has a short final block. Pink generation
plus six-output DSP still counts zero allocations/deallocations at −7.5 dBFS.

The normal PTY test now starts its explicit noise source at −30 dBFS, exercises
saving/recalling processing, and restarts without source/level options. Recovery
remains muted. Existing spectral, crossover/phase, routing, schema/migration,
EQ-history, backpressure/retry, fault and recovery regressions also pass.

## Limits and skipped classes

Only software-null PCM was opened. No physical device, JACK state or host audio
setting changed. Null results establish software integration, not physical
response, latency, transients, protection or soak performance. UMC1820 remains
future work; no analog loopback is assumed.

Hardware soaks, exhaustive research, workload benchmarks, acoustic auditions and
disposable evidence/artwork renderers were intentionally skipped. No historical
one-time test needed reclassification; existing on-demand commands remain in
[VALIDATION](../VALIDATION.md). The workload example was compiled, not benchmarked.
RTA, setup microphone, measurement workflows and runtime generator controls remain
pending, along with the rest of the [complete inventory](../DRIVERACK_MAP.md).

## Publication

This record accompanies the implementation commit on the existing upstream
`main`. The final task report states the pushed revision, remote match and CI
results for x86-64 and ARM64 separately. No version bump, release or tag is needed.
