# Current status

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
[hardware checkpoint](verification/0014-integration-checkpoint.md) exercised this
fixed full-range library on AudioBox USB 96 at 48 kHz, 192-frame periods and
768-frame buffers. A 600 s trial preserved dry/recording continuity with zero
xruns, but one late wet return failed the 8 ms admission target. An explicit
16 ms revision passed 30 s; repeated faults and its final soak were pending at
the checkpoint. The owning result is GigPies `docs/AUDIO_HARDWARE.md`.
Configurable embedding, acoustic acceptance and alignment remain pending.

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
partial transfers, startup priming, bounded waits and shutdown. Xruns/suspend,
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

The user confirmed no analog loopback, with outputs connected to an unpowered
amp. Analog round-trip latency, physical signal response, converter voltage,
noise floor, acoustic behavior and speaker protection remain unmeasured.

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
