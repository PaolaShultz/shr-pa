# Architecture

**Proposed architecture.** Only the terminal scaffold exists today.

## Boundaries

| Component | Responsibility | Timing |
| --- | --- | --- |
| Audio host | ALSA duplex negotiation, conversion, timing, fault handling | One synchronous render thread |
| DSP core | Compiled graph, filters, routing, delay, dynamics | Same thread and block as the host |
| Control | Validate commands, prepare coefficients/graphs, persistence | Outside real time |
| Analysis | RTA, measurement, EQ fitting, feedback detection | Worker work fed by bounded taps |
| Interface | Terminal, touch, optional MIDI | Independent of audio deadlines |

Begin with modules; split out a DSP crate when DSP implementation starts. No
plugin loader, web service, async runtime or network dependency is needed for
the initial system.

## Audio path

Negotiate the hardware streams explicitly. Convert native capture samples into
preallocated DSP storage, run the entire graph, convert to the playback format,
and submit promptly. Capture and playback may have different native formats
and channel counts. The compiled graph maps logical ports to verified hardware
ports; it must clear unused playback channels.

There is no planned asynchronous queue between audio processing stages and no
extra ping-pong audio pipeline. ALSA and USB still have transport buffers; their
negotiated sizes and observed delays belong in the latency report. Control-state
handoff and analysis storage do not add a block of delay to the signal path.
ALSA's [PCM documentation](https://www.alsa-project.org/alsa-doc/alsa-lib/pcm.html)
describes period/buffer negotiation, mmap and xrun states.

The USB clock/feedback mechanism is a transport property, distinct from whether
our application uses synchronous DSP. Qualify it on the connected device.

## Render contract to implement

- Allocate and touch all render storage before activation, including maximum delays.
- No allocation/deallocation, blocking locks, file access, terminal output,
  formatting, or logging in render work.
- DSP loops have explicit channel, node, filter and command limits.
- Transfer prepared parameters through bounded queues; reclaim replaced state
  on the control side. A full queue produces explicit backpressure.
- Meter values are snapshots. Analysis overload drops analysis data and reports
  the loss; it never holds the audio path.
- Numerical faults, xruns, clock loss and disconnects have explicit states and
  recovery. A fault must not silently route unprocessed audio around filters.

## Latency budget

At 48 kHz, 32 / 64 / 128 frames correspond to approximately
0.667 / 1.333 / 2.667 ms per period. These are scheduling intervals, **not**
analog input-to-output latency. Measure converters, USB scheduling and queueing,
DSP, any lookahead and configured delay together using physical loopback.

Initial engineering gate proposal: worst observed render work below 50% of the
period under the acceptance workload, leaving scheduling margin. Report maximum
and distribution, not just average CPU. This threshold is a starting gate to
validate, not a demonstrated guarantee.

Start scalar, profile on ARM64, then optimize the actual hot loops. Keep buffers
contiguous and consider NEON across independent channels where it helps. Compare
precision and stability before changing numeric types. Do not enable fast-math
assumptions that invalidate finite-value protection. Measure scheduler policy,
locked memory, affinity, IRQ placement and cooling changes individually before
turning them into installation requirements. PREEMPT_RT is an experiment if the
standard kernel cannot meet the measured deadline.

## Control and recovery plan

Before live implementation, define startup-muted, ready, active and fault states.
Show why output is unavailable and retain the edited configuration through
recoverable errors. Reconnection must validate device identity, formats and
mapping before any resume. UI failure and engine failure are separate events;
choose and test their operating policy when the appliance lifecycle is defined.

A software mute cannot guarantee silence after process death, USB failure or
power loss. Determine physical output behavior on the bench before choosing
any external mute/watchdog requirement. Do not claim hardware protection from
software state alone.

The proposed local flow is inspect → edit → validate → apply, with direct access
to mute and recovery. Routine navigation does not change signal routing.
Invalid edits retain their location and explanation. Measurement cancel retains
previous DSP settings. The actual live controls are deferred until their roles
are specified; the current shell only navigates documentation pages.
