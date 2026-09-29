# Contributing

This project has a working first DSP/offline/ALSA slice. Read [status](docs/STATUS.md) and the
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
