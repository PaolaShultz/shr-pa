# 0001 — standalone synchronous audio direction

Status: historical foundation (2026-09-29). Topology is superseded by
[decision 0002](0002-driverack-2x6.md); hardware ordering and transport status by
[decision 0003](0003-hardware-independent-engine.md). The text below records the
original scaffold, not current implementation status.

The project uses Rust on Raspberry Pi 5, with a compact terminal UI. Nearby SHR
projects established the Rust 1.97.1 toolchain, MIT licensing, offline entry points
and 40×13 terminal convention. This scaffold follows those conventions without
importing their engine, launcher or JACK dependency.

The first audio transport to evaluate is direct ALSA with one synchronous DSP
thread. The interface is a single UMC1820; stream negotiation and loopback
measurements precede choosing a supported operating buffer size. No additional
asynchronous audio-stage pipeline is planned.

Channel topology is intentionally open. Neither eight setup microphones nor the
later nine-channel requirement implies a fixed program feed, output count or
simultaneous measurement workflow. Further user content defines these roles.

The initial implementation is an offline shell so the repository can be built,
reviewed and documented before hardware streaming and DSP are introduced.
