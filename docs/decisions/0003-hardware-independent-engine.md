# 0003 — logical engine independent of physical channel count

Accepted and implemented, 2026-09-29. Supersedes any development ordering in
0001/0002 that could make UMC1820 qualification a prerequisite for the DSP engine.
Those earlier records describe the historical scaffold/planning work.

The current logical processor remains 2×6. UMC1820 is an eventual target; its
availability, channel qualification and tests do not gate engine development.
The currently connected AudioBox supplies stereo capture/playback. A validated,
explicit map selects which logical outputs reach those physical channels.
Unmapped logical outputs keep running and can be rendered offline. No six-to-two
sum or general matrix is inferred. Physical sockets/calibration/analog latency
require their own evidence and appropriate connections.

The first implemented slice uses fixed layouts, prepared f64 DSP state, bounded
f32 block I/O, separate control/persistence and terminal modules, and direct ALSA
with native conversion and negotiated buffers. It adds no audio-stage queue.
A versioned preset is rebuilt only while stopped; only per-output ramped mutes
are edited during streaming. Faults terminate the hardware session; an explicit
restart renegotiates and starts muted. More advanced recovery/transitions remain
future implementation work rather than unsafe partial live edits.

See [DSP behavior](../DSP.md), [commands](../RUNNING.md) and
[evidence](../verification/0003-engine.md).
