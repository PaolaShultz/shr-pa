# D08 independent crossover controls — 2026-09-30

> Historical evidence for the dated slice below. Test counts, pending features and
> commit/push statements describe that moment. See the [evidence index](README.md)
> for subsequent commits and the [0.2 alpha record](0007-0.2-alpha.md) for release checks.

Implemented on the existing uncommitted preset-library/EQ-history checkout.
Those features and their files are retained. Nothing was committed or pushed;
no new remote CI result is claimed.

## Working offline

- Per-pair independent HP/LP bypass and cutoff, BW6/12/18/24/30/36/42/48 and
  LR12/24/36/48. Cutoffs validate separately, including stored bypassed edges;
  intentional overlaps, gaps and full-range pairs are allowed.
- Fixed layouts and logical roles remain. Inactive outputs are silent, all six
  outputs process independently of physical mapping, and mono bass retains its
  explicit averaging and layout scope. Pair gain/polarity remain user-owned.
- Default `layout_lr24` retains the original tree and three-way phase compensation.
  Explicit independent mode uses direct per-pair HP→LP without automatic phase
  correction. Mode, both edge settings and inactive-pair status are displayed.
  Split shortcuts reject in independent mode. Mode resets and layout behavior
  are documented in [RUNNING](../RUNNING.md#crossover-controls).
- Matched LR12/LR36 require one branch inverted; LR24/LR48 use equal polarity.
  Independent three-way and arbitrary edge/polarity combinations have no flat-sum
  guarantee. See the complete [phase contract](../DSP.md#independent-edges-d08).
- Strict processing v3 and library/working v2 envelopes. Explicit `migrate OLD NEW`
  accepts standalone v1/v2 and old v1 library/working envelopes. It preserves source
  files, EQ histories, both saved/working baselines, names and selection. Validation
  precedes writing; atomic publication refuses existing destinations and symlinks.
  Old recovery blocks writes until migrated or explicitly archived by the user.

## Working during live streaming

Coefficients and stability checks run on the controller. Independent edges use
four fixed sections per edge/channel. All changes use the existing bounded
prepared handoff and 5 ms mute/reconfigure/20 ms settle/resume path. No filter
coefficient design, allocation/deallocation, locks or filesystem work enters
render. Latest desired edits survive handoff backpressure. Runtime mutes and
faults retain ownership and precedence. Mode changes reset crossover state only;
ordinary edge edits retain unchanged filters and all unrelated DSP history.

The 40×13 software-null PTY check exercises both edge bypasses, frequencies,
families and slopes, enabled BW48 HP/BW42 LP on the high pair, LR48 mid LP and
LR12 low LP, intentional HP/LP gaps, rapid edits, rejected split shortcuts,
save/import, recall, pending/busy completion, ongoing blocks, meters and mutes.
It also retains the library/EQ commands, incompatible-rate rejection and terminal
cleanup checks. A fresh explicit null session without a generator recovers the
saved independent crossover muted, even with `--unmute --ui` requested.

Software null proves control/transport integration. It does not establish USB
scheduling, physical response or audible transition quality.

## Normal validation

Native aarch64, pinned Rust `1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked
python3 scripts/check-terminal.py target/release/shr-pa
python3 scripts/check-live-controls.py target/release/shr-pa
```

All passed: **45 production Rust tests**, no ignored tests. Focused checks ran
before the full normal suite. Final display-only and test-strengthening edits
received focused rechecks, strict Clippy and final terminal/null checks.

New and retained evidence:

- Independent analog pole-product references, with bilinear frequency warping,
  compare complex impulse DFTs for every family/order and both HP/LP directions
  at 44.1/48/96 kHz. They do not reuse production coefficient or Q design.
  Tests check cutoff magnitudes, phase, warped stopband slopes and stereo pairing.
- All orders prepare at both cutoff extremes for 8/44.1/48/96/192 kHz. BW48/LR48
  render complex cutoff references at 16 Hz and the upper valid frequency at
  8/192 kHz. Bypass is exact identity; invalid/nonfinite cutoffs/slopes reject.
- Matched LR sums with explicit polarity, odd-order BW sums, even-order BW
  cancellation/boost, deliberate gaps/overlaps, all fixed layout roles and bass
  averaging. Existing compensated LR24 two-way/three-way complex regressions pass.
- Crossovers preserve untouched pair samples after mute/reconfigure, including
  compressor/limiter envelopes, EQ and delay histories. Bounded backpressure,
  final transaction acceptance, retained mute targets and fault rejection pass.
- Migration covers each layout, source/destination protection, unknown/incomplete
  fields, invalid histories, incompatible versions, working/saved baselines,
  restored EQ, slots and blocked old recovery. Existing v1 conversion remains.
- Allocation/deallocation counting includes maximum-order independent HP/LP,
  bypass/order changes, return to layout mode, full EQ/dynamics/delay edits,
  recall and faults: **zero render allocations and deallocations**.
- Terminal lifecycle/navigation/resize/recovery checks pass. New controls and
  summaries fit 40×13; pending/modified/recovered status and cleanup are retained.

These are bounded production regressions. Hardware soaks, exhaustive frequency/
parameter sweeps, workload benchmarks and disposable evidence renderers remain
opt-in and were intentionally skipped. No historical tests required reclassification.
On-demand commands remain in [VALIDATION](../VALIDATION.md).

## Hardware actually exercised

**None in this slice.** Only explicitly selected ALSA software-null PCM ran.
No card numbers, device assumptions, host audio settings or JACK state changed.
Development defaults remain 48 kHz / 128-frame periods / 512-frame buffers.
Earlier AudioBox trials remain separate historical evidence. UMC1820 remains a
future target; no analog loopback is assumed.

## Remaining implementation and measurement

D08's requested controls are implemented. Automatic phase compensation for
arbitrary independent crossovers is not provided; the explicit layout LR24 mode
retains the supported compensated three-way behavior. No proprietary response
or flat arbitrary sum is claimed.

Physical crossover response, switching transients, analog/acoustic sums, maximum
USB render timing for the expanded workload, long soaks, power-cut durability and
UMC1820 acceptance remain unmeasured. There is no new physical performance claim.
Limiter extensions, RTA/AutoEQ/setup mic, general routing and the other pending
functions in the [map](../DRIVERACK_MAP.md) were not added to this slice.
