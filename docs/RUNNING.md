# Run the processor

Requires Linux, the pinned Rust 1.97.1 toolchain, `pkg-config` and ALSA development
headers (`sudo apt-get install libasound2-dev pkg-config` on Debian/Ubuntu).
No sibling repository, JACK server or UMC1820 is required.

```sh
cargo build --release --locked
./target/release/shr-pa --version
# From the repository root, create an isolated session.
SHR_PA_BIN="$PWD/target/release/shr-pa"
mkdir -p artifacts
SESSION=$(mktemp -d "$PWD/artifacts/session-XXXXXX")
"$SHR_PA_BIN" init "$SESSION/preset.json"
"$SHR_PA_BIN" check "$SESSION/preset.json"
(cd "$SESSION" && "$SHR_PA_BIN")
```

The shell remains in the repository root after the editor exits. The examples
below reuse `$SESSION`; migration examples name their own source/destination paths.

`init` writes a generic three-way LR24 preset (120/1800 Hz, stereo, 48 kHz,
128-frame maximum block, zero gain/delay, flat PEQ, bypassed GEQ/compressor,
−1 dBFS limiters). It **replaces an existing destination without a prompt**.
Use a fresh path. `render` also replaces its output WAV.

The default terminal is an **offline editor**. It opens no audio device. It starts with defaults or compatible working recovery;
it does not automatically import `preset.json`. Press `l` to import that file. `s` saves
`preset.json` in the current directory (press twice to confirm an existing file); `l` loads and validates it, muting all
outputs. Saving is atomic (file sync, rename, directory sync). Invalid presets
leave the previous configuration intact. Schema v3 rejects unknown/missing fields and unknown versions. Runtime mutes and active test signals are never saved.
Interactive sessions also use a local `.shr-pa` library: in the current directory
offline, or beside the command-line preset live. Keep each system/session in its
own directory. A second editor can run without persistence when that library is
already locked; the library view reports `NO STORAGE`. Snapshots, WAV rendering,
validation and non-UI live runs do not recover or write working state.

### First edit and preview

In a fresh offline session, press `u`, then `r` for a computed preview. This
produces meters only, with no speaker output. Press `g` to raise input gain,
then `r` again. Press `:`, type `select U1`, Enter, then `:save My first setup`
and Enter to save a library preset. `s` separately exports processing JSON.
Press `m` to close all mutes and `q` to exit. Reopening recovers edits muted.

### Local library and working recovery

Press `P` for the library view; `j`/`k` browse slots/templates without recalling. Press `:` to enter a command, Enter to apply,
Backspace to edit, or Escape to cancel. Ctrl+C still exits from the prompt.
These commands work offline and during explicit `live --ui` streaming:

| Command | Operation |
| --- | --- |
| `:select U75` | Select user slot 1–75; show name, layout, rate and block size |
| `:select T4` | Select immutable template 1–6 |
| `:recall` | Recall selection through the existing mute/transaction path |
| `:save My venue` | Save desired working state to selected empty user slot |
| `:save! My venue` | Explicitly replace selected user slot, including its name |
| `:copy U12 Spare setup` | Copy the **selected saved preset/template** to an empty slot |
| `:copy! U12 Spare setup` | Explicitly replace the destination with that copy |
| `:geq manual` | Restore retained manual GEQ gains |
| `:geq flat` | Audition zero GEQ gains while retaining manual settings |
| `:geq speech`, `:geq warm`, `:geq gentle` | Audition original documented curves |
| `:flat inL`, `:restore inL` | Flatten/restore eight PEQs on input L (`inR` for R) |
| `:flat H`, `:restore H` | Flatten/restore eight PEQs on high pair (`M`/`L` for others) |
| `:recover-reset` | Archive rejected working state, then checkpoint current edits |

Names contain 1–24 printable ASCII characters to fit the 40-column terminal. `save!`/`copy!` explicitly authorize an overwrite;
plain save/copy reject occupied or corrupt destinations. Templates are T1 full
range, T2 external, T3 two-way, T4 three-way, T5 six full-range, T6 four mains plus
subs, all generic 48 kHz/128-frame configurations. They contain no speaker tunings.
Select/copy/preview changes no processing or runtime mute. Recall deliberately
mutes all six outputs and needs a fresh unmute after the pending/busy state clears.
Incompatible rates/blocks are rejected before replacing edits; start a new session
with a compatible standalone JSON preset. Selection remains available for inspection.

The library view separates selected slot, active preset baseline, modified state,
and recovery state. Live pending/busy/fault indicators remain underneath it;
`Active` identifies the last saved/recalled baseline, and pending/busy means its
latest processing changes have not settled. `P` closes the view; live Tab returns
to meters. Save records the desired snapshot, including edits waiting for audio.
Standalone `s`/`l` still export/import processing JSON; importing establishes new
manual EQ settings and clears restore history. Library slots retain that history.

Every processing edit is checkpointed synchronously on the controller, independently
of saved presets. Startup restores compatible working edits **muted**. No file
opens hardware, restores an unmute, starts a generator or clears an audio fault.
Only explicit `live` opens audio; `--signal` is a fresh command-line request.
`live --ui` always starts muted, including when `--unmute` is also supplied.

Corrupt, incomplete, unknown-version or rate/block-incompatible working files
leave startup defaults/the explicit live preset muted and report blocked recovery.
They are preserved and further working writes are blocked. `:recover-reset` keeps
a timestamped `working-rejected-*.json` archive before recording current edits.
Interrupted temporary files are ignored. Saved slots remain independently usable.
Failed checkpoints show `Unsaved`; disk failures can lose edits made since the last
successful checkpoint. See [persistence contract](DSP.md#library-and-working-state).

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
| `(` / `)` (live only) | Generator level down/up 1 dB, clamped to −60…0 dBFS; requires `--signal` |
| `s` / `l` | Save (confirm overwrite) / load `preset.json` |

The controls above also work live, except `r` (offline preview only). During live
streaming, gain, polarity, filters, limiter and delay edits transition for 20 ms. Topology edits
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
makeup/attack/release; limiter ceiling/release; input/pair gain and delays/polarity; fixed layout/input mode, crossover splits and independent HP/LP edges.
GEQ/PEQ gain steps are 0.1 dB; frequencies/Q use semitone-ratio steps. The JSON
accepts exact values within validated bounds. See [DSP meanings](DSP.md).

### Crossover controls

Use `v` to select CROSSOVER, then `n`/`N` for the field and `x`/`X` to edit.
After layout, input, low split, high split and mono bass, field 6 is **Mode**.
Default **Layout LR24** retains the original compensated three-way tree.
Select **Independent** explicitly to seed per-pair edges from the layout and
remove the shared three-way phase correction. The mode/phase policy stays visible. A summary shows both independent edges
together (off or family/slope plus Hz), or the three-way layout cascade.

Fields 7–10 are HP bypass, cutoff, family and slope; fields 11–14 are the same
for LP. `b` selects H/M/L; inactive pairs are labelled OFF and remain silent.
Cutoffs step by a semitone and clamp at validated bounds. Families display BW/LR;
BW steps 6–48 by 6 dB/octave, LR steps 12–48 by 12. Switching BW to LR rounds
an unavailable slope upward to the next supported slope; the value is displayed.
Bypass retains all edge settings. Pair gain/polarity remain on `+`/`-`/`p` and the
gain/delay page. Nothing adjusts gain or polarity automatically.

In independent mode, split shortcuts reject rather than discard edge edits.
Layout changes retain independent edges. Returning Mode to Layout LR24 discards
custom edges and restores the layout tree; selecting Independent again reseeds
its defaults. Save custom settings to a slot before changing modes to retain them.
Three-way independent settings, overlaps and gaps have no flat-sum guarantee.
For matched two-way LR12/LR36, invert one branch manually; LR24/LR48 use the same
polarity. See [all polarity and phase rules](DSP.md#independent-edges-d08).

### Existing presets and library migration

Loading processing v1/v2 or library/working envelope v1 fails with an explicit
migration instruction. Stop editors before migrating library files. Convert to
new destinations, preserving the originals:

```sh
./target/release/shr-pa migrate old-v2.json new-v3.json
./target/release/shr-pa check new-v3.json
# v1 processing is also accepted directly and converted to v3.
./target/release/shr-pa migrate old-v1.json migrated-v3.json
# Slot and working envelopes use the same command:
mkdir -p migrated-session/.shr-pa
./target/release/shr-pa migrate old-session/.shr-pa/U1.json migrated-session/.shr-pa/U1.json
./target/release/shr-pa migrate old-session/.shr-pa/working.json migrated-session/.shr-pa/working.json
```

Repeat for each occupied U1…U75 slot, keeping the same filename. Missing/empty
slots need no file, and templates are compiled into the app. Do not copy the old
lock or interrupted temporary files. Start the offline editor from the migrated
session directory, or place the migrated standalone live preset beside its
`.shr-pa` directory. Recovery still requires compatible rate/block settings and
starts muted. Original files and rejected recovery remain available for rollback.
Failed validation creates no destination; inspect any migration error before
starting the new session. Existing destinations are never overwritten.

V2 processing migration adds `crossover: "layout_lr24"`, preserving every old
value and the original audible behavior. V1 first gains bell types, S=1, enabled
PEQs, flat/bypassed GEQ and bypassed compression, then converts to v3. Envelope
migration converts both saved and working baselines and retains their EQ restore
histories, names and selection. Unknown/missing/incompatible state is rejected.
Changing only the version number is not a migration.

## Six-channel offline artifacts

```sh
./target/release/shr-pa render "$SESSION/preset.json" impulse "$SESSION/impulse.wav" 1 --unmute
./target/release/shr-pa render "$SESSION/preset.json" sweep "$SESSION/sweep.wav" 5 --unmute
./target/release/shr-pa render "$SESSION/preset.json" noise "$SESSION/noise.wav" 5 --unmute
./target/release/shr-pa render "$SESSION/preset.json" pink "$SESSION/pink.wav" 5 --unmute
./target/release/shr-pa render "$SESSION/preset.json" sine:1000 "$SESSION/tone.wav" 1 --unmute
./target/release/shr-pa render "$SESSION/preset.json" stereo-input.wav "$SESSION/processed.wav" 60 --unmute
```

Output is a six-channel IEEE float WAV, ordered **H-L, H-R, M-L, M-R, L-L, L-R**.
No device mapping affects offline rendering. Input WAV must be stereo at the
preset rate; PCM integer and float input are supported. Rendering stops at EOF
or the requested duration; append silence to the input to retain a complete
filter/delay tail. Input and output files must differ. There is no resampling.

Generators are deterministic, with a default 0.1 (−20 dBFS) peak bound before processing;
noise RMS and observed peaks are lower. The impulse has a 10 ms lead-in to clear
the startup ramp. Sweep is logarithmic from 20 Hz to 0.4 × sample rate. `noise` is
seeded white noise; `pink` approximates equal power per octave using 16 octave
rows plus white noise. Both feed identical L/R samples. Pink noise has spectral
ripple and finite-record mean variation; it is not calibrated measurement noise.
Select a source peak bound with `--level=DBFS` (finite −60…0, default −20):

```sh
"$SHR_PA_BIN" render "$SESSION/preset.json" pink "$SESSION/pink-quiet.wav" 2 --unmute --level=-40
"$SHR_PA_BIN" live "$SESSION/preset.json" null null 2 2 0,1 0,1,-,-,-,- 1 --signal=pink --level=-40 --unmute
```

The option applies only to generated sources; WAV input and live capture without
`--signal` reject it. It sets a peak bound before input gain/EQ/dynamics, not noise
RMS or calibrated physical level. Sine/impulse peaks use the same dBFS convention.
Source and level are session-only; saving or recovering processing never restores
them. During `live --ui`, `(` lowers and `)` raises the source level by 1 dB,
clamped to −60…0. The meter header shows the desired target; each edit returns
to that view. A 5 ms linear gain ramp starts at the next serviced block boundary,
retargeting from the current gain during rapid edits. Processing pending/busy
indicators describe processing transactions, not this independent ramp. Capture-only
sessions reject these keys; recalls leave the runtime target unchanged.

In an explicit `--signal` session, `~` toggles that generator off/on. Off restores
both mapped capture channels through a 5 ms linear crossfade at the program-input
insertion point. **Off restores program audio; it does not mute outputs.** Use
`m` to mute all outputs. Turning it on crossfades back to the selected source.
The header shows the desired `Gen ON`/`Gen OFF` state and generator level target,
including while off; that level never scales capture. `(`/`)` can edit it while
off. Rapid toggles retarget from the current mix; phase and noise history keep
advancing even while off. Impulses are not retriggered and sweeps are not restarted.
Recall leaves on/off and level unchanged; restart without `--signal` has no
source to enable. UI startup remains muted. Source-type changes during a session
remain pending. See the
[generator contract](DSP.md#test-generators-m03).
Timing reports measure just `Engine::render`, excluding WAV I/O and generation.

## Software-null streaming (no hardware)

With a fresh preset, this bounded command exercises the live backend and generator
using only ALSA's software `null` PCM:

```sh
./target/release/shr-pa live "$SESSION/preset.json" null null \
  2 2 0,1 0,1,-,-,-,- 1 --unmute --signal=sine:1000
python3 scripts/check-live-controls.py target/release/shr-pa
```

For interactive practice, replace `1 --unmute --signal=sine:1000` with
`86400 --ui --signal=sine:1000`; press `u` after pending/busy clears, edit modules,
then `m` and `q`. Null PCM has no USB clock and can run faster than real time;
duration counts program frames. It provides no physical timing or sound evidence.

## Direct ALSA: explicit hardware and channel selection

```sh
./target/release/shr-pa devices
# Replace CARD_ID using the device inspection above; no card number is assumed.
./target/release/shr-pa live "$SESSION/preset.json" \
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

Capture/playback must share one card/clock; the operator must select matching
endpoints because the backend does not verify a shared clock domain. The backend independently negotiates
native formats and actual period/buffer sizes, requires the requested rate and
channel counts, and prints the results. Supported conversion formats are S32_LE,
S24_3LE, S16_LE and FLOAT_LE (in preference order). Raw `hw:` endpoints avoid
implicit plugins, resampling or device fallback. `null` is available only when
explicitly named for software transport experiments; it is not hardware evidence.
The development request is 128 frames/period and four periods/buffer at 48 kHz;
read the actual negotiated values. Smaller settings are not qualified.

Live starts muted. With `--ui`, use the controls above; Tab opens modules. It displays actual
logical peaks and physical mappings. Without `--ui`, use `--unmute` explicitly
for a bounded run. Optional `--signal=sine:1000`, `--signal=noise`, `--signal=pink`, etc. replaces
program input before processing and ends with the session; it is never recalled
from a preset. Without that option, the mapped physical capture feeds the DSP.

```sh
# Explicit, bounded hardware experiment; keep the external system at a known level.
./target/release/shr-pa live "$SESSION/preset.json" \
  hw:CARD=CARD_ID,DEV=0 hw:CARD=CARD_ID,DEV=0 \
  2 2 0,1 0,1,-,-,-,- 10 --unmute --signal=sine:1000
```

Startup primes ALSA with silence. Partial I/O advances by frames; EINTR/EAGAIN
are retried with bounded waits, and no progress for two seconds is an error.
Normal/keyboard/signal shutdown ramps mutes and flushes silence before dropping
both streams. Xruns, suspend, disconnect and numerical faults end the session,
drop both streams and require an **explicit restart** (which renegotiates and
starts muted). This first backend does not reconnect or resume sound automatically.
A busy device is a local error; offline DSP remains usable. Leave existing audio
services alone during offline/null validation. The dated AudioBox trials found
JACK owning the card; that is not a statement about the current host. Hardware
access needs a separately arranged session. SHR PA does not stop other applications
or services automatically. Software cannot
promise analog silence when USB, process or power fails.

## Three separate limits

1. **Logical engine:** two inputs and six outputs, regardless of hardware width.
2. **Recorded hardware (2026-09-29):** the AudioBox reports two capture/two playback channels;
   only an explicitly selected output pair can be played at once.
3. **Recorded measurement connections (2026-09-29):** no analog loopback and amp off, confirmed
   by the user. Processing time, sample peaks, negotiated buffers and xruns were
   measured. Analog round-trip latency, socket/voltage calibration, acoustic
   response and speaker protection were not measured.

See [DSP behavior](DSP.md), [status](STATUS.md) and the
[bench record](verification/0003-engine.md). UMC1820 qualification is future
acceptance work and does not gate engine development.

## Backup, recovery and rollback

| Location | Contents / ownership |
| --- | --- |
| `preset.json` offline; CLI preset path live | Explicit processing import/export, schema v3 |
| `.shr-pa/U1.json` … `U75.json` | Saved names, processing and EQ history; envelope v2 |
| `.shr-pa/working.json` | Desired edits, histories, selection and saved baseline; envelope v2 |
| `.shr-pa/working-rejected-*.json` | Preserved rejected recovery bytes after explicit reset |
| `.shr-pa/lock` | Editor lock; not a preset or recovery record |

Live UI uses `.shr-pa` beside its CLI preset; offline uses the current directory.
Stop editors before copying the standalone preset and the whole `.shr-pa` directory
for backup. Keep the old executable/revision with its original files for rollback;
older applications cannot read newer schemas. Migrate copies into a new session as
shown above. Do not replace originals by editing version numbers.

| Symptom | Action |
| --- | --- |
| `NO STORAGE` | Close the other editor or fix the reported directory/permission error, then restart. The OS releases a lock on exit/crash; deleting its file is not an unlock procedure. |
| `Recovery BLOCKED` | Preserve the file; migrate supported legacy state into a new session, or use `:recover-reset` to archive it and checkpoint current edits. |
| `Unsaved` | Resolve storage failure and make/checkpoint another edit; do not assume the last edits survived a restart. |
| Pending/busy | Wait for the latest desired transaction to settle, then retry unmute. Saving still records desired settings. |
| Rate/block recall rejected | Start a compatible session with an explicit standalone preset; the offline editor's startup defaults are 48 kHz/128. |
| Stream/numerical fault | End the session, investigate the error and explicitly restart. Recall/recovery/unmute cannot clear it. |

A terminal left damaged after an unhandled kill can be restored with `stty sane`
and `reset` in that terminal. Handled exits restore it automatically. A software
mute and terminal cleanup do not establish physical silence after process failure.
