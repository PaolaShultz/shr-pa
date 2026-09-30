# M03 generator off/on and capture restoration — 2026-09-30

Started from clean `main` at `58f335741bd767fcb9574bc05e1813709fda2412`,
matching `origin/main` after fetch. Baseline CI run `36722518515` passed both
architectures. Read working agreements, current documentation, verification
records and actual generator, control, transport, UI and regression code.

## Implemented

- During `live --ui --signal=...`, `~` toggles the selected generator off/on.
  Off restores the two current mapped capture channels; on resumes the selected
  source. The 40×13 meter header shows desired `Gen ON`/`Gen OFF`, generator
  level target and key hints. Capture-only sessions reject the toggle.
- A separate atomic insertion target coalesces requests independently of gain
  and processing transactions. The audio thread services it once per block even
  while processing transitions are busy. No allocation, lock, queue, device
  reopen or processing-state reset is introduced.
- The source/capture mix ramps linearly over max(1, floor(rate/200)) samples:
  240 at 48 kHz. Rapid toggles retarget from the current mix; identical requests
  do not restart it. Endpoints copy their selected samples exactly. This bounds
  digital switching behavior; no analog transient quality is inferred.
- Phase, source clock, PRNG, pink history and gain ramps keep advancing while
  off. Re-enabling does not restart an impulse/sweep or reseed noise. Gain edits
  while off affect the generator only. The resulting source/capture blend feeds
  input meters and all six existing logical output chains.
- Off restores program audio rather than muting outputs. Capture can exceed
  the generator's peak bound, and processing tails remain. Existing mutes,
  limiter ceilings and fault precedence remain active. Shutdown ignores further
  control requests and retains the bounded mute cleanup.
- Recall retains insertion state and level. Neither enters Config, presets,
  library, EQ history or working recovery. Restart without `--signal` remains
  capture-only; UI startup remains muted.

Application `0.2.0-alpha.1`, processing schema v3, library/working envelopes v2,
logical 2×6 processing and 48 kHz / 128-frame / 512-frame defaults are unchanged.
No release or tag is created.

## Validation

Native aarch64 with pinned Rust 1.97.1. Focused generator/allocation checks ran
before the complete normal production suite.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | Pass: 58 production tests, zero failed/ignored |
| `cargo build --release --locked` | Pass |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Pass: lifecycle, navigation, resize and working recovery |
| `python3 scripts/check-live-controls.py target/release/shr-pa` | Pass: software-null toggles, levels while off, recall, capture-only restart/rejection and existing controls |
| `python3 scripts/check-docs.py` | Pass |
| Fresh-directory publication examples | Pass: version, three 40×13 snapshots, init/check, six-channel WAVs, source-preserving v2 migration and explicit null streaming |
| Final diff review and `git diff --check` | Pass |

Three new production regressions cover independent sample-by-sample crossfade
references with distinct stereo capture, rapid reversals, exact endpoints,
continuing source sequences, empty/short blocks, latest-target publication and
simultaneous independent level changes. Rates include 8/44.1/48/192 kHz plus a
synthetic one-sample ramp boundary. They exercise off during processing recall,
nonfinite capture fault propagation, fault persistence after re-enable, exact
signed-zero capture restoration and the distinction between silence and off.

The existing null test now observes capture restoration and resumed signal at
input meters and all six outputs, including four unmapped outputs. Processing
transaction counters remain zero. Startup mutes, limiter ceiling and shutdown
mutes still pass. Allocation counting now includes insertion toggles, level
ramps, capture blending and six-output DSP through two pink counter wraps:
zero allocations and deallocations.

The PTY test saves and recalls with the generator off, checks the retained level
and insertion state, then resumes it and restarts without a signal option. An
accidental spacing change to an existing compressor test expectation was corrected
after the first PTY run; the final complete PTY run passed.

## Limits and skipped classes

M03 remains partial: changing source type during a session is pending. RTA,
separate setup microphone and measurement workflows remain pending. Desired UI
state is not an audio acknowledgement; processing pending/busy excludes both
source ramps. Noise level remains a peak bound, not calibrated RMS or physical
output level. Physical response, latency, transients, protection and soak
performance remain unmeasured; no analog loopback is assumed.

Only explicit software-null PCM was opened. Physical hardware, JACK and host
audio settings were untouched. UMC1820 remains a future target. Hardware soaks,
exhaustive research, workload benchmarks, acoustic auditions and disposable
artwork renderers were intentionally skipped. The publication WAV examples used
fresh temporary paths and were removed after inspection. No historical default
test needed reclassification; on-demand commands remain in [VALIDATION](../VALIDATION.md).

## Publication

This record accompanies the implementation commit on the existing upstream
`main`. The final task report identifies the commit, verifies the remote revision
and reports both x86-64 and ARM64 CI results separately from local validation.
