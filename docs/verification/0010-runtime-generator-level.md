# M03 runtime generator level — 2026-09-30

Started from clean `main` at `e89e25d0fb73b10caca7a974bc33d137a69e06aa`,
matching `origin/main` after fetch. Baseline CI run `36719270804` passed both
architectures. Read the working agreements, current documentation, verification
records and source/tests before choosing this bounded M03 continuation.

## Implemented

- During explicitly requested `live --ui --signal=...` sessions, `(` lowers and
  `)` raises the source peak target by 1 dB, clamped to −60…0 dBFS. The compact
  meter header shows the desired target and key hints; edits return to this view.
  Capture-only sessions reject the keys. Existing startup level/defaults remain.
- A separate atomic prepared-gain target retains the latest request without a
  queue, allocation or processing transaction. Validation and exponentiation stay
  on the controller. The audio thread reads once per block for an existing source.
- A linear amplitude ramp takes floor(rate/200) samples, at least one; 240 samples
  at 48 kHz. Rapid edits retarget from the current gain. Repeated identical targets
  do not restart the ramp. Sequence, pink history and stereo pairing are preserved.
- Source level precedes engine input meters and all six DSP output chains. Runtime
  mutes, limiter ceilings, processing transactions and fault handling retain their
  existing behavior. Shutdown stops accepting level edits during mute cleanup.
- Levels never enter Config, presets, EQ history or working recovery. Recall leaves
  the runtime target unchanged. Restart without `--signal` remains capture-only
  and muted in the UI.

Application `0.2.0-alpha.1`, processing schema v3, envelope v2 and development
48 kHz / 128-frame / 512-frame defaults remain unchanged. No release/tag is created.

## Validation

Native aarch64, pinned `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
Focused generator/allocation tests ran before the full normal checks.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | Pass: 55 production tests, zero failed/ignored |
| `cargo build --release --locked` | Pass |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Pass |
| `python3 scripts/check-live-controls.py target/release/shr-pa` | Pass: software-null controls, bounds, recall, capture-only rejection and recovery |
| `python3 scripts/check-docs.py` | Pass |
| Fresh-directory release CLI examples | Pass: version, 40×13 snapshots, six-channel WAV, source-preserving v2 migration and explicit null streaming |
| Final diff review and `git diff --check` | Pass |

Three new production tests cover the reference ramp for every source at
8/44.1/48/192 kHz, mid-ramp retargeting, exact block-partition agreement, latest
publication from another thread, exact return to the continuing default sequence,
and actual software-null level application before meters and all six outputs.
The null test observes muted −60 dBFS startup, requests 0 dBFS, unmutes, checks
all six outputs including four unmapped outputs, and checks the −1 dBFS limiter
ceiling and shutdown mutes. Existing CLI/WAV, spectral, schema, migration,
recovery, crossover/phase, processing concurrency and fault regressions remain.
Allocation counting now includes rapid generator gain publication/service and
ramps through two pink counter wraps: zero allocations/deallocations.

The 40×13 PTY test exercises both bounds, repeated keys, a target changed from
session startup, save/recall preserving that target, and fresh capture-only
recovery rejecting level edits. It caught a one-character clipped rejection
message during development; the shortened message fits the compact display.

## Limits and skipped classes

M03 remains partial. Runtime source switching/off with capture restoration,
RTA, setup microphone and measurement workflows remain pending. The UI shows
a desired target; processing pending/busy counters exclude this independent ramp.
Noise level is a peak bound, not calibrated RMS or physical output level. During
a downward ramp the previous bound can persist until settling. No analog
transition quality, latency, response, speaker protection or soak claim is made.

Only explicit software-null PCM was opened. Physical audio, JACK and host audio
settings were untouched. UMC1820 remains a future target. Hardware soaks,
exhaustive research, workload benchmarks, acoustic auditions and disposable
artwork renderers were intentionally skipped. Short publication example WAVs
used temporary paths and were removed after inspection. No historical default
test needed reclassification; on-demand commands remain in [VALIDATION](../VALIDATION.md).

## Publication

This record accompanies the implementation commit on the existing upstream
`main`. The final task report identifies the commit, verifies the remote revision
and reports both x86-64 and ARM64 CI outcomes separately from local validation.
