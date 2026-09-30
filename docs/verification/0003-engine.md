# First working engine and stereo ALSA slice — 2026-09-29

> Historical evidence for the dated slice below. Test counts, pending features and
> commit/push statements describe that moment. See the [evidence index](README.md)
> for subsequent commits and the [0.2 alpha record](0007-0.2-alpha.md) for release checks.

Implementation working tree based on `488c682`, Rust 1.97.1, native aarch64 release
build. [Run commands](../RUNNING.md) and [DSP contract](../DSP.md) describe the
actual scope. Historical scaffold/planning records remain unchanged.

## Normal validation

- Formatting and Clippy (`--locked --all-targets -- -D warnings`): pass.
- Complete normal suite: **22 tests passed** (3 layout, 3 CLI, 8 contract,
  7 DSP/reference, 1 render allocation). No physical hardware is opened by tests.
- Seven offline PTY lifecycle cases: pass (keyboard, mouse, Ctrl+C, SIGINT,
  SIGTERM, SIGHUP, resize). Terminal attributes/cursor/mouse state restored.
- Native release build: pass.
- Three-way and two-way complete complex responses match independent analog
  Butterworth/bilinear references at 44.1/48/96 kHz, including summed phase.
- Full EQ/delay render, mute commands and numerical-fault path allocate/free
  zero times. Delay wrap, short blocks, all layouts, mono cancellation/gain,
  PEQ gain, linked limiter attack/release/ceiling and fault silence are covered.
- PCM byte conversion, selected-output packing and clearing unused physical
  channels, invalid local maps, partial I/O/EINTR/EAGAIN and terminal errors,
  atomic preset round-trips/rejected writes and deterministic WAVs are covered.
- Early startup cancellation has an explicit ALSA **software-only null PCM**
  regression; it does not wait for an unstarted capture stream.

## Hardware and authorization

Pi 5 Model B Rev 1.1, Debian 13, aarch64, kernel
`6.18.34+rpt-rpi-2712` PREEMPT. This is the observed OS; Linux Lite remains a
target. AudioBox USB 96 at `usb-xhci-hcd.0-1`; ALSA ID `A96`, selected explicitly
at the command line (never embedded in the backend). USB descriptors advertise
stereo capture/playback, S32_LE/24 significant bits and
44.1/48/88.2/96 kHz. Only 48 kHz was exercised in these physical trials.

The user authorized running audio, with the amp off. JACK initially held both
PCM endpoints; inspection found only system ports and no application connections.
The idle `jack.service` was temporarily stopped for direct ALSA, then restored
with its unchanged configuration (48 kHz, 128-frame periods, two periods).
No mixer settings, routing configuration, service files, scheduler policy or
host tuning were changed. End-of-trial temperature was 57.85 °C;
`vcgencmd get_throttled` returned `0x0` at both recorded checks.

## Stereo duplex trials

All trials ran the six-output three-way engine, including unmapped outputs.
Native S32_LE stereo was negotiated separately for capture/playback. Physical
mapping selected either H-L/H-R or L-L/L-R on channels 0/1, with no downmix.
Durations count processed program frames; startup/flush are additional.

The loaded preset enabled all **64 PEQ sections** (16 input + 48 output),
frequencies `80 * 2^band_index` Hz, Q 0.707, alternating −1/+1 dB; output pair
delays 0/2.5/5 ms. Other settings match `Config::default()` (120/1800 Hz LR24,
−1 dBFS limiter, 100 ms release). Generator is seeded −20 dBFS white noise.

| Trial | Period / total buffer (frames) | Program duration | Mean / max DSP µs | Xruns | Result |
| --- | --- | --- | --- | --- | --- |
| Default DSP, 1 kHz, high pair | 128 / 512 | 5 s | 84.45 / 110.31 | 0 | Completed |
| Loaded DSP, noise, low pair | 128 / 512 | 10 s | 87.63 / 152.17 | 0 | Completed |
| Loaded DSP, noise, low pair | 64 / 256 | 1,664 submitted frames (~35 ms) | 43.06 / 50.57 | 1 | Playback EPIPE; stopped, required explicit restart |
| Loaded DSP, noise, low pair | 32 / 128 | 5 s | 20.39 / 34.00 | 0 | Short observation only; not qualified |
| Loaded DSP, noise, high pair | 128 / 512 | 30 s | 84.26 / 281.81 | 0 | Completed after the earlier xrun/restart |

The user noted unreliable small buffers. **128-frame period / 512-frame buffer
remains the development request**, not a venue acceptance guarantee. A 32-frame
period is not a 32-frame total buffer. The contradictory short low-period outcomes
show why no minimum-buffer claim or reliable operating setting is inferred.
These runs used ordinary scheduling and included concurrent development activity;
they are not controlled worst-case or eight-hour qualification tests.

30-second render maximum was about 10.6% of the 2,666.7 µs period, measuring only
DSP work. Conversion, waits, USB buffers and converters are outside that timer.
Reported logical output peaks were approximately 0.19475 / 0.06120 / 0.01448
for the high/mid/low pairs; these are digital sample observations.

## Lifecycle and terminal checks on the card

Explicit `scripts/check-live.py` passed:

- Actual physical capture with startup-muted output and normal timed shutdown:
  48,000 frames, zero xruns, logical output peaks all zero.
- Unmuted physical capture → DSP → selected stereo playback: 96,000 frames
  (2 seconds), zero xruns, mean/max render 84.05/105.17 µs. Digital high-pair
  output peaks approximately 0.000052/0.000057; no acoustic or loopback claim.
- Active sine generation interrupted by SIGTERM: clean ramp/drop, zero xruns,
  card released and successfully reopened.
- Active live terminal: block progress, physical mapping display, unmute/mute,
  quit, terminal attribute restoration.
- Cancellation during startup.

The first live-terminal trial exposed early cancellation attempting to flush an
unstarted capture stream. This was fixed by tracking stream start before cleanup;
subsequent physical and software-null regression checks passed. An xrun terminates
the session safely; automatic reconnect/recovery is not claimed. Disconnect/suspend
error handling was tested synthetically; physical unplug/power-loss was not tested.

## Offline evidence and limits

Generated `artifacts/six-output-sweep.wav`: 144,000 frames, six float channels,
loaded DSP preset, 3-second log sweep. Output peaks approximately
0.11833 / 0.10436 / 0.09740 per pair. Render mean/max 74.15/196.22 µs. Local raw
trial logs/presets remain under ignored `artifacts/`; the settings and results
above are the durable record, with reproducible commands in RUNNING.md.

**No physical analog loopback is connected**, confirmed by the user. Outputs
remain wired to an amp that is off. No analog round-trip latency, physical
frequency/phase response, output voltage, acoustic response, channel/socket
calibration, inter-sample overshoot or speaker protection claim is made.
Physical input sample peaks are recorded, not treated as a calibrated noise floor.

Long soaks, exhaustive matrices, feedback auditions, physical disconnect/power
experiments and UMC1820 qualification were intentionally not run. No existing
slow historical default tests needed reclassification. WAV and SVG evidence
renderers were run explicitly; they remain opt-in.

## GitHub CI feedback

Queried live through `gh`, rather than assuming it is automatically delivered.
[Run 36608692588](https://github.com/PaolaShultz/shr-pa/actions/runs/36608692588)
passed ARM64 and x86-64 before this local implementation. Earlier failed runs
reported a terminal resize race, fixed in the existing history. The workflow
already uses `actions/checkout@v7.0.1` with `runs.using: node24`; no application
Node 20 target exists. Retained that action and added `libasound2-dev`/`pkg-config`
installation for the new ALSA dependency. New working-tree changes have not been
pushed, so there is no remote CI result for this implementation yet.
