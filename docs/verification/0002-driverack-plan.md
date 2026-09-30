# 2×6 planning update verification — 2026-09-29

> Historical evidence for the dated slice below. Test counts, pending features and
> commit/push statements describe that moment. See the [evidence index](README.md)
> for subsequent commits and the [0.2 alpha record](0007-0.2-alpha.md) for release checks.

Scope: documentation, original illustrations and offline terminal labels. No DSP,
audio device, host configuration or live controls were added.

## Function audit

Reviewed the manufacturer's PA2 manual through the Full Compass PDF mirror,
including the feature list, connector/control descriptions, home screens, wizard
options, manual commissioning, presets, every processing chapter, utilities,
power-up functions, application/preset families, remote operation/networking,
updates, block diagram and hardware specifications. The official manual endpoint
returned HTTP 403; the source and mirror links are retained in DRIVERACK_MAP.md.

The resulting map has 42 unique IDs: 4 configuration, 11 processing, 9 measurement
and setup, 13 operating/system, and 5 hardware entries. Each identifies an
implementation stage and acceptance evidence. Hardware/proprietary differences
are explicit. Matrix routing and eight-point positional measurement are future scope.

## Checks run on the Pi

- Focused layout tests passed after the label changes.
- Complete normal Rust suite passed: 3 unit and 2 CLI tests.
- Seven pseudo-terminal lifecycle cases passed.
- Formatting, Clippy with warnings denied and native release build passed.
- Local Markdown targets, SVG XML and map-ID uniqueness checked.
- Documentation artwork regenerated and rendered previews visually inspected.

No new tests were added for text-only changes. The existing layout/CLI/terminal
regressions protect the affected behavior. Hardware, acoustic, long-soak and
exhaustive DSP tests were intentionally not run: this update only plans those
functions, and their implementations remain pending. The artwork renderer was
run explicitly because its diagrams changed.
