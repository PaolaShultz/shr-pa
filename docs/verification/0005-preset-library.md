# Practical P3 preset and EQ state — 2026-09-30

Historical slice record. The later [D08 slice](0006-crossover.md) upgrades processing
to v3 and library/working envelopes to v2 with explicit migration of EQ histories.

Started from a clean checkout at `b955027` (verified with git status/log).
This slice is uncommitted and has not been pushed. The previously reported
GitHub Actions result belongs to the baseline; no new remote CI result is claimed.

## Working offline

- 75 user slots, six immutable generic layout templates, 1–24 character ASCII
  names, keyboard browsing/selection, named save/copy and explicit overwrite.
  Selection and copying do not change working processing or mutes.
- Independent atomic working checkpoints, including EQ restore points and the
  saved baseline used for modified status. Startup recovers compatible edits
  muted. Corrupt/incomplete/unknown-version/incompatible state is preserved and
  blocks working writes until explicitly archived with `:recover-reset`.
- Strict v1 library/working envelopes embed unchanged v2 processing JSON.
  Existing standalone import/export and explicit v1 processing migration remain.
  Runtime mute, generator, device and fault state are excluded.
- GEQ manual/flat plus original speech/warm/gentle curves. Both manual channel
  arrays survive curve audition. Input-channel and output-pair PEQ flatten/restore
  retain unrelated settings. Histories survive save, recall and working recovery.
- Compact 40×13 library view and command prompt with selection, active baseline,
  modified and recovery status; Escape cancels commands, Ctrl+C exits, and terminal
  attributes are restored. See [commands](../RUNNING.md) and
  [precise EQ/persistence semantics](../DSP.md#library-and-working-state).

## Working during live streaming

The same controller commands submit ordinary prepared Config snapshots through
the existing bounded handoff. Latest desired processing is retained for retry;
metadata and disk operations stay on the controller. Audible EQ changes use the
existing 20 ms transitions. Recall uses the mute/reconfigure procedure and leaves
all outputs muted until a fresh unmute. Faults retain precedence.

The software-only ALSA `null` PTY check exercised ongoing block progress, previous
module controls, library select/copy, occupied-slot rejection, explicit overwrite,
rapid curve commands, PEQ flatten/restore, rate rejection, recall, mute/unmute and
terminal cleanup. It checked the working snapshot after selection to establish
that selection/copy did not alter processing. A second explicitly started `null`
session omitted the signal option, recovered saved working processing, completed
its transaction and remained muted even with `--unmute --ui` supplied.

The library view retains live pending/busy/fault indicators and dynamics meters;
Tab returns to all six output meters and explicit physical mappings. `Active`
identifies the last saved/recalled preset baseline; pending/busy identifies processing
that has not settled. Null PCM proves integration, not USB timing or physical audio.

## Normal validation

Native aarch64, pinned Rust `1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked
python3 scripts/check-terminal.py target/release/shr-pa
python3 scripts/check-live-controls.py target/release/shr-pa
```

All passed. **39 production Rust tests**, no ignored tests: existing 33 plus six
library/recovery/EQ regressions. New coverage includes every user slot, immutable
and invalid slot IDs, writer exclusion, name validation, overwrite protection,
round-trip persistence, unknown/incomplete/incompatible recovery, invalid history,
ignored interrupted temporaries, explicit archival, selection/copy isolation,
restored EQ history, scoped state preservation, bounded backpressure retry and
fault precedence. Existing strict schema and explicit v1 migration tests passed.

Extended allocation/deallocation counting covers prepared curve, flatten and restore
operations as well as prior full-processing edits, recall and faults: **zero**.
An EQ flatten/restore regression compares unaffected output pairs sample-for-sample
against a continuing reference engine, including delay/filter history and mutes.
The normal terminal checks cover seven lifecycle/navigation/resize cases plus
working restart, startup mutes, restore history, prompt cancellation, corrupt-state
preservation and explicit archive/reset. Ctrl+C also works inside a command prompt.

No slow historical default tests needed reclassification. Hardware sessions,
soaks, exhaustive sweeps, workload benchmarks, disposable renderers and acoustic
auditions were intentionally skipped. Existing on-demand commands remain in
[VALIDATION](../VALIDATION.md). No new performance percentile is claimed.

## Hardware actually exercised

**None in this slice.** Only explicitly selected software null PCM was opened.
No physical capability assumptions, ALSA card numbers, host audio settings or
JACK state were changed. Earlier AudioBox evidence remains limited to the workload
and conditions recorded in [0003](0003-engine.md) and [0004](0004-live-controls.md).
No analog loopback is assumed.

## Remaining work and unmeasured behavior

Physical live preset/EQ edits, analog transients, power-cut durability, USB timing
under storage contention, long soaks, calibration and acoustic performance remain
unmeasured. Atomic sync/rename recovery has software regression coverage; no
physical power-loss experiment was run. The UI may wait for disk sync while the
independent audio worker continues. A failed checkpoint can lose newer edits on
restart; it reports Unsaved and never replaces a saved slot implicitly.

Automatic device reconnection, AutoEQ/automatic restore sources, setup microphone
and RTA, wizards, speaker/amplifier profiles, feedback suppression, subharmonics,
additional crossover/limiter modes, PA2 shelf units, preferences/lockouts, remote
writers, physical touch and full hardware acceptance remain pending. UMC1820
qualification remains future work. See the [complete map](../DRIVERACK_MAP.md).
