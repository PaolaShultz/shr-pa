# Phase-alignment plan and publication boundaries — 2026-10-03

The new [phase-alignment task](../PHASE_ALIGNMENT.md) is linked from the working
agreements, roadmap, function map and status. SHR PA owns development of the PA
module intended for later GigPies integration. Measurement and automatic alignment
remain pending; application version stays **0.2.0-alpha.1** and DSP is unchanged.

Publication protection now includes ignored user/artifact/media paths, a reviewed
script list, Git index/outgoing-history checks, local hooks and a CI check.
See [publication rules](../PUBLICATION.md) for installation and limits.

Validation used Rust 1.97.1, the committed lockfile and `CARGO_INCREMENTAL=0`:

- All 58 normal Rust tests passed, plus formatting, warning-denied Clippy and
  the locked release build. The executable reports 0.2.0-alpha.1.
- Offline terminal checks and software-null live-control checks passed.
- Four Python publication regressions passed, including forced private additions,
  staged secrets, renamed media/symlinks and a leak deleted at the branch tip.
- Documentation/link/SVG/version checks passed. Complete staged content and the
  existing commit history passed the publication guard; no tracked file matches
  private/generated ignore rules. Synthetic ignore probes passed.

No physical audio, host configuration change, new DSP evidence render, benchmark,
artwork regeneration or acoustic test was run. Existing physical limitations remain.
The guard and local checks do not establish passing remote CI; check the pushed
revision separately. No recordings, private user state or generated artifacts are
part of this documentation/publication change.
