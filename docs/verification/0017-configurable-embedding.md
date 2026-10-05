# Configurable PA embedding software verification — 2026-10-05

C-PA v2 implements the [owner contract](../EMBEDDING_V2.md), preserving the fixed
v1 interface and standalone processor. Tests run on Raspberry Pi 5 with Rust
1.97.1, committed Cargo.lock, CARGO_INCREMENTAL=0, one Cargo job and the shared
nonblocking build reservation. Physical audio devices were not opened.

| Requirement | Production code | Acceptance |
| --- | --- | --- |
| Dynamic inputs, weighted sums, independent outputs | `graph::GraphConfig`, `Graph` | Every input at 16/32/48/53; exact neutral samples and 4×8 weights |
| Three/four-way LR24 phase compensation | `Graph::prepare`, reused `dsp::Biquad` | Each branch and total complex response against independent analog/bilinear reference |
| Existing PA controls | `Input`, `Output`, reused EQ/delay/compression primitives | Input/speaker PEQ, GEQ, compression reach samples; exact gain/polarity/delay checks |
| Sum/output protection | `Graph::process` | Weighted sums, speaker gain, delay and final ceiling; explicit silent output |
| Structural transactions and safe recovery | `ffi_v2` | Persistent mute, quiescence, failed atomic commit, explicit rearm, source epoch/frame fault |
| Realtime ownership | `shr_pa_v2_apply/process/status` | Allocation/deallocation guard at 16/32/48; occupied retirement reservation refuses; retired render refuses |
| Exact ABI and actual library | v2 header and C corpus | 80/64-byte C static assertions; all three JSON graphs rendered by release-linked C caller |
| Compatibility | Existing `ffi`, `Engine`, terminal | Original v1 actual C caller, all old Rust tests, offline terminal and software-null live control scripts |

Passed: 79 normal Rust tests, format, warnings-denied all-target Clippy, release
build, five script unit tests, documentation checker, actual C v1 caller and C v2
caller with all three provider fixtures. Offline terminal cleanup/recovery and
explicit ALSA software-null live transaction/restart tests passed. Publication
checks cover the complete staged index; the coordinator owns upstream publication
and consumer/library/network acceptance. Exact local source/header/fixture/library
hashes are retained in the private task0014 provider manifest, not copied into
an independent consumer implementation.

The focused graph suite takes roughly 1.3 seconds on this host; this is test wall
time, not a realtime capacity measurement. Resource admission is documented and
advertised; no 48-channel hardware deadline claim follows. The 4×8 matrix uses
explicit weights and no normalization. Output limiters/compressors are mono in
v2; legacy stereo linking is preserved in the original processor. All v2 control
changes currently use muted replacement, including delay history reset.

Intentionally skipped: physical PCM/playback, UMC/ADAT clock and socket mapping,
acoustic/true-peak/calibrated protection, measurement, historical auditions,
exhaustive research and long load/thermal campaigns. Existing on-demand commands
remain in [validation](../VALIDATION.md). These checks do not authorize hardware,
prove physical synchronization, or complete the wider DriveRack measurement plan.
