# Beyond the initial 2×6 system

## Matrix foundation delivered — 2026-10-05

[C-PA v2](EMBEDDING_V2.md) now implements dynamic program inputs, explicit
weighted mono routing and independent protected outputs, including compensated
LR24 stereo three/four-way and 4×8 references. Existing standalone fixed presets
and v1 bytes/semantics remain supported. This supersedes earlier fixed-2×6-only
ordering and deferral of all matrix/configurable embedding below. Scope here is
software implementation; physical I/O, acoustic/true-peak protection and
measurement remain unqualified or unavailable as documented.


These are future project directions. They are not prerequisites for the
[DriveRack function plan](DRIVERACK_MAP.md).

- General matrix mixing and flexible crossover patching, including the earlier
  4×8 example. Final dimensions and graph behavior remain to be defined.
- Advanced routing beyond the fixed full-range/two-way/three-way configurations.
- Positional RTA using eight measurement points, including the possibility of
  eight simultaneous measurement microphones during venue setup. Position
  metadata, calibration, timing and spatial combination need their own design.
- The later nine-channel operating arrangement, whose roles will be supplied later.
- Additional controllers such as MIDI where useful.

The function plan has two program inputs, six outputs and one separate
measurement input. The measurement input is not implemented in 0.2 alpha.
Ordinary PA2-style sequential measurement with one microphone is part of that plan. An eight-point analysis system is future work.

Keep future expansion possible through clean module boundaries. Do not require a
general graph compiler, matrix editor or multichannel measurement engine before
the fixed 2×6 system works.
