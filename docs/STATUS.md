# Current status

## Optional live EQ extension — 2026-10-08 software acceptance

The [EQ v1 extension](LIVE_EQ_ABI.md) is implemented and independently reviewed.
It prepares exactly two distinct program-input EQ banks off the processing path,
then commits both at one boundary with a shared transition: 240 frames at 48 kHz.
Each bank has eight parametric bands and 31 graphic bands with independent enables.
The transition blends EQ outputs before continuing compression, delay, routing,
crossovers, speaker processing and protection. Graph configuration and output rearm
remain separate operations; the original embedding ABIs retain their layouts.

Validation passed 91 normal Rust tests, warning-denied Clippy, formatting, normal
and allocation-guard release builds, real C callers and software-null/terminal
checks. Coefficient fixtures cover 8/48/192 kHz; synthetic host checks exercise
actual owner admission, transition, retirement, bypass, faults and recovery.
The allocation evidence covers the narrow owner commit/render path, not the
surrounding host control loop. Physical outputs, listening, clock lock, acoustic
protection and whole-host deadlines remain separate acceptance work.

## Configurable owner graph and C-PA v2 — 2026-10-05

[Configurable embedding](EMBEDDING_V2.md) implements dynamic inputs, explicit
weighted DAG sums, independent speaker outputs, existing input/speaker EQ, gain,
mono dynamics, delay/polarity and final sample protection. Real 2×6 three-way,
2×8 four-way and weighted 4×8 fixtures share the graph and original DSP primitives.
The whole LR24 sum includes later-split phase compensation on earlier branches.
Fresh/replaced state remains muted until explicit rearm; active/prepared/retired
ownership and epoch/frame/generation fences are part of the additive v2 C ABI.
Fixed standalone behavior and v1 ABI remain unchanged. This supersedes old PA-02
and all-matrix deferrals below; measurement and physical acceptance remain separate.
[Software verification](verification/0017-configurable-embedding.md) records 79 normal
Rust tests, script checks, real C callers, format/Clippy/release and standalone
offline/software-null validation. Integration and hardware claims remain separate.


Application **0.2.0-alpha.1** uses processing schema **v3** and library/working
envelopes **v2**. These version numbers are independent. See [release notes](../CHANGELOG.md)
and the [release verification](verification/0007-0.2-alpha.md).

## Requested next measurement capability — phase and delay alignment

The [phase-alignment task](PHASE_ALIGNMENT.md), recorded on 2026-10-02, extends
P4/P5 with synchronized reference/setup-mic measurement and a bounded delay/polarity
proposal stage. Pink generation and pair controls already exist; measurement and
automatic alignment remain pending. RTA magnitude alone cannot supply this analysis.
Develop and validate the capability in SHR PA, with the finished PA module intended
for later GigPies integration. Standalone operation remains supported. No new DSP,
integration or physical validation was performed when recording this task.

## Embedding interface — 2026-10-03

The [version 1 C ABI](EMBEDDING.md) exposes the existing full-range PA processor
as `libshr_pa.so`, accepting stereo f64 and producing six logical f64 outputs.
Logical 0/1 carry L/R with unity gain, the existing −1 dBFS sample limiter and
5 ms startup ramp; 2–5 are silent. The same core now accepts f32 or native f64
buffers. ABI bounds, numerical faults, precision, protection and allocation
regressions are part of the normal suite. See [verification](verification/0013-embedding.md).
GigPies owns integrated USB/network/recording measurements. Its subsequent
[hardware results](verification/0014-integration-checkpoint.md) exercised this
fixed full-range library on AudioBox USB 96 at 48 kHz. The earlier 384-frame periods,
3072-frame buffers and 16 ms wet admission passed a 600 s / 28.8 million-frame bench
with exact dry/DAC replay, recording hashes and journal, and zero xruns, missing
or expired wet packets, or queue errors. Repeated packet-fault/Brain-restart
trials preserved local continuity; a forced driver stall correctly stopped with
an incomplete take. Earlier smaller-buffer failures and the failed 8 ms target
remain recorded. GigPies `docs/AUDIO_HARDWARE.md` owns the results. Subsequent
generated-only probes verified left output to input 1 under the recorded knob
settings; the right route remains unresolved and the user directs use of the
working channel. The measured reference/capture offset includes prefill and
does not isolate converter latency. Configurable embedding, acoustic acceptance
and alignment remain pending.

The user rejected the roughly 57 ms electrical reference/capture offset for live
use; 56 ms came from that bench configuration's prefill. Its continuity checks
do not establish acceptable live latency. The standalone
[capture pacing fix](verification/0015-transfer-pacing.md) now waits for a full
block before reading, avoiding an extra period wait after early partial reads.
It is offline/software-null validated; smaller-buffer physical acceptance and
the integrating host's latency work remain separate.

The subsequent [low-latency diagnosis](verification/0016-low-latency-integration.md)
records short GigPies trials around 5.19–5.35 ms with 48-frame blocks and zero
silent prefill, alongside failed reliability trials. A kernel trace identified a
6.219611 ms wait for page migration at a PA library function-address load despite
process memory locking. It did not identify DSP allocation or soundcard delay.
The H7 comparison temporarily excluded locked pages from ordinary compaction,
with verified restoration after each trial. Its 600 s / 28.8 million-frame run
had no USB xruns or queue drops, and all 11977 physical windows measured exactly
5.1875 ms. Two late wet returns failed the 4 ms admission gate, with 189 DAC
sample differences from uninterrupted replay.

H8's 6 ms wet admission then passed a 600 s / 28.8 million-frame digital trial:
zero xruns, wet losses, queue drops or network errors, with exact ADC/stem hashes,
dry/DAC replay and journals. Physical analysis found 11975 trusted windows at
249–251 frames (5.1875–5.229167 ms) and two weak windows. Finer analysis found
two one-frame offset changes; their cause and exact physical continuity remain
unresolved, so physical qualification stays open. Dry-path settings and the
measured PA library are unchanged. All temporary settings were restored.

## Working offline

- Hardware-independent, synchronous 2-input/six-output Rust engine with bounded,
  allocation-free rendering and explicit rate/block configuration.
- Full-range/external, two-way, phase-compensated three-way, six full-range and
  four-mains-plus-subs layouts; stereo/mono-left and explicit averaged mono bass.
- Live gain/polarity and delay transitions, per-output 5 ms mute ramps,
  31-band linked/separate GEQ, eight bell/shelf input PEQs per channel and eight
  speaker PEQs per output pair, and stereo-linked soft/hard-knee compression.
- Stereo-linked sample peak limiters, peak/clip/reduction meters and latched
  numerical-fault silence. See [precise behavior and limits](DSP.md).
- Seeded white and pink noise, sine, impulse and log sweep; stereo WAV input and six-channel
  float WAV output, independent of physical channel counts.
- 75 named user slots, six immutable layout templates, selection before recall,
  explicit overwrite/copy, atomic working-edit recovery and retained EQ restore points.
  GEQ manual/flat and three original curves; scoped input/pair PEQ flatten/restore.
- Independent HP/LP bypass/cutoff, BW6–48 and LR12/24/36/48 on every pair;
  original layout LR24 remains default with its three-way phase compensation.
- Validated v3 JSON presets and v2 library envelopes with explicit v1/v2 migration. Terminal edits
  and computed preview; compact keyboard/mouse navigation and terminal cleanup.

Pink generation (M03) has [offline and software-null evidence](verification/0008-pink-noise.md).
The session-start `--level=DBFS` option now sets its peak bound (−60…0 dBFS,
default −20); see [level evidence](verification/0009-generator-level.md).
Live `(`/`)` now edits the generator level with a 5 ms ramp; see
[runtime level evidence](verification/0010-runtime-generator-level.md).
Live `~` now toggles that selected source off/on with a 5 ms crossfade to/from
mapped capture; see [capture restoration evidence](verification/0011-generator-capture-restore.md).
Changing source type during a session and measurement workflows remain pending.

## Implemented live transport

Direct ALSA negotiation, independent native-format conversion, explicit input
and logical-to-physical output selection, silence for unused physical channels,
complete-block availability checks, partial transfers, startup priming, bounded
waits and shutdown. Xruns/suspend,
disconnect and numerical faults stop both streams; explicit restart is required.
Live terminal has all implemented module controls, six ramped mutes, peaks,
compression/limiting reduction, physical mappings and modified/pending/fault status.
One bounded prepared-transaction slot applies edits at block boundaries without
render allocation, locks or audio queues. Layout/recall use mute/reconfigure/resume.
Software-null streaming and PTY checks exercise these controls. Earlier live-control
physical trials were blocked by JACK owning the interface (left running); the
crossover slice made no hardware attempts.
See [live-control verification](verification/0004-live-controls.md) and
[preset/recovery verification](verification/0005-preset-library.md), and
[independent crossover verification](verification/0006-crossover.md).

## Earlier physical trials on this Pi

AudioBox USB 96 stereo capture/playback, S32_LE, 48 kHz, 128-frame periods and
512-frame buffers. All six logical outputs ran, including a fully enabled 64-PEQ
workload. A 30-second trial had zero xruns, mean render 84.3 µs and maximum
281.8 µs (2,666.7 µs period). Shorter/smaller-buffer trials and their failures are
recorded in [verification](verification/0003-engine.md). These are development
observations, not venue/long-soak acceptance or minimum-buffer guarantees.

Those earlier standalone trials had no analog loopback, with outputs connected
to an unpowered amp. The later GigPies left-channel electrical measurements above
have their own scope. Converter voltage, noise floor, acoustic behavior and
speaker protection remain unmeasured.

## Still pending

PA2 shelf-slope unit compatibility, automatic input-EQ restore sources,
subharmonic synthesis, feedback suppression,
RTA/separate mic integration, AutoEQ/setup wizards, limiter bypass/knee modes,
speaker/amplifier profiles, lockout/remote/update management, automatic device
recovery, and physical touchscreen integration. The [full map](DRIVERACK_MAP.md)
retains these features and acceptance work; partial rows are not marked complete.

UMC1820 is a **future target**, not a development gate. Its eventual channel and
physical acceptance work remains pending. General matrices, advanced routing,
eight-point positional RTA and the later nine-channel layout remain [future](FUTURE.md).

[Run commands and controls](RUNNING.md) · [Validation](VALIDATION.md)

## GigPies integration planning — 2026-10-04

[Owning GigPies plan](GIGPIES_IMPLEMENTATION.md) records scoped tasks, contract dependencies,
validation and launch instructions. This is planned work; existing implementation
and hardware status above are unchanged.

## PA-01 read-only embedding extension — 2026-10-04

The fixed C-PA:1 interface now exposes an 80-byte capability descriptor and a
24-byte quiesced handle-health snapshot, preserving the original v1 signatures
and processing behavior. They report the actual rate/block settings, logical
active/silent channels, linked sample-limiter scope, latched fault/recreation
requirement and unavailable configurable controls/measurement/acoustic/true-peak
capabilities. Invalid query shapes and owned-storage overlap leave output
unchanged. See [the contract](EMBEDDING.md)
and [E08 corpus](../tests/fixtures/cpa/v1/README.md) for reproducible software checks.

PA-02 configurable embedding and PA-03 phase measurement remain planned;
physical channel, acoustic safety and hardware integration acceptance retain
the limits above. This extension opens no physical endpoint.

PA-01 software checks passed: 70 normal Rust tests, five script tests, fmt,
warning-denied Clippy, release-linked C caller, docs/publication checks, offline
terminal recovery and explicit software-null live controls. Fresh version,
six-channel float WAV, 40×13 snapshot and source-preserving migration checks also
passed. Historical/long/exhaustive and hardware tests were intentionally skipped.
The coordinator owns final receiving review and source publication.
