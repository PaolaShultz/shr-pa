# Release notes

## Unreleased

- M03: deterministic, bounded pink noise for six-output offline renders and
  explicitly requested live sessions (`pink` / `--signal=pink`). Spectral and
  software-null checks are recorded in [0008](docs/verification/0008-pink-noise.md).
  Runtime source switching remains pending. No application or persistence version change.
- M03: `--level=DBFS` selects a −60…0 dBFS generator peak bound for offline
  renders and explicit live sources; default remains −20 dBFS. The level is
  session-only and rejects WAV/capture-only use. See [0009](docs/verification/0009-generator-level.md).

## 0.2.0-alpha.1 — “0.2 alpha” — 2026-09-30

Experimental fixed 2-input × 6-output PA processing for Linux and Raspberry Pi 5.
This release packages the implemented baseline; it does not complete the PA2 plan.

- Six-channel offline rendering and explicit ALSA physical mappings; all logical
  outputs process even on stereo hardware. Development defaults remain 48 kHz,
  128-frame periods and 512-frame buffers.
- Original LR24 layouts, including compensated three-way summation, plus explicit
  independent HP/LP bypass/cutoffs, BW6–48 and LR12/24/36/48 with documented polarity.
- GEQ, input/output bell and shelf PEQ, linked compression and sample-peak limiting,
  delays, gain/polarity, meters and ramped mutes. Prepared live transactions retain
  the latest desired edits under backpressure; faults require restart.
- 40×13 controls, 75 user slots, six immutable templates, working recovery and
  retained manual GEQ/PEQ histories.
- Processing schema **v3** and library/working envelopes **v2**. Explicit migration
  accepts processing v1/v2 and envelope v1, preserving sources and EQ histories.
  Application version `0.2.0-alpha.1` is independent of these schema numbers.
- Audited operating instructions, backup/recovery guidance, current signal-flow
  artwork and indexed historical evidence. See [migration](docs/RUNNING.md#existing-presets-and-library-migration)
  before opening older saved state.

Validation is recorded in [0007](docs/verification/0007-0.2-alpha.md). Hardware
soaks and exhaustive research remain opt-in. This release adds no hardware
measurements: short earlier AudioBox trials are limited to their recorded workload.
UMC1820, analog response/latency, physical protection and transient/soak acceptance
remain unverified. Limiter extensions, feedback/subharmonics, setup mic, RTA,
AutoEQ/wizards, profiles and remote operation remain in the [complete plan](docs/DRIVERACK_MAP.md).
