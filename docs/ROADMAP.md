# Implementation roadmap — DriveRack functions in 2×6

**Current scope: two program inputs, six outputs, one separate setup microphone.**
The [DriveRack function map](DRIVERACK_MAP.md) is the complete inventory for the
PA2 reference: processing, setup, measurement, presets, operation, remote control,
maintenance and hardware differences. The first engine, offline tools and direct ALSA slice now work; see [status](STATUS.md).

Stages describe feature groups, not a requirement to finish hardware qualification
before DSP work. P2 and initial P3/P7 work proceed independently of P1. Keep all
mapped functions visible as work progresses. LR24 is the first/default crossover.
Independent BW6–48 and LR12/24/36/48 edges now work in D08. Matrix routing,
advanced patching, eight-point positional RTA and the later nine-channel layout
are [future work](FUTURE.md).

## P0 — foundation (done)

Rust package, toolchain/lockfile, offline 40×13 terminal shell, MIT license,
illustrated documentation and Linux x86-64/ARM64 CI. No audio device is opened.

## P1 — capability-based transport and eventual hardware qualification

**Initial stereo backend exercised; full hardware acceptance pending.**
Use available interfaces by capabilities with explicit physical mappings. A stereo
card runs all six logical outputs; unmapped channels remain available offline.
UMC1820 availability and channel qualification never gate DSP implementation.

Covers H01–H04. Inspect the UMC1820 when eventually attached: stable device identity, capture
and playback formats independently, negotiated rates/channel counts/periods,
clocking, physical gain/pad/monitoring controls and actual socket mapping. Bind
two program inputs, a separate mic and six output roles only after verification.
If the native USB stream has more channels, process the required ones and clear
all unused playback channels. No implicit sample-rate conversion or device fallback.

Develop the direct ALSA duplex bench alongside the offline engine. Start with read/write and explicit
conversion; compare mmap only if supported and useful. Handle partial transfers,
interrupts, stream priming, capture/playback readiness, xruns and disconnects.

Use 48 kHz, 128-frame periods and 512-frame buffers as the development request.
Smaller periods remain unqualified; the 64-frame trial failed. Do not infer a
32-frame operating target from the earlier short observation. Compare 96 kHz only after a stable baseline. Record actual negotiated
values and physical loopback latency. These are test points, not promised settings.

**Exit:** repeatable 2×6 mapping, duplex operation, measured transport latency and
fault behavior. Preserve measurements with kernel/firmware/USB/thermal context.

## P2 — fixed signal path, crossover and protection (first slice working)

See [DSP behavior](DSP.md) and [first-slice evidence](verification/0003-engine.md).
Live filter/delay transitions now work; calibrated physical protection remains pending.

Covers C01–C04, D07–D10, O01–O02. Create a DSP library with fixed-size channel and
processor storage. Implement the configuration table in the map; no general graph
compiler or matrix UI. Keep processing order fixed and paired band parameters clear.

Build gain/polarity/ramped mute, explicit mono sum, LR24, pre-delay, band delays,
peak limiter and meters. Use two cascaded Butterworth sections per LR24 edge;
verify complete three-way summation/phase, not just a pair of filters. Alignment
and compensation decisions must be represented in the response tests.

Implement delays as preallocated circular buffers. At 48 kHz, two 100 ms input
lines plus six 10 ms output lines require 12,480 sample slots, about 49 KiB in
f32 before storage padding; memory is not the limiting factor here. Avoid abrupt
read-position jumps during live adjustment; use bounded transitions.

Implement the limiter envelope with a measured overshoot/recovery bound. Tie
output protection to actual calibration; any lookahead is an explicit delay.
Unused outputs remain zero; faults do not bypass crossover or protection.

**Exit:** offline impulse/sweep, mono/stereo routing, numerical-fault, delay,
limiter and mute tests; render allocation checks; low-level bench loopback.

## P3 — full processing controls and presets (core modules working)

GEQ, bell/shelf PEQ, stereo-linked compressor, prepared live edits, compact module
controls and versioned presets with explicit migration are implemented. See
[verification](verification/0004-live-controls.md). The practical preset/EQ slice
now adds 75 user slots, immutable templates, naming/copy/selection/recall, working
recovery, original GEQ curves and scoped PEQ restore; see
[preset evidence](verification/0005-preset-library.md). Independent crossover edges
and all planned BW/LR slopes now have [D08 evidence](verification/0006-crossover.md).
Extended limiter modes, PA2 shelf units and automatic input-EQ sources remain pending.

Covers D01–D03, D06, D08–D11, O04–O05. Add 31-band GEQ, dedicated eight-band room
PEQ, eight-band speaker PEQs, compressor and full parameter/bypass controls.
The mapped crossover families now coexist with the retained layout LR24 default.

Design coefficients outside render. Use double precision for coefficient design;
measure f32 versus f64 state before choosing. Check shelf response and extreme
Q settings. Do not interpolate filters through unstable coefficient sets. Bound
any extra work for transitions and preserve channel balance in linked dynamics.

Use a versioned preset schema for fixed configurations, typed parameters, setup
selections and profile references. Keep global mutes/preferences and working edits
separate. Add immutable templates, at least 75 user slots, naming/copy/recall/save,
atomic writes, working-state recovery and migrations. Store no active test-signal
intent. Implement our own tonal curves and identify them as such.

**Exit:** filter/dynamics reference tests, full-configuration renders, extreme
parameter tests, curve/restore tests and persistence/recovery tests. Benchmark the
fully enabled path, not only a minimal LR24 configuration.

## P4 — measurement input, RTA and test signals

Covers M01–M03, H03. Capture one setup microphone on the same hardware clock;
keep it isolated from PA playback. Add calibration, signal level and clipping checks.
Feed an analysis worker through a bounded tap. Drops are visible and invalidate
affected measurements; analysis never delays audio.

Implement a windowed FFT and calibrated 31-band power display, slow/fast response,
peak hold, display offset and readable views. Begin FFT-size experiments off the
audio path. Add deterministic white noise, pink shaping and a controlled sweep
source for P5. Generate excitation in bounded render work, outside any display loop.

**Exit:** tone/noise/calibration tests, all display controls, generator level and
insertion-point tests, startup/cancel/fault stop, and real single-mic capture.

## P5 — setup, level balancing, AutoEQ and tuning profiles

Covers M04–M07, M09, O06. Implement the manual setup flow and generic 2×6
configurations before relying on profiles. Define a profile format for speakers
and amplifiers with units, provenance and validation. Allow manual/unlisted equipment.
Apply speaker EQ, crossover, polarity, delay and limiter settings through the same
validated configuration path; do not invent manufacturer tunings.

For level balancing, measure the relevant speakers/bands, estimate differences,
and either guide physical adjustment or apply bounded, visible trims. Check noise,
clipping and headroom. Make trims removable and retain the old configuration.

For AutoEQ, measure sweeps using one microphone, with up to four sequential
positions in the reference workflow. Average aligned response estimates/powers,
not unaligned raw audio. Fit up to eight PEQ bands off-thread with limits on boost,
Q and correction range. Avoid filling cancellation nulls. Provide the three target
curve roles in the map using our documented curves. Keep automatic/manual/flat
states separate. Apply and verify with a fresh measurement.

Compose setup, level-only, EQ-only, combined and feedback stages into a cancellable
wizard. Support rerunning one stage on the existing configuration or starting a
new setup; retain completed work and remember selections/name preferences.

**Exit:** synthetic known-response and poor-data cases, calibration/trim tests,
all configuration paths, cancel/retry/partial rerun, then repeatable venue evidence.

## P6 — feedback suppression and subharmonic synthesis

Covers D04–D05, M08. Both belong to the 2×6 capability plan.

Feedback: start with narrowband tracking and persistence/growth evidence in a
worker, then a bounded command path to twelve notch slots. Implement fixed/live
allocation, the three width policies, live replacement, clearing, gradual timed
lifting and per-filter inspection. Use program-input evidence; test mono-analysis
cancellation and define the detector's actual policy explicitly. Add guided
ring-out and automatic handoff to live filters. Evaluate false triggers on speech,
flute, sustained notes and recorded music. Document detection delay and limits.

Bass synthesis: band-limit the mono analysis feed, evaluate an octave-divider
with tracked envelope, shape the two output regions and mix their bounded levels
into the stereo dry path. Compare alternative generators only if tracking/artifact
tests fail. Gate silence/noise, remove DC, and test interaction with compressor,
crossover and limiter. Default off. No proprietary dbx algorithm is claimed.

**Exit:** synthetic signal/feedback regressions, filter lifecycle tests, bounded
CPU, sustained listening tests and controlled acoustic trials. Record one-time
audition evidence and keep its renderer opt-in.

## P7 — complete local operation

Covers O01–O10 and H05. Implement the live terminal flows for every mapped module,
preset and wizard. Provide configuration, dynamics, RTA and system-info views,
quick module access, band stepping, visible current preset/edits and all six mutes.

Add persistent preferences, timeout, readable display choices, device naming,
startup mute policy, forced-muted startup, four lockout modes, global-only reset
and full reset. A reset shows its scope and permits cancellation. Adapt physical
LCD/demo controls to the terminal explicitly; do not leave source features untracked.

Use common commands for keyboard and touch. Validate physical touch separately
from terminal mouse tests. Cap refresh rate and telemetry work. Controller or
rendering stalls must not enter the audio thread. Define and test UI/process
failure policy before live deployment.

**Exit:** every ID has a reachable local operation and tests for normal use,
cancel, restart, rejection and recovery. Actual display and touch trials pass.

## P8 — remote operation, maintenance and full acceptance

Covers O06, O11–O13, H01/H05 and integration of all earlier IDs. Keep remote
controls in scope: authenticated terminal access and a shared command interface
can expose the same functions without requiring a desktop GUI. Separate the engine
lifetime from client connections. Use revisioned commands to reject stale edits;
serialize writes and bound telemetry for slow clients. Support device identity,
network status, access control and reconnect. Test from the intended client OSes.

Provide optional validated profile/catalog import, versioned releases, update,
backup/migration and rollback. No update or catalog download may block render.
Do not require the network for local use or local profiles.

Run the fully enabled 2×6 path while measuring its actual peak load. Validate
startup-muted operation, all fixed configurations, retained settings, failures,
restart, disconnect/reconnect, clock loss and physical output transients. Choose
scheduler/IRQ/affinity/kernel tuning based on evidence. Begin with a proposed
50% worst-render-time margin and an eight-hour zero-xrun soak under UI/analysis
load; publish actual results and conditions, not an unqualified latency guarantee.

**Exit:** function-by-function evidence in the map, complete normal test suite,
recorded hardware/acoustic acceptance and reproducible installation/rollback.
Anything unfinished retains its ID and pending status.

## Test policy

[Validation](VALIDATION.md) defines the normal suite. Fast DSP, configuration,
schema, protection, command and recovery tests belong there. Hardware sessions,
long soaks and exhaustive/one-time research are explicit opt-in tasks. Each stage
starts with focused tests; shared engine/model/render changes require the full
normal suite. Offline DSP regressions and short AudioBox duplex evidence now exist; physical
loopback and full hardware acceptance remain pending.
