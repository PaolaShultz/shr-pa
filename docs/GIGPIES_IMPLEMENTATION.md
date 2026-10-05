# SHR PA: GigPies embedding implementation

## PA-02 successor implementation — 2026-10-05

[C-PA v2](EMBEDDING_V2.md) now implements dynamic program inputs, explicit
weighted mono routing and independent protected outputs, including compensated
LR24 stereo three/four-way and 4×8 references. Existing standalone fixed presets
and v1 bytes/semantics remain supported. This supersedes earlier fixed-2×6-only
ordering and deferral of all matrix/configurable embedding below. Scope here is
software implementation; physical I/O, acoustic/true-peak protection and
measurement remain unqualified or unavailable as documented.


Planning baseline **2026-10-04 / GP-2026-10-04.1**. PA-01 is now implemented; later tasks remain
**planned**. Current software validation is recorded in Progress below. [Central inventory](https://github.com/PaolaShultz/gigpies/blob/main/docs/MODULE_IMPLEMENTATION_MAP.md) ·
[Agreed contracts](https://github.com/PaolaShultz/gigpies/blob/main/docs/MODULE_CONTRACTS.md). Existing product roadmaps remain authoritative for
unrelated work; this plan owns only the GigPies integration increments below.

## Objective and boundary

Own fixed 2×6 speaker processing/protection, measurement and alignment. Stagebox device and whole-band mixer are GigPies responsibilities. Preserve standalone ALSA operation, v3 presets/v2 envelopes and exact fixed-v1 embedding; no matrix/monitor engine expansion by name.

## Source and evidence reviewed

Repository: `/home/shome/p/shr-pa`. Inspected HEAD: `7683f9b38e2452e904468be3b90306ea450618ee`.
Clean at inspection; recheck before editing. This dated observation is not a future ownership claim.

Owning documents: README.md; docs/STATUS.md, ROADMAP.md, DSP.md, EMBEDDING.md, PHASE_ALIGNMENT.md, DRIVERACK_MAP.md, VALIDATION.md, PUBLICATION.md.

Source inspected: `src/ffi.rs::shr_pa_v1_*`, `src/dsp.rs::Engine`, `src/control.rs::Handoff`, `src/config.rs`, `src/shr_pa.h`.

`Engine::render_f64`, `Handoff` and Prepared changes implement six logical
outputs, pair controls, protection and bounded transitions. Fixed ABI v1 only
exposes full-range L/R, silent 2..5, 5 ms startup ramp and sample limiter. Phase
measurement/RTA/automatic alignment and configurable embedding remain planned.
Status/verification 0013..0016 records software and qualified stereo hardware;
no renewed acoustic, UMC1820 or complete six-output physical acceptance here.

These are source inspection and previously recorded results, not fresh builds or
physical acceptance. The planning session runs documentation checks only.

## Milestones and tasks

First: truthful fixed-v1 descriptor and read-only health seam. Next: reviewed prepared configurable ABI and offline host acceptance. Phase-analysis tasks remain in their existing PHASE_ALIGNMENT.md owner, independent of UI/device availability.

Task states are execution dependencies: READY has no missing software provider;
WAITING names its precise prerequisite; DEFERRED has an activation condition.
Source delivery and build reservation are additional launch prerequisites on a
peer. Every row has one owner, the repository named in its Owner column. A later
task starts only after the previous artifact is reviewed, never merely delivered.

| Task / priority / state | Owner | Work area, inputs and required artifact | Output and measurable acceptance |
|---|---|---|---|
| PA-01 / P1 / IMPLEMENTED | SHR PA | `src/ffi.rs`, src/shr_pa.h, ABI tests, EMBEDDING.md; C-PA:1/E08. | Add read-only versioned capability/status seam without changing v1 behavior. Describe logical vs physical outputs, fixed preset, sample-limiter scope, fault and unavailable measurement/control. E08 fixture/header/library artifact for GP-05. |
| PA-02 / P1 / READY | SHR PA | Existing Prepared/Handoff and schema v3; C-PA B-PA. Initial work is a design increment in EMBEDDING.md. | Review exact prepare/apply/readback/retire ABI, single pending slot, linked pair identity, reject invalid state before publish, muted structural change and meter snapshot ownership. After reviewed header/version, implement one prepared gain/mute path first, then remaining declared controls; each independently tested. GigPies writable consumer waits this accepted provider artifact. |
| PA-03 / P2 / READY | SHR PA | PHASE_ALIGNMENT.md requested offline phase task, production DSP references; no GP task required. New bounded measurement model/tests under src; owner chooses module name when implemented. | First small slice: synchronized two-channel reference/mic data descriptor and known-delay/polarity test vectors; reject dropped/clipped/ambiguous/clock-invalid input. Then estimator/confidence and unchanged-or-bounded pair proposal, separate review per slice. No automatic apply, new capture host or mic-to-program route. |
| PA-H1 / P2 / DEFERRED | SHR PA | PA-03 confidence/proposal evidence plus fresh physical/setup-mic reservation. | Loopback then isolated speaker/overlap/multiple-position acoustic validation, preserve protection, unsupported independent L/R or >10 ms requests refused. Never infer room alignment from offline tests. |

## Validation and failure behavior

Focused `CARGO_INCREMENTAL=0 cargo +1.97.1 test --locked ffi -j 1` for PA-01; full normal `cargo +1.97.1 test --locked --all-targets -j 1` under same environment for ABI/engine/schema changes. Run fmt, warning-denied Clippy, `python3 scripts/check-docs.py`, publication guard and normal script unittest suite per VALIDATION.md. Prepared live changes also need release build and software-null `check-terminal.py`/`check-live-controls.py` there. Those do not authorize physical PCM.

Protect whole-block/latching numerical silence, disjoint bounds, startup and mute ramps, linked limiting and zero render allocations. Prepared retirement never frees on callback. Measurement cancellation/uncertainty preserves old configuration and mic isolation. Strong design review for ABI lifetime and phase confidence; keep recorded physical failures.

Historical research, auditions, exhaustive matrices, long soaks, full-show renders
and physical/combined-load checks are intentionally outside the normal software
milestones unless their protected behavior changes. Retain their owning documented
on-demand commands; no private media download or test hardware side effect.
Independent builds retain lockfiles and existing repository editions; this plan
does not upgrade dependencies/editions or replace existing intra-repository workspace
paths. The ban is on new sibling-repository path dependencies.

## Resources, review and recovery of work

Wave 2/3 on local NVMe lane after GP/Lux first reviews; <1 GiB compiler RSS and ≤768 MiB target growth provisional for descriptors. PA-03 synthetic data cap ≤8 seconds/fixture and ≤32 MiB aggregate temporary output; no exhaustive room simulation. One jobs=1 slot. Physical gates separate.

Independent fallback: PA-02 bounded ABI design or PA-03 reference/mic descriptor design before compiling; do not implement later slices without review. No unbounded render or research assignment.
Before builds check free space and target size; below 20 GiB free or above 5 GiB
output is a review, not permission to delete another task's cache. No reduced
coverage/debug information to make a budget appear to pass.

Handoff: exact changed files, commit plus patch hashes or bounded source manifest
if uncommitted, contract IDs/versions and provider-fixture hashes, commands/results,
intentional skipped classes, remaining limits and next task/owner. Stage only named
owned changes if a later implementation session commits; no public push is implied.
Receiving owner reviews independently and writes an immutable private-ledger
acknowledgement. Interrupted work stays visible with last completed acceptance
criterion; never reset/stash/clean another session or replay an uncertain mutation.


## Implementation launch prompt

Host/cwd assignments and source preparation are in GigPies PARALLEL_WORK_PLAN.md.
This is a prompt for a later user-started session; no implementation worker has
been started by the planning pass.

```text
Work only in the current shr-pa checkout. Read AGENTS.md (if present), the
owning docs and docs/GIGPIES_IMPLEMENTATION.md, then the referenced GP-2026-10-04.1 contracts.
Implement only PA-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback PA-02 bounded ABI design or PA-03 reference/mic descriptor design before compiling; do not implement later slices without review within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

## Progress

- 2026-10-04: source and owner documents inspected; plan written. Implementation
  tasks remain in the states above. Physical evidence retains its original limits.

- 2026-10-04 task0009 PA-01: additive fixed-width `shr_pa_v1_descriptor` and
  single-owner/quiesced `shr_pa_v1_status` implemented. Exact version/size and
  pointer-shape checks refuse without writing; status rejects overlap with inline
  handle and all eight heap delay allocations. Original process arithmetic and
  v1 signatures remain unchanged. [Embedding contract](EMBEDDING.md) and
  [E08 corpus](../tests/fixtures/cpa/v1/README.md) own exact layouts/commands.
  PA-02/PA-03, configurable controls, measurement and all physical gates remain
  separate. Task0009 supersedes the historical launch prompt's publication stop;
  the coordinator alone owns source commits/pushes after final review.

  Software validation: 70 normal Rust tests, five script tests, fmt check,
  warning-denied all-target Clippy, release build, actual release-linked C caller,
  docs checker and complete-index publication guard passed. Offline terminal
  restoration/recovery and explicit software-null live controls passed. Fresh
  version, six-channel float WAV, 40×13 snapshot and source-preserving v2 migration
  checks passed. Commands are in [VALIDATION.md](VALIDATION.md),
  [RUNNING.md](RUNNING.md) and the E08 corpus. Historical media, exhaustive/long
  research and hardware tests were intentionally skipped. Final receiving review
  and publication belong to the coordinator; no source commit/push by the worker.
