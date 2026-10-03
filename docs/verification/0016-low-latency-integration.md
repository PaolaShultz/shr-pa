# Low-latency integration diagnosis — 2026-10-03

GigPies owns the integrated host, measurements and acceptance in its
`docs/AUDIO_HARDWARE.md`, sections H4–H8. This owner record follows the
[large-buffer checkpoint](0014-integration-checkpoint.md) and
[standalone transfer fix](0015-transfer-pacing.md). No PA DSP or ABI change is
part of this documentation update. Private recordings, traces and failed takes
remain with the measuring project.

## Latency target and measured scope

The user rejected the earlier roughly 57 ms electrical reference/capture offset
for live use. That configuration inserted 56 ms of silence before playback.
GigPies now separates ring capacity from prefill: capture starts first, then the
first processed block is written before playback starts. Its smaller-buffer
trials use 48-frame processing blocks at 48 kHz, zero silent prefill and the
unchanged native f64 PA library. SHR PA's standalone startup policy remains as
documented in the transfer-fix record; these host measurements do not validate
different standalone startup settings.

The physical reference is generated on left output only at −54 dBFS, returned
to input 1. Captured audio never feeds playback in these tests. The right route
remains unresolved; the user directed use of the working channel. Continuous
physical analysis compares intended DAC samples with the ADC return in 100 ms
windows at 50 ms steps, excluding startup silence and the final short tail.
That offset includes host, USB and converter timing. It does not isolate
converter delay, establish acoustic response or prove continuity outside the
analyzed windows.

## Smaller buffers: short passes and retained failures

- A 96-frame ring failed with the current synchronous transfer path. In the
  CPU3/FIFO20 retry, writes blocked for up to 1.401 ms with about 6.5 µs of
  thread CPU time, while capture backlog filled the ring. This is evidence
  about this host path, not the interface's intrinsic minimum capacity.
- A 144-frame ring passed short trials. With CPU3/FIFO20 and 4 ms wet admission,
  a 30 s run had no xruns or missing wet packets; all 497 physical windows after
  5 s measured 257 frames (5.3542 ms). Other trials failed, including a capture
  fault after 294912 frames. Earlier physical offsets also changed despite zero
  reported xruns. Exact recorded digital samples alone do not establish
  uninterrupted physical output.
- A 192-frame ring retained 48-frame blocks and zero prefill. Packet loss,
  Brain restart and fresh recovery trials preserved local sample accounting,
  with measured offsets of 249 frames (5.1875 ms). A deliberate 100 ms driver
  stall correctly retained an incomplete take. The following 600 s attempt
  failed after 36.914 s: the render interval took 3.597 ms wall time but only
  0.191 ms thread CPU. Its retained samples verified exactly; the last prepared
  block was not claimed delivered. The long-run reliability gate failed.

These comparisons show that spare ring capacity need not add an equal amount
of deliberately queued audio. They do not establish reliable live operation
at the smallest tested buffers. All failed trials remain part of the evidence.

## H6: locked-page migration during PA processing

The instrumented host records render wall time, thread CPU time and calling-thread
minor/major faults and voluntary/involuntary switches. These counters cover
observed render intervals, not the full capture/write cycle. Counter-read errors
make that observation incomplete. Memory locking applies only to the fresh owned
host process and is restored after PCM stops.

Locking 145008 KiB did not remove the failure. A traced 48/192-frame trial stopped
after 867744 fully written and 867792 recorded frames. Its failing render took
6.468 ms wall time and 0.276 ms thread CPU, with one minor fault, one voluntary
switch and zero involuntary switches. Perf switch timestamps
`52591.965418389`–`52591.971638000` bracket a **6.219611 ms** wait; the kernel stack
contains `migration_entry_wait_on_locked`, `migration_entry_wait`,
`do_swap_page` and `handle_mm_fault`.

The faulting PA instruction, `memset@plt+4` at library address `0x38c4`, is
`ldr x17, [x16, #3784]`: a load of the function address from the GOT. This trace
attributes the wait to a page under migration. It does not show a DSP allocation,
expensive `memset` computation or a soundcard delay. The existing PA process
contract remains allocation-free. The `do_swap_page` path also handles migration
entries, so its name does not establish disk swap-in. The trace does not identify
the migration initiator; diagnostic overhead remains part of this traced run.

The measured PA library remains code revision `390a45f` with SHA-256
`0c69949d99d209f4b23c0256a1b80f8b3b9d169d4d09d91ce9f064ea5bfd75a4`.
The H6 host is GigPies revision `1539915c845160d4a38fd1ba7f77ae1c1cce666a`,
v13 binary SHA-256
`8402bbcb88ae37ac4fae9202087218970413da02dfe04394bc7fa3def0f74500`.
Both failed locked takes retained exact samples and returned locked memory to zero.

## H7: stable analyzed offset; 4 ms wet admission failed

The kernel's `compact_unevictable_allowed` value was 1. Linux documents that
compacting locked pages can block a task on a minor fault, and that ordinary
memory locking does not prevent page migration. See the
[kernel setting](https://docs.kernel.org/admin-guide/sysctl/vm.html#compact-unevictable-allowed)
and [locked-page migration](https://docs.kernel.org/mm/unevictable-lru.html#migrating-mlocked-pages).

H7 used a fresh peer/coordinator reservation for one bounded comparison: the same
v13 host and owner libraries, active process memory locking, and that one key
temporarily changed from 1 to 0 with verified restoration after each trial.
Fresh checks require no other locked process memory. The reviewed separate helper
owns restoration on pipe closure, parent exit, handled signals or deadline;
uncatchable termination and host failure require coordinator recovery. No other
kernel, service, IRQ or audio-device setting is part of this comparison.

The selected configuration kept 48-frame blocks, 192-frame ring capacity, zero
silent prefill, FIFO20 on CPU3 and 4 ms wet admission. A separate 144-frame ring
passed 30 s but settled at 257 frames of physical offset; the 192-frame ring
measured a lower, stable 249-frame offset and retained more spare capacity.

Packet/stall and Brain-restart trials each preserved 768000 recorded frames,
local dry continuity and physical offsets of 249 frames, with wet output
recovering after the injected faults. A forced 100 ms driver stall retained an
exact, explicitly incomplete 96000-frame take. A fresh 30 s run then passed
without xruns or missing wet returns.

The 600 s soak completed 28.8 million frames with zero USB xruns or recorder/network
queue drops. All eight recorded PCM hashes, direct ADC hashes, dry replay and
journal entries matched. All **11977 analyzed physical windows measured 249
frames / 5.1875 ms**, with zero weak windows or detected persistent offset steps.
The first quiet second and final 100 ms remain outside this correlation coverage.
Render p99/max was 134/299.386 µs; post-read service p99/max was 168/392.644 µs.
Observed render fault and context-switch counters and observation errors were
all zero. The earlier multi-millisecond render stalls did not recur in this run.

The integrated zero-loss gate nevertheless **failed at 4 ms wet admission**.
All 600000 returns arrived, but two missed admission and were counted as two
missing and two expired packets. RTT maximum was 4156.374 µs while Brain FX
compute maximum was 77.129 µs. Intended DAC samples differed from uninterrupted
FX replay at 189 samples, consistent with the retained deadline-loss evidence.
The exact recording hashes describe the samples actually submitted; they do not
erase those wet-output differences or identify which network worker was delayed.

Each completed H7 trial verified process locking during streaming, zero locked
memory afterward and restoration of the kernel key to its original value 1.
These results support the targeted comparison under its measured conditions.
The setting does not prevent every migration source; the kernel's CMA
range-isolation path can still request locked pages, as shown in the inspected
[compaction source](https://raw.githubusercontent.com/raspberrypi/linux/rpi-6.18.y/mm/compaction.c).
That branch was not matched byte-for-byte to the installed kernel. The original migration
initiator remains unidentified, and no permanent host-setting recommendation
or general live-reliability claim follows.

## H8: 6 ms wet admission passed; physical offset needs qualification

H8 increased only wet admission to 288 frames / 6 ms, retaining the
same v13 binary, owner libraries, 48/192-frame audio setup, zero prefill, audio
thread settings, process locking and temporary kernel-key comparison. It adds
no dry-path queue or silence. The fresh reservation covered repeated recovery
checks and another 600 s soak.

The soak completed **28.8 million frames with zero xruns, missing or expired wet
returns, queue drops or network errors**. All 600000 wet returns arrived. All
eight stored PCM hashes, native ADC hashes, dry replay, uninterrupted DAC replay
and journal entries matched exactly. Render p99/max was 135/289.665 µs;
post-read service p99/max was 168/385.146 µs; RTT p99/max was 426/3732.252 µs.
Observed render fault and context-switch totals and observation errors were zero.
This passes the recorded digital zero-loss gate at 6 ms wet admission.

Physical qualification remains open. Of 11977 physical windows, 11975 were
trusted at **249–251 frames / 5.1875–5.229167 ms**; two 100 ms windows were weak.
Additional 10 ms analysis localized one-frame offset changes near 513.71 s and
520.30 s. The first transition window remained weak (correlation 0.580); the
second was trusted (0.782). No persistent step of at least 24 frames was detected.
The original analyzer and its thresholds were unchanged, and `requires_review`
remains true. These observations do not establish a fixed physical offset,
identify clock drift or prove exact converter-sample continuity or a dropout.
The first quiet second and final 100 ms remain outside the analysis.

The repeated packet/stall and Brain-restart trials retained exact ADC/stem
hashes, dry replay and journals, with no USB xrun or recorder gap. Their analyzed
physical offsets stayed at 249 frames. Wet output recovered in both trials;
the deliberate 20 ms loss faded over 240 frames with maximum error 0.621 PCM24
LSB against the quantized preceding sample. Expected missing/expired packets
and seven restart socket errors remain in the fault reports. The forced driver
stall again retained an explicitly incomplete take, followed by an exact fresh
recovery run.

Process memory locking and audio-thread settings were restored, and the kernel
key returned to its original value 1 after each trial. The coordinator released
resources at 22:21 UTC in exchange record `38e31a0`. No permanent setting change
was made. The PA library and standalone transport were not rebuilt for H7/H8.

GigPies retains the full trial summaries, fault assessment and small-offset
review. The next physical acceptance work must explain or bound the observed
offset changes; the right route, acoustic response, calibrated speaker
protection, complete UI workload and broader live reliability remain unverified.

## Documentation validation

This follow-up changes documentation only. Documentation links, the scoped
staged diff and the complete-index publication guard were checked. Software
builds, production tests, historical renderers and additional hardware operations
were intentionally not repeated by the PA owner while the integrating host
owned hardware timing.
No public push or release was made.
