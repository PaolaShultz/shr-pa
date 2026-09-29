# Current status

## Implemented

- Standalone Rust executable and library for the small terminal layout.
- Three offline pages: project, planned features, target hardware.
- Keyboard and terminal mouse navigation; small-terminal fallback.
- Plain text snapshots, help/version and explicit invalid-command errors.
- Terminal cleanup on ordinary exit and handled SIGINT/SIGTERM/SIGHUP.
- Fast layout/CLI regressions and a pseudo-terminal lifecycle check.
- Linux x86-64/ARM64 CI configuration, MIT license and project documentation.

## Not implemented

ALSA audio, DSP, routing, meters, configuration/presets, direct touchscreen
integration, MIDI, measurement, automatic EQ, feedback suppression or service
installation. The interface has no live controls and opens no audio devices.

## Hardware and performance

The foundation is built/tested on the target Pi 5. UMC1820 hardware validation,
physical touchscreen validation, latency measurements and live PA tests remain
pending. There are no fabricated meters, measurements or benchmark numbers.

## Open details

The nine-channel operating roles, final matrix topology, physical screen/input
integration, signal levels and speaker protection limits await further project
content. They do not block this scaffold and are not filled in with assumptions.
