# Live transactions and P3 core controls — 2026-09-29

Implemented over the existing uncommitted engine/ALSA slice based on `488c682`.
Prior work was preserved; nothing was committed or pushed. This record supplements
[0003](0003-engine.md), whose physical trials used the earlier processing workload.

## What works offline

- Prepared, validated, coherent control transactions at block boundaries; one
  atomic slot, explicit busy rejection, one latest editor snapshot for retry.
  Existing filter, delay and dynamics state survives unrelated edits.
- 20 ms gain/polarity ramps, old/new biquad output crossfades, dual-tap delay
  crossfades and limiter-setting ramps. Reconfiguration mutes before installation,
  holds through transition, then resumes runtime mute targets. Faults override recall.
- 31-band GEQ with linked/separate settings; eight input and eight paired output
  PEQ bells/low shelves/high shelves with bypass; stereo-linked compressor with
  attack/release, ratio, threshold, knee, makeup and bypass transitions.
- Existing LR24 summation/phase, six logical outputs, mono rules, limiter protection,
  mutes and WAV processing remain covered. A linked ceiling guard after the delay
  protects old buffered samples when the limiter threshold is lowered.
- Strict v2 presets, atomic save/load and explicit `migrate OLD NEW` for v1.
  Old bell settings retain their meanings; new GEQ/compression start bypassed.

## What works during streaming

The direct ALSA loop services prepared transactions before each render block.
The compact terminal edits every implemented module parameter, selects input
channel/output pair/filter band, shows desired values, modified/pending/busy state,
input/output peaks, clip bits, compression/limiting reduction and faults. Refresh
is capped at 10 Hz, including bursts of keys. Saving/loading stays on the controller.

The software-only `null` PCM PTY regression exercised streaming while editing
pair gain, GEQ enable/band gain, PEQ frequency and shelf type, compressor enable/
threshold, limiter release, input delay and crossover split frequency. It also exercised atomic save,
preset recall with muted outputs, rejected rate-changing recall, layout changes,
unmute/mute and quit with restored terminal attributes. It asserts actual audio
block progress and transaction acceptance. `null` has no physical clock/USB deadline;
this demonstrates integration, not hardware latency or xrun performance.

One terminal bug found by this check was fixed: a pending submission could replace
a recall rejection message before painting it. Submission status now uses the
separate counters/pending/busy fields; action errors remain visible.

## Normal validation

All commands in [VALIDATION](../VALIDATION.md) passed with Rust 1.97.1:

- `cargo fmt --all -- --check` and strict Clippy on all targets.
- **33 production Rust tests passed**, no ignored tests: 3 UI layout, 3 CLI,
  8 contracts, 7 original engine, 10 live-processing, 2 render-allocation tests.
- Native release build, including the opt-in workload example.
- Seven offline PTY lifecycle cases: keyboard, mouse, Ctrl+C, SIGINT, SIGTERM,
  SIGHUP and resize/recovery.
- Software-null live-controls PTY case above; added to normal ARM64/x86-64 CI.

Independent DSP checks include warped analog complex shelf references at both
slope bounds, GEQ gain/link/separation/bypass and unavailable-band policy,
compressor static transfer, knee midpoint, makeup, linked gain and exact attack/
release time-constant values. Transition checks compare independent old/new
filter responses, exact two-tap phase-cancelling delay fades and gain/polarity
ramps. Unrelated output filter/delay state is checked sample-for-sample. Tests
also cover reconfiguration mute sequencing, rate rejection, strict schema bounds,
explicit legacy migration, 100 concurrent handoff publications, full-slot rejection
and fault precedence. Original complex three-way summation tests remain unchanged
apart from explicit new bell schema fields and Copy configuration use.

The allocator regression counts allocation, reallocation **and deallocation**
through handoff publication/service, fully enabled processing, filter/delay/
limiter/polarity transitions, recall, mutes and fault rejection: **zero**.

No slow historical default tests needed reclassification. Hardware soaks,
exhaustive sweeps, disposable WAV/artwork rendering, acoustic auditions and
physical unplug/power tests were intentionally skipped. The timing example below
is opt-in; normal tests build it but do not run its workload.

## Fully enabled offline workload

Command:

```sh
cargo run --release --locked --example processing-load -- 30
```

Pi 5 Model B Rev 1.1, aarch64, Debian kernel
`6.18.34+rpt-rpi-2712` PREEMPT, Rust 1.97.1 native release. No scheduler, affinity,
service or host audio changes. Post-validation observation: 60.4 °C,
`get_throttled=0x0`; this is not a thermal soak.

48 kHz / 128 frames, all six outputs, LR24 three-way compensation, all **126 EQ
sections** enabled (62 GEQ + 16 input PEQ + 48 output PEQ, including shelves),
compressor enabled, maximum-capacity input/output delays and all limiters.
Seeded noise at −20 dBFS. Input EQ frequencies use every fourth GEQ center;
output EQ frequencies are `80 * 2^index`; gains alternate −1/+1 dB.
GEQ gains alternate −1/+1 dB. Compressor threshold is −30 dBFS.

Every 16 blocks a transaction alternates all EQ gain signs, pair gain 0/−3 dB,
limiter ceiling −1/−10 dBFS, input delay 100/30 ms, pair delays 10/3 ms and
compressor ratio 4/8. There are 18 crossover/all-pass sections in addition to EQ.
At most two filter evaluations per changed section and two reads per changed
delay are active; no whole-engine duplication or audio-stage queue is used.

| Program audio | Blocks | Transactions | Mean | p99 | Maximum |
| --- | --- | --- | --- | --- | --- |
| 30 seconds | 11,250 | 704 | 215.77 µs | 330.04 µs | 355.46 µs |

Timer includes transaction application plus render, excludes source generation,
preparation and recording timings. Observed maximum is 13.3% of the 2,666.7 µs
period. It is an offline observation under ordinary scheduling, not a hard
worst-case bound, physical xrun result or analog latency measurement.

## Hardware exercised this turn

Read the attached device's advertised stereo S32_LE capabilities and running
stream status. JACK owned both AudioBox endpoints at 48 kHz. One explicit
startup-muted 48 kHz / 128-frame / four-period request using the discovered
`hw:CARD=A96,DEV=0` identity failed at open with ALSA EBUSY (16).
No new audio stream was established, and JACK remained active and unchanged.
No manufacturer-specific branch or card index was added to production code.

Earlier [0003](0003-engine.md) hardware evidence remains valid for its recorded
64-PEQ workload, including the loaded 30-second zero-xrun trial at 128/512 frames.
It does **not** qualify the new 126-EQ/compressor/transition workload. No new
physical live-edit, transient or dropout observation is claimed. There is still
no analog loopback; analog response/latency, converter voltage and speaker
protection remain unmeasured. Smaller periods were not tried.

## GitHub Actions

Queried `gh run list` and actual jobs for
[run 36608692588](https://github.com/PaolaShultz/shr-pa/actions/runs/36608692588).
ARM64 and x86-64 jobs passed, but that run predates both local implementation
slices. These working-tree changes have no remote CI result because they were
not pushed. Checkout remains `actions/checkout@v7.0.1` (Node 24); existing ALSA
package installation is preserved and the null-PCM control check is added.

## Remaining work and limits

GEQ tonal curves/restore, PA2 shelf-slope units, infinite-ratio compression,
additional crossover slopes/independent edges, limiter bypass/knee/calibration,
subharmonics, feedback suppression, RTA/setup mic, AutoEQ/wizards, preset slots/
profiles/recovery, lockouts, remote revisioned writers, automatic reconnection,
physical touchscreen and full hardware acceptance remain pending. A busy/failed
stream still requires explicit restart; recalling a preset never clears faults.
See the [function map](../DRIVERACK_MAP.md) for complete scope.
