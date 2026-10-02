![SHR PA — signal paths, precisely connected](docs/assets/banner.svg)

# SHR PA

**0.2.0-alpha.1 (“0.2 alpha”) — experimental Rust PA management for Raspberry Pi 5 and Linux.**

[Release notes](CHANGELOG.md) · [Documentation index](docs/README.md)

[![Rust checks](https://github.com/PaolaShultz/shr-pa/actions/workflows/ci.yml/badge.svg)](https://github.com/PaolaShultz/shr-pa/actions/workflows/ci.yml)

[Run commands](docs/RUNNING.md) · [Status](docs/STATUS.md) · [DSP behavior](docs/DSP.md) · [Roadmap](docs/ROADMAP.md) · [Hardware](docs/HARDWARE.md)

SHR PA is a hardware-independent **2-input × 6-output** processor:
LR24 full-range/two-way/phase-compensated three-way layouts, mono selection/bass
summing, independent BW6–48/LR12–48 crossover edges, 31-band GEQ, bell/shelf PEQ,
stereo-linked compression, delays,
gain/polarity, ramped mutes, linked peak limiters and meters.
Six-channel offline WAV rendering works without an audio interface, including
seeded white and pink noise sources with explicit −60…0 dBFS generator levels. Direct ALSA
runs selected logical outputs on the physical channels available today, with no
implicit stereo mixdown. Explicit live generators also support `(`/`)` level edits
with a 5 ms ramp. `~` turns the selected generator off/on, crossfading to/from
mapped capture without restarting its sequence.

The 2026-09-29 AudioBox USB 96 trials exercised the stereo transport path. **UMC1820 is a future
target, not a development prerequisite.** Analog latency and speaker protection
are unmeasured; no loopback is connected. See the [bench record](docs/verification/0003-engine.md).

## Build and run

Requires Linux, Rust **1.97.1** via rustup, a C linker, `pkg-config` and ALSA headers
(`libasound2-dev` on Debian/Ubuntu).

```sh
cargo build --release --locked
./target/release/shr-pa --version
# Use a new session directory: init replaces an existing destination.
mkdir -p artifacts
SESSION=$(mktemp -d "$PWD/artifacts/session-XXXXXX")
./target/release/shr-pa init "$SESSION/preset.json"
./target/release/shr-pa check "$SESSION/preset.json"
./target/release/shr-pa render "$SESSION/preset.json" sweep "$SESSION/six-outputs.wav" 5 --unmute
(cd "$SESSION" && ../../target/release/shr-pa)
```

The default terminal is an offline preset editor with real DSP previews. It opens
no audio device. A fresh session starts with the default three-way preset; press
`l` to import an existing `preset.json`. Use Tab/arrows and footer mouse buttons for pages; `q` exits.
`s`/`l` save/load `preset.json`; `r` renders a preview; `1`…`6` toggle logical mutes.
The compact layout is **40×13**. Terminal state is restored on handled exits.

![Offline engine editor](docs/assets/terminal.svg)

## Explicit live operation

```sh
./target/release/shr-pa devices
# Substitute the selected ALSA card ID. Starts muted; u unmutes, q stops.
./target/release/shr-pa live "$SESSION/preset.json" \
  hw:CARD=CARD_ID,DEV=0 hw:CARD=CARD_ID,DEV=0 \
  2 2 0,1 0,1,-,-,-,- 10 --ui
```

Maps are zero based. The six entries select physical outputs for
H-L/H-R/M-L/M-R/L-L/L-R; `-` leaves a logical output unmapped. All six DSP outputs
still run and remain renderable offline. Invalid physical mappings fail locally.
Live controls use prepared transactions and bounded transitions for gain, polarity,
EQ, dynamics and delays. Tab opens module controls; `v` selects a module, `n` a
parameter and `x`/`X` edits it. Layout/recall uses mute/reconfigure/resume.
Presets use schema v3; `migrate OLD NEW` explicitly converts v1/v2 files and old
library/working envelopes, retaining source files and EQ history. [Full instructions](docs/RUNNING.md).

## Remaining scope

The [complete PA2 function inventory](docs/DRIVERACK_MAP.md) still tracks automatic EQ
restore sources, extended limiter modes, subharmonic synthesis, feedback suppression,
measurement/RTA, AutoEQ/setup workflows, speaker profiles, remote controls and maintenance.
The local library, working recovery, manual EQ restore and crossover controls are
[implemented](docs/STATUS.md).
The full PA2 function plan remains incomplete.
There is no claim of proprietary dbx algorithm equivalence.

The requested [phase/delay alignment workflow](docs/PHASE_ALIGNMENT.md) will add
reference/mic measurement and guided delay/polarity decisions. Develop the PA
module here for standalone use and later integration into GigPies; measurement,
automatic alignment and that integration remain pending.

General matrices, advanced routing, eight-point positional RTA and the later
nine-channel arrangement remain [future work](docs/FUTURE.md).

[Validation and test policy](docs/VALIDATION.md) · [Contributing](CONTRIBUTING.md)

Built independently of the related SHR projects, including
[SHR DAW](https://github.com/PaolaShultz/shr-daw).
MIT licensed. Documentation artwork is original. Vendor marks belong to their
owners; SHR PA is an independent project.
