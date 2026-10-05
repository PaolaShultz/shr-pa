# Configurable C-PA v2 — 2026-10-05

The additive [C header](../src/shr_pa_v2.h) and strict JSON graph schema provide
owner-native configurable PA DSP. The original [v1 ABI](EMBEDDING.md), fixed
standalone presets and existing terminal remain supported with unchanged behavior.
No physical endpoint, speaker calibration or measurement engine is added.

## Ownership and boundary protocol

All calls are exclusive single-owner operations: the controller and audio worker
must transfer ownership, never concurrently inspect/mutate a handle. Keep the
library loaded until all prepared, active and retired handles are destroyed.
`prepare(json, bytes, 2)` parses, validates, designs coefficients and allocates
controller-side. Input JSON is copied; callers may release it immediately.
`validate` provides a bounded, NUL-terminated diagnostic. Unknown fields/schema,
nonfinite/out-of-range values and resource failures are rejected, not truncated.
Status queries on prepared handles are supported (`committed=0`).

A fresh handle is muted and quiescent. At an explicit source-frame boundary,
`apply(&active, prepared, &retired, epoch, frame)` transfers ownership, returning
old state through the initially NULL retirement reservation. It does not destroy
or allocate anything. A nonempty retired slot rejects another application; this
is one pending prepared change and one retirement reservation, without an
internal queue. The host owns any scheduling/transport queue and admission.

For replacement: request `mute`, render its 5 ms ramp until `quiesced=1`, prepare
and validate the new graph offRT, then commit. Preparation can also precede mute.
Old state stays active/muted on rejection. A successful commit stays muted until
an explicit `rearm` with the current epoch/frame. Free retired state offRT before
reusing the reservation. Discard an unwanted fresh prepared handle offRT. The
initial NULL-active application assigns generation 1; replacements increment it.
Retired handles cannot render or be resubmitted as fresh prepared state.

Epoch zero is invalid. Same-epoch application must match `next_frame`; a new
epoch must increase. Rendering requires exact epoch/frame continuity, explicit
input/output counts and 1..max_block frames of interleaved native f64. Bad shape
leaves output/state unchanged. Discontinuity or numerical failure silences the
whole block and latches a fault; explicit fresh-state application/rearm is required.
The failed block does not advance the source cursor. Status reports the generation,
applied frame, epoch and next frame, never a wall-clock estimate. A muted graph
still processes and advances its matching source timeline.

The 80-byte status and 64-byte capabilities structures require exact version 2
and size. Header fixed-width fields and reserved zeros define their layout.
Pointer alignment, checked spans, buffer overlap and all known handle-owned
allocation overlaps are rejected. This cannot prove backing allocation validity,
detect dangling pointers or synchronize callers. `destroy` is controller-only,
once per owned handle, after the worker stops referencing it.

## Graph schema and signal path

The canonical serde types are [GraphConfig/Input/Node/Output](../src/graph.rs).
[Three-way](../tests/fixtures/cpa/v2/stereo3way.json),
[four-way](../tests/fixtures/cpa/v2/stereo4way.json) and
[weighted 4×8](../tests/fixtures/cpa/v2/matrix4x8.json) are complete executable JSON
fixtures. The graph schema version 2 is separate from standalone preset schema 3.
Hosts retain the exact accepted configuration with its generation for control
readback/persistence. Restoring intent always prepares muted state; never persist
rearm permission. Unknown versions must be refused, not silently adapted.

Inputs, nodes and outputs are ordered vectors. Their zero-based indices are
module-local ports, never physical USB/socket mappings. A source is `{"input": N}`
or `{"node": N}`. Every node is an explicit weighted sum of its routes. Node
references must point backward in topological order; invalid, forward and cyclic
references are rejected. There is no hidden normalization: weights multiply then
sum exactly, so callers own headroom. Duplicate routes intentionally add (they
are not duplicate physical writers). Each output vector index has one writer;
physical-output uniqueness is the host's separate mapping contract.

```text
program input -> gain -> 8 PEQs -> 31 GEQ bands -> mono compressor -> input delay
              -> topologically ordered explicit weighted sums
              -> output source -> compensated LR24 branch -> gain/polarity
              -> 8 speaker PEQs -> sample limiter -> delay -> final ceiling guard
              -> persistent global mute ramp / configured output mute
```

Input/speaker EQ use the original RBJ primitives and shelf-slope interpretation;
crossover and delay reuse the original owner DSP primitives. A source of `null`
is explicitly silent. Measurement channels are not invented or automatically
routed into program. Program dimensions are admitted from configuration.

Each output selects ascending `splits_hz` and `band` 0..number-of-splits (low to
high). Empty splits mean full range. For band j: cascade HP24 at every earlier
split, LP24 at split j when present, then one second-order allpass at **every**
later split. This recursively splits the remaining high branch and compensates
earlier low branches. The sum is the product of all split allpasses: unity
magnitude with their common phase, not zero phase. Fixtures list high-to-low
stereo pairs to preserve familiar six-output ordering. Custom branch/sum weights,
EQ, gain, polarity and delay deliberately modify the summed response.

Protection follows each actual summed source and speaker EQ/gain. Mono sample
limiters are independently processed per output, with existing instantaneous
attack/release arithmetic. Final guards also protect delayed samples. v2 does not
advertise stereo linking; existing standalone/v1 linked behavior is preserved.
This is sample-peak protection, not true peak or calibrated speaker protection.
Measurement, true peak, acoustic calibration and physical I/O capabilities are
explicitly zero. No algorithmic crossover lookahead is added; configured input
and output delays and host buffering remain separate.

All configuration controls currently use the mute/prepare/commit/rearm path,
including route, crossover, EQ, gain, polarity, delays and limiter settings. No
live coefficient/delay switch is hidden inside processing. Replacement safely
resets delay/filter histories under mute. Configured output mutes remain in force
after global rearm. A future live-control transaction may preserve histories,
but is not advertised by this ABI.

## Available ranges and resource admission

| Control or budget | Range / origin |
| --- | --- |
| Sample rate / block | 8000..192000 Hz / 1..8192 frames, existing engine support |
| Input gain / delay | −60..20 dB / 0..100 ms |
| Output gain / delay | −60..20 dB / 0..10 ms |
| PEQ / GEQ | Eight PEQs per input/output; 31 input GEQ bands, −12..12 dB |
| PEQ frequency / Q / shelf S | 20..min(20000,0.45×rate) Hz / 0.1..15.909 / 0.1..1 |
| LR24 splits | Strictly ascending 16..min(20000,0.45×rate) Hz |
| Route weights | −16..16, explicit engineering headroom bound; no normalization |
| Sample limiter | −60..0 dBFS ceiling, 1..2000 ms release, never bypassed |
| Mono compressor | Threshold −60..0 dB, ratio 1..100, knee 0..24 dB, makeup −20..20 dB, attack 0.1..200 ms, release 1..2000 ms |
| JSON bytes | 1 MiB controller parsing/admission budget |
| Inputs / nodes / outputs | At most 4096 each: descriptor traversal budget, not hardware/product claim |
| Per-frame work estimate | At most 65536 units: 40/input + routes + 2×splits+12/output |
| Delay storage | At most 8,388,608 f64 samples (64 MiB), preallocated max input/output delays |

The work estimate bounds loops, not CPU deadlines. GEQ frequency bands above the
usable rate range are identity, preserving their stored controls. Budgets are
reported by capabilities and failures explain the violated bound. Increasing them
requires resource/deadline review and capability update; 4×8, 16, 32 and 48 are
examples, not ceilings. No realtime hardware-throughput claim follows from these
software tests. Host physical port maps and clock synchronization require their
own acceptance.

## Validation

The normal graph tests compare each three/four-way impulse response and summed
complex response against independently computed analog LR24 prototypes after
bilinear prewarping. They cover all weighted matrix sources, 16/32/48/53 ports,
EQ/delay/polarity partition equivalence, protection, silent unused outputs,
cycles/resources, exact ABI sizes, stale timelines, failed atomic replacement,
persistent mute and retired-state ownership. Allocation guards include apply,
render, fault/refusal, status and retirement at 16/32/48 inputs.

The [actual C caller](../tests/fixtures/cpa/v2/caller.c) links the release library,
loads these same fixtures, renders, mutes and swaps ownership. Reproduce it using
[fixture commands](../tests/fixtures/cpa/v2/README.md). Run the complete normal
suite and release/publication checks in [VALIDATION](VALIDATION.md). Physical I/O,
long load runs, historical auditions and measurement research remain opt-in.
