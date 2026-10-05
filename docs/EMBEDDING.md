# Embedding the PA engine

## Additive configurable successor — 2026-10-05

[C-PA v2](EMBEDDING_V2.md) now implements dynamic program inputs, explicit
weighted mono routing and independent protected outputs, including compensated
LR24 stereo three/four-way and 4×8 references. Existing standalone fixed presets
and v1 bytes/semantics remain supported. This supersedes earlier fixed-2×6-only
ordering and deferral of all matrix/configurable embedding below. Scope here is
software implementation; physical I/O, acoustic/true-peak protection and
measurement remain unqualified or unavailable as documented.


`cargo build --release --locked` builds the standalone application and
`target/release/libshr_pa.so`. The versioned C interface is declared in
[src/shr_pa.h](../src/shr_pa.h). It embeds existing SHR PA processing in a host
without a sibling Cargo dependency. The library opens no audio device, network
socket or recording file. One host owns physical I/O and maps logical outputs.

## Version 1 contract

| Call | Contract |
| --- | --- |
| `shr_pa_v1_create(rate, max_block)` | Controller allocation/preparation; returns an opaque handle or null for invalid settings |
| `shr_pa_v1_process(handle, input, output, frames)` | Exclusive worker, interleaved stereo f64 input and six-channel f64 output |
| `shr_pa_v1_delay_frames()` | Zero fixed algorithmic frames for this preset |
| `shr_pa_v1_descriptor(output, version, size)` | Read-only fixed capabilities; exact version 1 / 80 bytes |
| `shr_pa_v1_status(handle, output, version, size)` | Quiesced health; exact version 1 / 24 bytes |
| `shr_pa_v1_destroy(handle)` | Controller destruction after the worker stops; null is safe |

Rates are 8000–192000 Hz. Prepared block sizes are 1–8192 frames; each process
call accepts 1 through the prepared maximum. The host supplies `frames*2`
initialized input doubles and `frames*6` writable output doubles. Input, output
and handle storage must be aligned, live and disjoint. A handle has one owner:
simultaneous process/destruction or two concurrent process calls are invalid.
The host must retain the loaded library until all its handles are destroyed.

Null, alignment, arithmetic-overflow, overlap and frame-bound errors return
`SHR_PA_INVALID_ARGUMENT` (−1) before changing state or output. These checks
cannot prove arbitrary pointer validity, allocation lengths or absence of races.
Successful processing returns `SHR_PA_OK` (0). Non-finite samples or an internal
numerical fault return `SHR_PA_FAULT` (−2), silence the whole block including its
earlier samples, and latch silence for subsequent valid blocks. The host must
explicitly stop processing, destroy and recreate to reset a fault. A fresh
handle starts a fresh 5 ms gain ramp; it never inherits old filter state.

Process performs no allocation, deallocation, lock, I/O or coefficient design.
Allocation failure during creation follows the Rust allocator's process-abort
behavior; null specifically indicates rejected settings. Valid process calls
have no expected panic path. Arbitrary foreign-pointer misuse is outside the
contract and must not be treated as a recoverable audio fault.

## Fixed processing configuration

Version 1 exposes the existing `FullRange` stereo layout with unity input/output
gains, flat EQ, compressor bypass, zero delays, and stereo-linked −1 dBFS sample
peak limiting with 100 ms release. Logical outputs 0/1 contain protected L/R;
2–5 are explicitly silent. The host chooses physical channels. Creating this
interface explicitly requests unmute: the existing 5 ms startup ramp opens
automatically. The standalone application retains its startup-muted controls.

There is no lookahead or block queue in this preset. The startup ramp changes
gain, not sample alignment. USB buffers, host prefill, network admission and FX
algorithm delays are separate quantities. The limiter is an existing sample
ceiling, not a calibrated speaker, amplifier, RMS or true-peak safety system.
Hosts must keep the PA limiter after any mixed wet signal that needs its ceiling;
mixing additional audio after PA output can exceed that ceiling.

`Engine::render_f64` uses the same DSP implementation as the existing
`Engine::render` f32 interface, with no intermediate f32 audio conversion.
Approximate meters remain f32; filter, delay, dynamics and audio state are f64.
Future configurable embedding should use a new explicit contract and preserve
the current version's semantics. Preset mutation, measurement/alignment, remote
authority and transport recovery are outside version 1.

## Validation and integration ownership

Normal regressions check ABI bounds, native precision, full-block fault silence,
fault persistence/recreation, stereo/channel identity, startup ramps, linked
limiting, output silence, f32/f64 agreement across all six layouts and allocation
counts. See [verification](verification/0013-embedding.md).

GigPies owns the integrated capture/network/recording host and its actual-device
measurements. SHR PA remains independently buildable and owns PA DSP. This
interface alone establishes no USB, acoustic or physical-latency acceptance.
The subsequent [2026-10-03 hardware checkpoint](verification/0014-integration-checkpoint.md)
records actual USB use, continuity/recovery findings, the failed 8 ms wet target
and a successful 600 s trial at 16 ms with larger device buffers. Subsequent
generated-only probes verified left output to input 1 under bounded conditions;
the right physical route remains unresolved. Their reference/capture offset
includes prefill and does not isolate converter latency.
The later [low-latency diagnosis](verification/0016-low-latency-integration.md)
records short measurements around 5.19–5.35 ms with zero silent prefill, retained
reliability failures, and a traced kernel page-migration wait during PA processing.
The measured library is unchanged. H7 held 5.1875 ms in every analyzed window of
a 600 s run without USB xruns, but two late wet returns failed 4 ms admission.
H8's 6 ms wet admission passed 600 s with exact digital replay and zero losses.
Its physical offset ranged over 249–251 frames, with two weak windows and small
offset changes that still require qualification. All temporary settings were
restored; this does not establish complete live or acoustic acceptance.
GigPies `docs/AUDIO_HARDWARE.md` owns acceptance. Measurement/alignment remains the separate
[planned capability](PHASE_ALIGNMENT.md).

## Read-only descriptor and health (C-PA:1 / E08)

The additive queries preserve all original v1 signatures and processing behavior.
The header defines fixed-width caller-owned structs and constants. Pass version 1
and the exact `sizeof` the corresponding struct. No output initialization is
required; a successful query writes every field. Unknown version or size, null,
misalignment, address-span overflow or status overlap returns −1 without writing
output. The caller must supply live, correctly sized, unaliased writable memory;
these checks cannot establish arbitrary pointer validity.

Status queries require the handle's single owner to quiesce processing and all
other queries/destruction. Output cannot overlap the inline handle or any of its
eight heap delay allocations. Queries allocate nothing and perform no locks or I/O.
They are snapshots, not concurrent telemetry or a fault reset. A valid status
query returns 0 even when `fault_latched` and `recreate_required` are 1. Fault
recovery remains stop/destroy/create; a recreated handle reports both flags as 0.
`sample_rate` and `max_block` report the actual creation settings.

The 80-byte descriptor has twenty 32-bit fields in header order. Values identify
native f64 (1), the fixed full-range preset (1), two input and six logical output
channels, active mask 3 (0/1), silent mask 60 (2..5), and host-owned physical I/O
(`physical_io_owned=0`). Limiter kind 1 is the linked sample limiter, threshold
−1000 milli-dBFS, release 100 ms, startup ramp 5 ms and fixed delay 0 frames.
Rate/block bounds are 8000–192000 Hz and 1–8192 frames. The unavailable bit mask
15 explicitly covers configurable controls, measurement, calibrated acoustic
protection and true-peak limiting. It does not describe the broader standalone
editor's controls or establish physical acceptance.

The reusable [E08 corpus](../tests/fixtures/cpa/v1/README.md) contains expected
values and an actual C caller checking layout, bounds, stereo identity, limiting,
latched fault silence and recreation against the release library.
