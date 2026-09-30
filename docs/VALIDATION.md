# Validation

## Normal production checks

```sh
python3 scripts/check-docs.py
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked
python3 scripts/check-terminal.py target/release/shr-pa
python3 scripts/check-live-controls.py target/release/shr-pa
```

The Rust suite covers complex crossover response/sum/phase, all layouts,
mono bass, delays, gain/polarity, PEQ response, limiter ceiling/release/linking,
mute ramps, meters, numerical faults, block boundaries and render allocations.
It also checks mapping, PCM conversion, partial transfers/error classification,
validated atomic presets, deterministic WAV output, editor controls, page bounds,
navigation, resize behavior and headless CLI errors. The Linux
pseudo-terminal check covers keyboard navigation, touch exit, Ctrl+C, termination
signals, small-terminal recovery and restoration of terminal attributes. It uses
no audio device. The Rust suite also explicitly uses ALSA's software-only `null`
PCM for the pre-start cancellation regression; it never selects hardware. These
are fast production regressions in the default validation path and CI.

Run the full normal suite for shared engine, render, model, routing, persistence,
concurrency or safety changes, and before publication. During implementation,
start with the focused checks for the changed behavior.

## Current coverage and future additions

Extend the existing DSP/schema/protection/allocation regressions as each new
module lands. Prepared handoff/backpressure, concurrent publication, response/dynamics,
transitions, migration and zero-allocation regressions now protect live controls.
The live-controls PTY script uses only software `null` PCM, exercises module edits,
save/recall, rejected rate changes and cleanup, and belongs to normal CI.
Library/working recovery, immutable templates, all 75 slots, explicit overwrites,
EQ restore, strict envelope validation, incompatible state, writer exclusion and
interrupted temporary files now have normal Rust regressions. Allocation tests
include curve/flatten/restore transactions. PTY checks also exercise working
restart, muted recovery, corrupt-state archival and live library/EQ commands.
D08 adds independent analog-pole magnitude/phase references for all BW/LR orders,
cutoff/stopband/bypass/extreme-rate checks, matched polarity sums, layout roles,
v2/envelope migration, retained history under mute/reconfigure and allocation-free
edge edits. The null PTY test edits both edges on all pairs, rejects split shortcuts
in independent mode and recovers the saved crossover after restart. See
[crossover verification](verification/0006-crossover.md).
M03 pink generation adds bounded stereo/block/rate checks, an independent
windowed spectral regression with a white-noise control, six-output WAV comparison,
explicit muted/unmuted software-null runs and generation allocation counting.
Generator level regressions cover finite/range validation, all source shapes,
block-independent scaling and exact default preservation, CLI rejection before
I/O, six-output levels/limiting, startup mutes and nondefault-level allocation counting.
Runtime level checks add ramp/retarget reference comparisons, exact block-partition
agreement, latest-target publication, zero-allocation gain edits, software-null
input/output level changes and 40×13 controls with capture-only rejection.
These short production tests run by default; they open no physical device.
Add analysis isolation and automatic device reconnection regressions with those
future features.

Use [function-map IDs](DRIVERACK_MAP.md) to link each future test/evidence record
to the planned capability. Include all fixed configuration families, inactive
outputs, single-mic isolation, full processing load, parameter/bypass transitions,
setup cancellation, presets/global-state ownership and local/remote command races.
Advanced matrix and eight-point measurement tests belong to future work.

## Opt-in bench and research classes

The initial direct ALSA command and opt-in control smoke test now exist:

```sh
# Explicit hardware use; inspect/select CARD_ID first.
./target/release/shr-pa live preset.json \
  hw:CARD=CARD_ID,DEV=0 hw:CARD=CARD_ID,DEV=0 \
  2 2 0,1 0,1,-,-,-,- 10 --unmute --signal=noise
python3 scripts/check-live.py target/release/shr-pa preset.json hw:CARD=CARD_ID,DEV=0
# Fully enabled offline workload with simultaneous live edits (opt-in):
cargo run --release --locked --example processing-load -- 30
# Explicit offline evidence renderer:
./target/release/shr-pa render preset.json sweep six-outputs.wav 5 --unmute
```

These are not run by `cargo test` or CI. The smoke script opens the specified
card, tests startup-muted capture, SIGTERM/reopen and terminal mute/exit behavior.
See [first-slice evidence](verification/0003-engine.md). No analog loopback is
connected, so analog latency cannot be inferred from these timings.

Hardware streaming, exhaustive parameter sweeps, long thermal/latency soaks,
feedback auditions and measurement renderers must be explicit opt-in commands.
Document each runnable command when its harness lands. Preserve results and
provenance; keep one-time research out of the normal suite unless it protects
current production behavior. Never represent absence of hardware tests as a pass.

Planned hardware record: board/OS/kernel/firmware, USB topology, device identity,
clock source, rates, native formats, channel map, period and buffer sizes,
processing configuration and software revision, temperature/throttling, xruns, maximum and percentile
render time, round-trip latency distribution and observed transients.

Initial proposed acceptance: repeated startup/recovery; then an eight-hour
full-load duplex soak with zero xruns, no throttling and the chosen processing
margin. Exercise UI and analysis load during the soak. Report the test duration
and actual workload with every result. A passing soak is evidence under those
conditions, not a guarantee of flawless operation in every venue.

## Release checks

Before publication, run all normal commands above, check `--version`, and exercise
[offline and software-null examples](RUNNING.md) with fresh temporary paths.
Verify six-channel WAV metadata, source-preserving migration and snapshot size.
Review changed artwork and run its generator when the diagram or snapshot changes.
The offline documentation checker validates local links/anchors, SVG XML and
Cargo application-version consistency; external links need a separate network
review. An HTTP access block is not evidence that a reference no longer exists.

Record results, skipped test classes and limitations in a dated verification
record. Review `git diff --check` and the final diff, commit, push to the existing
upstream and compare the remote branch revision. Report remote CI separately;
a successful push does not establish passing CI.

## Artwork

```sh
python3 scripts/render-docs.py target/release/shr-pa
```

Regenerates original SVG artwork and a terminal illustration from actual snapshot
text. This is an opt-in documentation renderer; it is not a runtime test.
