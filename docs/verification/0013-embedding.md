# Native f64 PA embedding — 2026-10-03

Base: `cf1a2b33668d6ee258487be8c865ba163c0a8937`, initially clean. This scoped
SHR PA change supports the separately owned GigPies hardware integration task.
Application `0.2.0-alpha.1`, processing schema v3 and working/library v2 remain
unchanged. No publication to GitHub, release or hardware test occurred here.

## Implemented behavior

The [version 1 C ABI](../EMBEDDING.md) prepares the existing FullRange engine,
processes native f64 stereo input into six f64 logical outputs, reports zero
fixed algorithmic delay, and destroys its opaque state. Logical 0/1 carry
unity-gain L/R through the existing −1 dBFS linked sample limiter and 5 ms ramp;
2–5 are silent. A new f64 render entrypoint shares all processing with the
legacy f32 boundary. No alternate PA algorithm or sibling dependency was added.

Arguments are bounded and obvious null, alignment, integer-overflow and buffer
overlap errors are rejected before output/state changes. Foreign callers still
own allocation validity, lifetime and exclusive access. Numerical faults silence
the entire affected block and latch; destroy/create is the explicit reset.
Initialization/destruction occur outside the audio worker. Allocation-counting
regressions include valid blocks, fault handling and argument rejection.

## Checks

Native aarch64, Rust 1.97.1, committed Cargo.lock and `CARGO_INCREMENTAL=0`.
Focused ABI/allocation checks preceded the complete normal production suite.

| Check | Result |
| --- | --- |
| `cargo test --locked --test ffi --test render_allocation` | 10 passed |
| `cargo test --locked --all-targets` | 65 passed, zero failed/ignored |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo build --release --locked` | Passed: standalone executable and shared library |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 4 publication regressions passed |
| `python3 scripts/check-terminal.py target/release/shr-pa` | Passed |
| `python3 scripts/check-live-controls.py target/release/shr-pa` | Passed, software-null PCM only |
| `cc -std=c11 -Wall -Wextra -Werror -fsyntax-only -x c src/shr_pa.h` | Passed |
| Release library loaded using Python ctypes | All four symbols, native precision, channel identity and latched fault silence passed |
| `python3 scripts/check-docs.py` | Passed |
| Complete-index publication guard and staged diff review | Passed |

The six ABI regressions cover constructor bounds (including accepted endpoints),
stereo identity, a sample below f32 resolution, the startup ramp, inactive output
silence, linked limiter gain, non-finite/huge input faults, silence of earlier
samples in the failed block, persistence/recreation, pointer/length rejection,
and f32/f64 sample agreement across every existing layout with EQ and delays.
The seventh new regression records zero allocations/deallocations for ABI
processing. Existing DSP/transition/protection tests continue to pass.

Release shared-library SHA-256 at validation:
`0c69949d99d209f4b23c0256a1b80f8b3b9d169d4d09d91ce9f064ea5bfd75a4`.
The normal target directory grew from 774 MiB to about 1.1 GiB; about 38 GiB
remained free. Current required executables and normal build output were retained.
No private media or temporary evidence files were added to source Git.

## Limits and skipped classes

Actual USB playback/capture, network timing and integrated recording belong to
GigPies' reserved host experiments. This change opens no physical device and
makes no acoustic or physical-latency claim. The sample limiter is not calibrated
amplifier/speaker protection. Host buffer latency and any FX delay remain separate
from the fixed preset's zero algorithmic delay.

Hardware soaks, unrelated historical renderers, long workload benchmarks and
exhaustive research were intentionally skipped. No historical one-time test
needed reclassification. Current complex-response regressions continue to
protect active production behavior. Their commands remain in
[validation](../VALIDATION.md). No remote CI was requested or run.
