# Implementation roadmap

This is the initial plan, not a list of working features. The first functional
reference is the dbx DriveRack family. PA2 is a concrete starting reference for
its processing feature list, not a restriction on the requested routing.

## Requirements carried forward

| Requested | Treatment in this plan |
| --- | --- |
| Raspberry Pi 5, Rust, Linux Lite, terminal | Primary platform and implementation |
| UMC1820 | First interface to qualify |
| Extremely low buffers and synchronous processing | Measure direct duplex ALSA; no extra block queue between DSP stages |
| LR24 | First crossover type |
| Matrix / crossover patching, e.g. 4×8 | Flexible topology; final dimensions pending |
| Mono sums, delays, limiters | Core processing |
| Up to 8 simultaneous measurement sources | Possible venue-setup measurement configuration |
| Later use of 9 channels | Preserve as a requirement; roles await the owner's explanation |
| RTA, automatic EQ, automatic feedback suppression | Separate implementation milestones |
| Small touch UI; optional controllers | Terminal control first, MIDI optional |

No layout is inferred from the nine-channel statement. Setup microphone use
and later operating use are not assumed to happen simultaneously.

## M0 — project foundation

- [x] Standalone Rust package, pinned toolchain and lockfile.
- [x] Offline terminal shell, keyboard and terminal mouse navigation.
- [x] MIT license, public-facing docs and original diagrams.
- [x] Normal CI checks on Linux x86-64 and ARM64 configured.
- [x] Hardware evidence and implementation plan.

Exit criterion: build and normal checks pass on this Pi; shell never opens audio.

## M1 — interface qualification and duplex transport

Implement an explicit device-inspection command, then a separate opt-in duplex
bench harness. Enumerate the ALSA hardware endpoints, rates, capture/playback
formats, channel counts, period constraints and controls independently. Record
actual negotiated values, not just requested ones. Identify the device by stable
properties and reject ambiguity; do not assume ALSA card 0.

Use one synchronous processing thread with direct `hw:` ALSA access. Start with
read/write access and explicit format conversion; compare mmap only if supported
and measurement shows a benefit. Handle partial transfers, interrupts, startup
priming, mismatched capture/playback readiness, xruns and unplugging explicitly.
Capture and playback scheduling must be measured before settling their wakeup
strategy. No automatic sample-rate conversion or fallback device.

Try 48 kHz first as a bench baseline, then compare 96 kHz. Sweep supported 128,
64 and 32 frame periods with supported period counts. These are experiments,
not promised operating settings. Investigate smaller periods only after stable
results. Save loopback latency, worst processing time, xruns, thermal state and
negotiated configuration together.

**Exit:** verified physical channel map, stable duplex timing, repeatable
round-trip measurement and explicit failure behavior. See [hardware](HARDWARE.md)
and [validation](VALIDATION.md).

## M2 — DSP primitives and protection

Implement an offline library before attaching speaker outputs. Each processor
must work independently of terminal, ALSA and persistence code.

| Processor | Implementation approach | Required evidence |
| --- | --- | --- |
| Gain / polarity / mute | Sample ramps, bounded finite parameters | Step transitions, silence, clipping/headroom behavior |
| Mono sum | Explicit input weights; averaging and unity sum are distinct | Correlated and uncorrelated signals; documented gain |
| Parametric EQ | Biquads; coefficients computed outside render | Reference responses, stability, extreme legal settings |
| Graphic EQ | Fixed frequency bands using the verified EQ core | Band and combined response; headroom under multiple boosts |
| Compressor | Linked or independent envelope; threshold, ratio, knee, attack/release | Static transfer curve, transients, pumping and channel tracking |
| LR24 | Two cascaded second-order Butterworth sections per branch | −6 dB at crossover, complementary summed magnitude and phase, impulse response |
| Delay | Preallocated circular storage; integer delay first | Exact sample alignment, wraparound, maximum memory; click-free changes |
| Limiter | Peak envelope, configurable attack/release and channel linking | Overshoot, bursts, recovery, sustained limiting and channel balance |
| Output handling | Finite checks, explicit conversion/clipping behavior | NaN/Inf faults, full-scale conversion and mute |

Use double precision for coefficient design; compare float and double state on
ARM64 before selecting the implementation. Flush or handle denormals deliberately.
Avoid unsafe coefficient interpolation through unstable intermediate filters;
validate bounded transitions and budget any temporary dual processing.

LR24 is the requested default. Its branches can sum flat with correct alignment;
it is not linear phase. Multiway split trees need phase accounting across all
branches, including any all-pass compensation. Test the complete multiway sum,
not only isolated pairs. Physical driver alignment remains a measurement task.
See [Rane's LR crossover primer](https://www.ranecommercial.com/legacy/note160.html).

Limiter design must make the latency tradeoff explicit: a reactive limiter does
not guarantee zero overshoot; lookahead adds a known delay. Set final behavior
from the protection requirements, then measure it. Amplifier gain and speaker
limits are needed before software thresholds can represent physical protection.

**Exit:** normal deterministic DSP tests pass, bounded render work is measured,
and protection limits are documented. No claim of speaker safety from a unit test.

## M3 — routing and system processing

Compile a validated, directed acyclic graph into a fixed processing order.
Represent physical ports separately from logical sources, sums, crossover
branches and outputs. Use names/IDs; do not bake a stereo feed or eight-output
layout into storage. Begin with the owner's supplied topology when available.

A 4×8 matrix is an example to exercise routing, not the permanent graph size.
Support weighted fan-in, fan-out, mono sums, polarity, linked crossover groups,
input and output EQ, and independent alignment delays. Reject cycles, missing
nodes, invalid parameters, duplicate physical ownership and graphs exceeding the
preallocated budget. Include worst-case correlated summing in headroom checks.
Do not let routing changes silently bypass an output's protection chain.

Prepare graph changes off the render thread, commit at a block boundary, and
retire old state outside real time. Bound command volume. Incompatible topology
changes use a defined mute/reconfigure/resume transition. Continuous values ramp.
Persistence gets a versioned schema, validation, atomic save and last-known-good
recovery before it can control live output.

**Exit:** routing/reference tests, boundary and recovery tests, graph allocation
checks and full normal suite pass. The final nine-channel layout must be defined
before its profile is built.

## M4 — RTA and measurement

Support up to eight simultaneous acquisition channels for venue setup. Each gets
calibration metadata, level/clipping indication and a common time base. Use a
bounded analysis tap so FFT work cannot delay audio. Record dropped analysis
frames instead of slowing the render thread.

Use windowed FFTs and power averaging with selectable resolution/smoothing.
Plan 2k–16k FFT sizes as initial experiments; long windows improve frequency
resolution but delay the display, not the audio path. Show individual microphones
and explicit spatial averages. Do not average unaligned raw waveforms and call
that a room response. Add calibrated SPL only when sensitivity and gain are known.

**Exit:** known-tone/noise tests, per-channel calibration tests, multi-input
synchronization checks and measured analysis CPU cost on the Pi.

## M5 — automatic EQ

Build on verified measurements: reject clipped or weak captures, apply microphone
calibration, choose a target curve and fit a bounded set of PEQ filters off the
render thread. Limit boost, Q and correction range; avoid trying to fill deep
cancellation nulls. Account for measurement position and repeatability.

The operator sees a proposed curve and can audition, apply or revert it. Keep the
previous settings until the new set is accepted. Measurement excitation is an
explicit setup action with an immediate stop control. Verify the result with a
new measurement; do not judge success only by the fitter's own predicted curve.

**Exit:** synthetic room cases, poor-data rejection, repeatability and measured
venue trials. Algorithm details remain open until measurement data exists.

## M6 — automatic feedback suppression

Track narrowband peaks over time with persistence/growth criteria and safeguards
against treating sustained musical notes as feedback. Apply a bounded number of
notch filters with limits on width and depth. Distinguish setup ring-out filters
from live adaptive filters; expose filter state, hold and clear controls.

Run detection outside the render thread and apply validated filter commands at
block boundaries. Test on recorded speech/music, tones, feedback onset and changing
acoustic paths. Quantify false positives, detection delay and remaining headroom.

**Exit:** repeatable feedback cases and listening/venue evidence, with documented
limits. This is our implementation, not a clone of dbx's proprietary AFS algorithm.

## M7 — operating interface and appliance validation

Replace scaffold pages with the actual routing, processing and measurement flows.
Retain the compact terminal convention where it fits; verify the chosen display's
physical touch targets. Keep live level, mute/fault state and the selected path
visible. Separate edits from their applied state and preserve context on errors.

Keyboard access must cover every action. Map terminal touch to the same commands.
Add direct touch integration only if the chosen console requires it. MIDI is
optional: learning, pickup and disconnect handling follow after the local UI.
No renderer or controller may block the audio thread.

Finish bounded telemetry, startup/recovery, preset migration, thermal testing,
long duplex soaks and measured end-to-end latency. Select any scheduler/IRQ/kernel
tuning from results. Provide an opt-in service installer only after lifecycle
behavior is proven.

## DriveRack reference coverage

The [PA2 feature list](https://dbxpro.com/en-US/products/driverack-pa2) provides a
concrete comparison: graphic/parametric EQ, automatic EQ, feedback suppression,
compression, crossover, limiting and alignment delay. Its subharmonic synthesis
is also a parity item to review after core system management. Networking and
vendor speaker presets are not assumed requirements for this terminal project.
Track parity by feature and test evidence; do not label the first DSP build
“DriveRack equivalent.”
