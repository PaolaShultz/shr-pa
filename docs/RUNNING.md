# Run the processor

Requires Linux, the pinned Rust 1.97.1 toolchain, `pkg-config` and ALSA development
headers (`sudo apt-get install libasound2-dev pkg-config` on Debian/Ubuntu).
No sibling repository, JACK server or UMC1820 is required.

```sh
cargo build --release --locked
./target/release/shr-pa init preset.json
./target/release/shr-pa check preset.json
./target/release/shr-pa
```

The default terminal is an **offline editor**. It opens no audio device. `s` saves
`preset.json` in the current directory; `l` loads and validates it, muting all
outputs. Saving is atomic (file sync, rename, directory sync). Invalid presets
leave the previous configuration intact. Schema v2 rejects unknown/missing fields and unknown versions. Runtime mutes and active test signals are never saved.
No automatic working-state recovery or 75-slot preset library exists yet.

## Terminal controls

Arrows/Tab and footer mouse buttons change pages. `q`, Escape or Ctrl+C exit.
Minimum terminal size is 40×13. Terminal mouse navigation works; raw physical
touchscreen integration remains unverified.

| Key | Operation |
| --- | --- |
| `c` | Cycle fixed layout |
| `i` / `o` | Stereo/mono-left input / mono bass average |
| `g` / `G` | Input gain up/down 1 dB |
| `a` / `A` | Input delay up/down 1 ms |
| `[` / `]` | Low crossover split down/up 10% |
| `{` / `}` | High crossover split down/up 10% |
| `b` | Select high, mid or low pair |
| `+` / `-` / `p` | Pair gain up/down / polarity |
| `d` / `D` | Pair alignment delay up/down 0.1 ms |
| `e` / `E` | First pair PEQ gain up/down 1 dB |
| `t` / `T` | Pair limiter ceiling down/up 1 dB |
| `1`…`6` | Toggle each logical output mute |
| `m` / `u` | Mute / unmute all |
| `r` | Run an offline 1 kHz, −20 dBFS preview; show computed peaks |
| `s` / `l` | Save / load `preset.json` |

The controls above also work live, except `r` (offline preview only). Gain,
polarity, filters, limiter and delay edits transition for 20 ms. Topology edits
use a 5 ms mute, reconfigure at a block boundary, hold muted through the 20 ms
transition, then resume the runtime mute choices. Preset recall stays muted until
a fresh unmute after completion. Sample-rate/block-size changes require restart.

### Module editor (offline page 2; live Tab)

| Key | Operation |
| --- | --- |
| `v` | Cycle GEQ, input PEQ, pair PEQ, compressor, limiter, gain/delay, crossover |
| `n` / `N` | Next/previous parameter |
| `x` / `X` | Increase/decrease selected value, or toggle/cycle an option |
| `j` / `k` | Next/previous GEQ band or PEQ section |
| `h` | Select input L/R; linked GEQ always edits L for both channels |
| `b` | Select high/mid/low output pair |
| Tab (live) | Switch module editor / six-output meter view |

The selected field and next field show current desired values. `*`/`modified`
means different from the last saved/recalled preset. Live displays the preset
filename on the controls page, physical mappings and six output peaks on the
meter page, plus held input clip bits, compressor/limiter reduction, transaction
counts, pending/busy state and faults. A full handoff slot retains the latest
editor snapshot for retry; repeated edits are coalesced. `busy` means audio is
still transitioning. Save stores the desired snapshot, including pending edits.
Errors remain visible until the next action. `s`/`l` use the command-line preset
path when live and `preset.json` in the offline editor. Runtime mutes are excluded.

All implemented module parameters are accessible here: GEQ enable/link/gain;
PEQ enable/type/frequency/gain/Q/shelf S; compressor enable/threshold/ratio/knee/
makeup/attack/release; limiter ceiling/release; input/pair gain and delays/polarity; fixed layout/input mode and crossover splits.
GEQ/PEQ gain steps are 0.1 dB; frequencies/Q use semitone-ratio steps. The JSON
accepts exact values within validated bounds. See [DSP meanings](DSP.md).

### Existing version-1 presets

Loading v1 fails with an explicit migration instruction. Convert to a separate
file, preserving the source:

```sh
./target/release/shr-pa migrate old-v1.json new-v2.json
./target/release/shr-pa check new-v2.json
```

Migration retains all prior values as bells, adds S=1, enables existing PEQs,
and adds flat/bypassed GEQ and a bypassed compressor. Unknown legacy fields are
rejected. Version-2 files require all new fields; changing only the version number
is not a migration. Existing v1 files are never silently reinterpreted.

## Six-channel offline artifacts

```sh
./target/release/shr-pa render preset.json impulse impulse.wav 1 --unmute
./target/release/shr-pa render preset.json sweep sweep.wav 5 --unmute
./target/release/shr-pa render preset.json noise noise.wav 5 --unmute
./target/release/shr-pa render preset.json sine:1000 tone.wav 1 --unmute
./target/release/shr-pa render preset.json stereo-input.wav processed.wav 60 --unmute
```

Output is a six-channel IEEE float WAV, ordered **H-L, H-R, M-L, M-R, L-L, L-R**.
No device mapping affects offline rendering. Input WAV must be stereo at the
preset rate; PCM integer and float input are supported. Rendering stops at EOF
or the requested duration; append silence to the input to retain a complete
filter/delay tail. Input and output files must differ. There is no resampling.

Generators are deterministic and peak at 0.1 (−20 dBFS) before processing. The
impulse has a 10 ms lead-in to clear the startup ramp. Sweep is logarithmic from
20 Hz to 0.4 × sample rate. Noise is seeded white noise. Pink noise is pending.
Timing reports measure just `Engine::render`, excluding WAV I/O and generation.

## Direct ALSA: explicit hardware and channel selection

```sh
./target/release/shr-pa devices
# Replace CARD_ID using the device inspection above; no card number is assumed.
./target/release/shr-pa live preset.json \
  hw:CARD=CARD_ID,DEV=0 hw:CARD=CARD_ID,DEV=0 \
  2 2 0,1 0,1,-,-,-,- 10 --ui
```

Arguments after the preset: capture device, playback device, capture channel
count, playback channel count, two capture indices, six output assignments,
duration in seconds. Indices are **zero based**; `-` means unmapped. The example
sends H-L/H-R to physical playback channels 0/1. To audition the low pair use
`-,-,-,-,0,1`. Duplicate or unavailable physical outputs are rejected locally
before opening ALSA. Unused physical channels are cleared. Nothing sums six
outputs into stereo; all six DSP channels continue to run.

Capture/playback must share one card/clock. The backend independently negotiates
native formats and actual period/buffer sizes, requires the requested rate and
channel counts, and prints the results. Supported conversion formats are S32_LE,
S24_3LE, S16_LE and FLOAT_LE (in preference order). Raw `hw:` endpoints avoid
implicit plugins, resampling or device fallback. `null` is available only when
explicitly named for software transport experiments; it is not hardware evidence.
The development request is 128 frames/period and four periods/buffer at 48 kHz;
read the actual negotiated values. Smaller settings are not qualified.

Live starts muted. With `--ui`, use the controls above; Tab opens modules. It displays actual
logical peaks and physical mappings. Without `--ui`, use `--unmute` explicitly
for a bounded run. Optional `--signal=sine:1000`, `--signal=noise`, etc. replaces
program input before processing and ends with the session; it is never recalled
from a preset. Without that option, the mapped physical capture feeds the DSP.

```sh
# Explicit, bounded hardware experiment; keep the external system at a known level.
./target/release/shr-pa live preset.json \
  hw:CARD=CARD_ID,DEV=0 hw:CARD=CARD_ID,DEV=0 \
  2 2 0,1 0,1,-,-,-,- 10 --unmute --signal=sine:1000
```

Startup primes ALSA with silence. Partial I/O advances by frames; EINTR/EAGAIN
are retried with bounded waits, and no progress for two seconds is an error.
Normal/keyboard/signal shutdown ramps mutes and flushes silence before dropping
both streams. Xruns, suspend, disconnect and numerical faults end the session,
drop both streams and require an **explicit restart** (which renegotiates and
starts muted). This first backend does not reconnect or resume sound automatically.
A busy device is a local error; offline DSP remains usable. On this Pi, the
existing `jack.service` was restored after testing and may own the card. For a
deliberate direct-ALSA session, stop it with `sudo systemctl stop jack.service`,
then restore it afterward with `sudo systemctl start jack.service`. SHR PA does
not stop other applications or services automatically. Software cannot
promise analog silence when USB, process or power fails.

## Three separate limits

1. **Logical engine:** two inputs and six outputs, regardless of hardware width.
2. **Present hardware:** the AudioBox reports two capture/two playback channels;
   only an explicitly selected output pair can be played at once.
3. **Present measurement connections:** no analog loopback and amp off, confirmed
   by the user. Processing time, sample peaks, negotiated buffers and xruns were
   measured. Analog round-trip latency, socket/voltage calibration, acoustic
   response and speaker protection were not measured.

See [DSP behavior](DSP.md), [status](STATUS.md) and the
[bench record](verification/0003-engine.md). UMC1820 qualification is future
acceptance work and does not gate engine development.
