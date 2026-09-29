![SHR PA — signal paths, precisely connected](docs/assets/banner.svg)

# SHR PA

**A Rust PA management system taking shape on Raspberry Pi 5.**

[![Status: highly experimental](https://img.shields.io/badge/status-highly_experimental-e6ac55)](docs/STATUS.md)
[![MIT license](https://img.shields.io/badge/license-MIT-65d6c4)](LICENSE)
[![Target: Raspberry Pi 5](https://img.shields.io/badge/target-Raspberry_Pi_5-c51a4a)](docs/HARDWARE.md)
[![Rust checks](https://github.com/PaolaShultz/shr-pa/actions/workflows/ci.yml/badge.svg)](https://github.com/PaolaShultz/shr-pa/actions/workflows/ci.yml)

[Roadmap](docs/ROADMAP.md) · [Architecture](docs/ARCHITECTURE.md) · [Hardware](docs/HARDWARE.md) · [Development](CONTRIBUTING.md)

> **Highly experimental. Scaffold stage.** The executable is an offline terminal
> shell. Audio processing, device streaming, and speaker protection are not
> implemented. There is no measured latency result yet.

## The direction

First, plan and build the DriveRack PA2 functions in a simple **2-input,
6-output** processor. Use Rust, Raspberry Pi 5, a Behringer UMC1820 and a small
touchscreen terminal on 64-bit Linux Lite.

- **Processing:** LR24 first, graphic/parametric EQ, compression, subharmonic
  synthesis, mono bass, input/output delays and limiters.
- **Setup and measurement:** one separate measurement mic, RTA, noise generation,
  level balancing, AutoEQ, feedback suppression and guided/manual setup.
- **Operation:** presets, speaker/amplifier profiles, meters, mutes, startup
  behavior, lockout/reset, remote terminal control and software maintenance.

The [full function map](docs/DRIVERACK_MAP.md) includes source references,
control ranges, implementation stages and acceptance checks. These are planned
capabilities, not working DSP or a claim of identical proprietary algorithms.

**Future:** matrix mixing, advanced routing, eight-point positional RTA and the
later nine-channel arrangement. See [future scope](docs/FUTURE.md); these do not
block the initial 2×6 system.

![Planned 2-input, 6-output processing; DSP is not implemented](docs/assets/signal-flow.svg)

## Try the scaffold

Requires Rust **1.97.1** through rustup, a C linker, and Linux. No audio development
headers are needed at this stage. Ensure `~/.cargo/bin` is on `PATH`.

```sh
cargo build --release --locked
./target/release/shr-pa

# Plain text previews work without a terminal or audio hardware.
./target/release/shr-pa --snapshot
./target/release/shr-pa --snapshot features
./target/release/shr-pa --help
```

Use arrows or Tab to change pages; `q`, Escape or Ctrl+C exits. Footer buttons
accept terminal mouse events. Touch needs a terminal/console bridge that emits
those events; raw touchscreen input is not implemented. The initial layout is
**40×13**, following the compact SHR audio projects. Smaller terminals show a
resize message. The application restores terminal state on normal exit and
handled termination signals.

![Actual scaffold text rendered as an illustration](docs/assets/terminal.svg)

No audio/MIDI connections, configuration files, services, or system settings are
created by running the shell. The screen above comes from `--snapshot`; it is
not a live audio display.

## What comes next

1. Qualify the actual UMC1820 on this Pi: formats, channel map, clocking and duplex timing.
2. Build the direct ALSA streaming harness and measure stable buffer sizes.
3. Implement and verify the fixed 2×6 processing chain and protection.
4. Work through the full function map: setup, measurement, processing and operation.

See the [implementation roadmap](docs/ROADMAP.md) for algorithms, dependencies,
and acceptance criteria, and [validation](docs/VALIDATION.md) for test classes.

## Related projects

This follows the Rust, Linux and terminal conventions used in
[SHR DAW](https://github.com/PaolaShultz/shr-daw),
[SHR FX](https://github.com/PaolaShultz/shr-fx), and
[SHR Rec](https://github.com/PaolaShultz/shr-rec).
It builds independently of those repositories.

MIT licensed. Original documentation artwork is included under the same license.
dbx, DriveRack, Behringer and Raspberry Pi are their respective owners' marks;
SHR PA is an independent project.
