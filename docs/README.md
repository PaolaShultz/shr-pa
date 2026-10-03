# SHR PA documentation

Current release: **0.2.0-alpha.1 (“0.2 alpha”)**. Start with the offline editor or
software-null examples; hardware qualification is a separate activity.

## Operate and understand

- [Running](RUNNING.md): build, first edit, controls, file locations, migration,
  software-null practice, explicit physical mappings and operational recovery.
- [Status](STATUS.md): implemented offline/live behavior, dated physical evidence
  and remaining scope.
- [DSP contract](DSP.md): processing order, crossover phase/sums, parameter ranges,
  transitions, faults, schemas and persistence ownership.
- [Architecture](ARCHITECTURE.md): implemented signal path and thread boundaries,
  followed by planned extensions and acceptance targets.
- [Hardware](HARDWARE.md): dated AudioBox observations, UMC1820 sources and future
  physical qualification. No analog loopback or protection evidence is implied.

## Develop and release

- [Contributing](../CONTRIBUTING.md) and [working agreements](../AGENTS.md).
- [Publication boundaries](PUBLICATION.md): private data, reviewed scripts and Git hooks.
- [Embedding](EMBEDDING.md): versioned C ABI, native f64 buffers, protection and host ownership.
- [Validation](VALIDATION.md): normal production checks and opt-in research/hardware work.
- [Release notes](../CHANGELOG.md) and [0.2 alpha verification](verification/0007-0.2-alpha.md).
- [Evidence index](verification/README.md): all dated records and their scope.
- [Pink generator verification](verification/0008-pink-noise.md): deterministic M03 source.
- [Generator level verification](verification/0009-generator-level.md): session-start level selection.
- [Runtime generator level verification](verification/0010-runtime-generator-level.md): live level edits and bounded ramps.
- [Generator off/on verification](verification/0011-generator-capture-restore.md): bounded capture restoration and source continuity.
- [DriveRack function map](DRIVERACK_MAP.md): complete 42-ID PA2 target inventory.
- [Roadmap](ROADMAP.md): feature groups, remaining requirements and exit criteria.
- [Phase and delay alignment](PHASE_ALIGNMENT.md): pending reference/mic measurement,
  delay/polarity decisions, validation and eventual GigPies module integration.
- [Future scope](FUTURE.md): general routing, eight-point positional RTA and the
  undefined later nine-channel arrangement.

## Decisions

- [0001 — foundation](decisions/0001-foundation.md), with superseded assumptions marked.
- [0002 — fixed 2×6 and complete PA2 scope](decisions/0002-driverack-2x6.md).
- [0003 — hardware-independent logical engine](decisions/0003-hardware-independent-engine.md).
