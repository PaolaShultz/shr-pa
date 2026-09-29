# Architecture — fixed 2×6 processor

**Target architecture with an implemented first slice.**
The actual processing/transport contracts are in [DSP](DSP.md) and [running](RUNNING.md).
The diagram below includes still-planned feedback, bass synthesis and measurement.
GEQ, bell/shelf PEQ and compression are implemented; see [DSP](DSP.md).
The [function map](DRIVERACK_MAP.md) defines the PA2 baseline and the
[roadmap](ROADMAP.md) its implementation sequence.

## Signal path

Two program inputs, six outputs and a separate setup-microphone capture channel.
Start with fixed full-range/two-way/three-way configurations, not a graph editor.
The microphone feeds measurement only. Mono-input and mono-bass choices have
explicit source and gain rules in the map.

```text
Program L/R -> input meters -> input mode / test-source selection
  -> 31-band GEQ -> 8-band room PEQ -> feedback notches
  -> subharmonic mix -> compressor -> input/backline delay
  -> crossover, band gain and polarity
       HIGH L/R -> speaker PEQ -> limiter -> alignment delay -> mutes -> meters
       MID  L/R -> speaker PEQ -> limiter -> alignment delay -> mutes -> meters
       LOW  L/R -> speaker PEQ -> limiter -> alignment delay -> mutes -> meters

Setup mic -> calibration / capture quality -> RTA and setup measurement
Program tap -> feedback detector -> prepared notch updates
```

This follows the processing order in the PA2 manual's block diagram (printed
p. 60; [source](https://www.fullcompass.com/common/files/43013-DriveRackPA2UserManual.pdf)).
Noise replaces program at the defined source-selection point for measurement;
its start/stop state is explicit. Verify excitation paths for each wizard.
Stereo band settings are paired; delay/filter state and output mutes are per
logical channel. Inactive and unmapped hardware outputs must be written as zero.

## Boundaries

| Component | Responsibility | Timing |
| --- | --- | --- |
| Audio host | ALSA duplex, conversion, clock/timing and faults | One synchronous audio thread |
| DSP core | Fixed configuration, EQ/crossover, dynamics, delays and generator | Same thread and block |
| Control | Validate commands, prepare parameters, presets and setup state | Outside real time |
| Analysis | One-mic RTA/measurement, EQ fitting and feedback detection | Workers using bounded taps |
| Interface | Local terminal/touch; remote client in P8 | Independent of audio deadlines |

The DSP is implemented in the library, separate from transport and terminal work. The control boundary
should support later engine/client separation for remote operation. Do not add
a network server, async runtime or general graph compiler to this first slice.

## Transport and latency

Negotiate capture and playback independently: their native formats and channel
counts can differ. Convert into preallocated storage, run the complete processing
chain, convert and submit playback. Process the required roles even when the USB
endpoint exposes extra channels. Logical output processing never depends on physical channel count.
Confirm socket mappings for whichever card is attached; UMC1820 is future work.

No asynchronous queue or extra ping-pong pipeline sits between DSP stages. USB
and ALSA transport buffers still exist and must be measured. Parameter handoff
and analysis storage do not add an audio block. Use the
[ALSA PCM contract](https://www.alsa-project.org/alsa-doc/alsa-lib/pcm.html) for
negotiation, partial transfers, mmap capability and xrun states. USB feedback is
a device transport property, independent of the application's synchronous DSP.

At 48 kHz, 32/64/128 frames are approximately 0.667/1.333/2.667 ms per period.
These are deadlines, not measured analog latency. Loopback must include converters,
USB queueing, processing and any enabled delays/lookahead. Publish baseline latency
with intentional delay settings separately. No operating buffer is approved yet.

## Render and optimization contract

- Allocate, size and touch every audio/filter/delay buffer before activation.
- No allocation/free, blocking lock, file/network/terminal work or formatted log
  in render. Bound processor count and parameter commands per block.
- Prepare and validate coefficients off-thread. Use safe transitions; bound and
  benchmark any temporary parallel filter evaluation during an edit.
- Reclaim replaced control state outside render. A full queue reports backpressure.
- Meter snapshots and analysis taps have fixed capacity. A slow client or FFT
  worker may lose telemetry, never hold the audio thread.
- Reject non-finite/out-of-range parameters. Define numerical faults, xruns,
  disconnects and clock loss without silently bypassing output protection.

For budgeting, the fully enabled baseline can contain 62 GEQ sections, 16 input
PEQ sections, 48 output PEQ sections, and 24 notch instances when the 12 feedback
filters are applied to both program channels. Add actual crossover sections,
subharmonic filters, envelopes and conversions. Count enabled work and benchmark
that configuration, including worst supported crossover slopes and parameter edits.
The count is an implementation budget, not measured CPU usage.

Start scalar with contiguous storage. Design coefficients in f64, compare f32/f64
state accuracy and speed on the Pi, and handle denormals deliberately. Optimize
measured hot loops; consider NEON across independent channels/sections where the
data dependency allows it. Do not use floating-point assumptions that remove
finite checks. Profile analysis and UI separately; cap their update rate.

A proposed initial gate is worst observed processing time under 50% of the period
with the acceptance workload. Record maxima/distributions and xruns. Evaluate
real-time scheduling, locked memory, affinity, IRQ placement and cooling from
measurements; investigate PREEMPT_RT if the standard kernel cannot meet deadlines.

## State, persistence and recovery

The first slice uses a validated fixed schema, atomic JSON save/load, runtime
mutes outside presets and explicit restart on audio faults. Prepared transactions now support live parameter edits through a single atomic
slot. Related edits begin together at block boundaries, with bounded transitions
and preserved unrelated state. Topology edits/recalls use mute/reconfigure/resume.
The rest of this section describes the longer-term target.

Use a fixed, versioned configuration schema. Presets contain processing, setup
selections and profile references; mutes, RTA preferences and utility/access settings
are global. Working edits can be recovered without overwriting saved presets.
Apply related settings as a validated transaction at a block boundary; incompatible
configuration changes use a defined mute/reconfigure/resume transition.

Plan startup-muted, ready, active and fault states. Show recovery next to the
fault and preserve edited work. Reconnection revalidates device identity, format
and channel map. The configured startup mute policy must not override a runtime
fault. Test signals always stop on cancel/fault and never resume from a saved preset.

Remote/local commands share validation, permission and revision checks. A client
reconnect reads the current engine state before writing. UI disconnection and
engine death have different policies; establish them before the P8 split.
A software mute cannot guarantee analog silence after USB/power/process failure;
bench those cases before making any physical protection claim.
