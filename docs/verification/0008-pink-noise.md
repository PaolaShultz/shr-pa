# M03 pink generator — 2026-09-30

Started from clean `main` at `e5344872d2281c491b314737c6b32360b8f932ae`, matching
`origin/main` after fetch. Baseline CI run `36712026099` was confirmed successful.
Read the working agreements, current documentation, historical verification and
source/tests before choosing this bounded generator slice. Existing preset/EQ
recovery and crossover controls remain implemented.

## Implemented

- Deterministic `pink` offline source and explicit live `--signal=pink`, feeding
  the existing program-input insertion point and all six logical output chains.
- Voss-McCartney octave rows plus white noise, fixed storage, exact integer row
  sum, at most two random draws per sample, 0.1 peak bound before f32 rounding.
  See [DSP contract and attribution](../DSP.md#test-generators-m03).
- Reproducible stereo sequence independent of block size and sample rate. Existing
  white sequence, source levels, development defaults and physical mapping remain.
- CLI help, operating instructions, function inventory and status now identify
  the implemented source and remaining M03 controls.

Application stays `0.2.0-alpha.1`; processing schema v3 and library/working
envelopes v2 are unchanged. No release or tag is created.

## Validation

Focused generator, CLI, WAV/contract and allocation regressions passed before the
complete normal suite. New production checks cover:

- Exact sequence agreement at 8/44.1/48/96/192 kHz, short/empty fills, four counter
  wraps, identical L/R, finite bounded samples and a broad RMS sanity range.
- Independent Hann-windowed Goertzel probes over nine octaves, with a white-noise
  control. Eight 32,768-frame windows cover octave edges from rate/2048 to rate/4.
  The pink octave-power spread must be under 3 dB; white must rise 21–27 dB over
  the eight-octave separation. These are statistical source checks, not calibrated
  physical response or a full-band tolerance guarantee.
- Deterministic six-channel WAVs, CLI output compared sample-for-sample with a
  direct generator/engine reference, including the short final block.
- Explicit software-null live runs with and without unmute; muted startup stays
  silent and an unmuted run produces logical output without faults.
- Zero allocations/deallocations during pink generation and six-output processing
  through two counter wraps. Existing render/transaction/fault checks remain.

Native aarch64, pinned `rustc 1.97.1 (8bab26f4f 2026-07-14)`:

| Check | Result |
| --- | --- |
| `python3 scripts/check-docs.py` | Pass: local links/anchors, SVG XML and version consistency |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | Pass: 49 production tests, zero failed/ignored |
| `cargo build --release --locked` | Pass |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Pass: lifecycle, navigation, 40×13 recovery and persistence checks |
| `python3 scripts/check-live-controls.py target/release/shr-pa` | Pass: software-null live edits, library/EQ/crossover, mutes and muted recovery |
| Release CLI examples in fresh temporary paths | Pass: version/help, three 40×13 snapshots, six generators, WAV metadata, stereo EOF and source-preserving v2 migration |
| Explicit one-second `live ... null null ... --unmute --signal=pink` | Pass: 48,000 program frames, zero xruns, no fault; all six logical output peaks nonzero |
| Final diff and `git diff --check` | Reviewed; no whitespace errors |

The 49 tests comprise 3 UI, 4 CLI, 8 contract, 5 crossover, 7 engine, 2 generator,
7 library, 10 live-processing and 3 allocation tests. The pink spectral test takes
about 0.8 seconds locally. The workload example was compiled, not benchmarked.
Release artifacts used temporary directories and were removed after inspection.
Null streaming is integration evidence; its timing is not a hardware measurement.

## Limits and skipped classes

No physical PCM was opened; JACK and host audio settings were untouched. Hardware
soaks, exhaustive research, workload benchmarks, acoustic auditions and disposable
artwork renderers were intentionally skipped. No slow historical default tests
needed reclassification. The new spectral check protects current source behavior
and takes about one second locally; it belongs in the normal suite.

Pink noise has spectral ripple, finite low-frequency coverage and finite-record
mean variation. The fixed level is a peak bound, not a measured/calibrated RMS.
M03 remains partial: adjustable level, runtime source switching/stop with capture
restoration, RTA, setup mic and measurement workflows are pending. Existing session
shutdown/fault handling still applies; sources are never persisted or recovered.
No new analog response, latency, transients, protection, soak or UMC1820 evidence
is claimed. All earlier physical observations retain their dated workload limits.

## Publication

This record is included in the implementation commit on the existing `main`
branch. Publication compares the pushed remote revision with the local commit
and checks both x86-64 and ARM64 CI jobs. The final task report gives that commit,
remote match and CI result separately; the local results above do not establish
remote CI success. No application version bump or release/tag was needed.
