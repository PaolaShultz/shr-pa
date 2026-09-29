# Validation

## Normal checks — run now

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked
python3 scripts/check-terminal.py target/release/shr-pa
```

The Rust suite checks page bounds/status, touch/navigation consistency, resize
behavior, headless CLI output and rejection of unsupported commands. The Linux
pseudo-terminal check covers keyboard navigation, touch exit, Ctrl+C, termination
signals, small-terminal recovery and restoration of terminal attributes. It uses
no audio device. These are fast production regressions and belong in the default
validation path and CI.

Run the full normal suite for shared engine, render, model, routing, persistence,
concurrency or safety changes, and before publication. During implementation,
start with the focused checks for the changed behavior.

## Later normal tests

Add deterministic DSP response tests, fixed-configuration/schema tests, finite-value and
protection invariants, bounded command handoff and recovery regressions when
those components exist. Add allocation checks around real render work. Compare
whole 2×6 chain output against references, including multiway crossover sums.

Use [function-map IDs](DRIVERACK_MAP.md) to link each future test/evidence record
to the planned capability. Include all fixed configuration families, inactive
outputs, single-mic isolation, full processing load, parameter/bypass transitions,
setup cancellation, presets/global-state ownership and local/remote command races.
Advanced matrix and eight-point measurement tests belong to future work.

## Opt-in bench and research classes — not implemented yet

Hardware streaming, exhaustive parameter sweeps, long thermal/latency soaks,
feedback auditions and measurement renderers must be explicit opt-in commands.
Document each runnable command when its harness lands. Preserve results and
provenance; keep one-time research out of the normal suite unless it protects
current production behavior. Never represent absence of hardware tests as a pass.

Planned hardware record: board/OS/kernel/firmware, USB topology, device identity,
clock source, rates, native formats, channel map, period and buffer sizes,
processing configuration and software revision, temperature/throttling, xruns, maximum and percentile
render time, round-trip latency distribution and observed transients.

Initial proposed acceptance: repeated startup/recovery; then an eight-hour
full-load duplex soak with zero xruns, no throttling and the chosen processing
margin. Exercise UI and analysis load during the soak. Report the test duration
and actual workload with every result. A passing soak is evidence under those
conditions, not a guarantee of flawless operation in every venue.

## Artwork

```sh
python3 scripts/render-docs.py target/release/shr-pa
```

Regenerates original SVG artwork and a terminal illustration from actual snapshot
text. This is an opt-in documentation renderer; it is not a runtime test.
