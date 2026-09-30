# Contributing

The 0.2 alpha baseline includes offline DSP, explicit ALSA streaming, live controls,
a local preset library and working recovery. Read [status](docs/STATUS.md) and the
[roadmap](docs/ROADMAP.md) before extending it. Keep proposed features distinct
from implemented and hardware-verified behavior.

Use the pinned Rust toolchain and commit `Cargo.lock`. Run the normal commands in
[validation](docs/VALIDATION.md). Audio work must have offline correctness tests
and explicit hardware evidence before claiming latency or protection guarantees.
Keep DSP allocation-free and bounded; keep terminal and storage work off its thread.

Keep the current 2×6 scope separate from future routing and measurement work.
Do not infer the pending nine-channel layout. Avoid machine-specific card numbers,
absolute user paths or sibling-repository build dependencies. Keep bench data and
large audio files outside Git under `artifacts/`.

Bug reports should include revision, OS/kernel, hardware, exact reproduction and
expected/observed behavior. Audio reports should also include negotiated stream
settings and whether the signal chain was physically verified.

## Documentation and releases

Use [DSP.md](docs/DSP.md) for current contracts, [RUNNING.md](docs/RUNNING.md)
for operations, and the [function map](docs/DRIVERACK_MAP.md) for requirements.
Keep verification records dated; add a supersession note instead of rewriting
old measurements as new evidence. Link new evidence from the documentation index.

Application SemVer lives in `Cargo.toml` and the `shr-pa` entry in `Cargo.lock`.
Processing and persistence schema versions change only with their contracts.
For a release, update [release notes](CHANGELOG.md), verify `--version`, run all
normal validation and review the diff before committing and pushing. Check the
upstream commit after pushing; report remote CI separately from local results.
