# SHR PA working agreements

## Scope

Highly experimental Rust PA management project for Raspberry Pi 5 and Linux Lite.
UMC1820 is a future target, not a prerequisite for DSP development.
Keep logical 2×6 processing, available physical channels and possible measurements distinct. Preserve implemented, planned and
hardware-verified status. Current scope is the complete DriveRack PA2 function
plan in fixed 2-input/6-output configurations, with one separate setup mic.
LR24 is first/default. General matrices, advanced routing, eight-point positional
RTA and the later nine-channel arrangement are future work; do not make them
prerequisites or infer their roles. See docs/DRIVERACK_MAP.md and docs/FUTURE.md.
Do not add live audio or change host audio settings as planning/scaffold work.

## Modular development and next measurement work

SHR PA owns the PA module intended for later integration into GigPies. Develop and
validate PA processing and measurement here while retaining standalone operation.
Read docs/PHASE_ALIGNMENT.md when continuing measurement, P4/P5 setup or alignment
work: it records the requested reference/mic phase measurement and delay/polarity
alignment tasks, acceptance criteria and future GigPies boundary. Integration is
planned; do not duplicate this implementation in GigPies or assume it already exists.

## Validation

The agent owns test classification and selection. Keep fast production unit,
contract, schema, safety, recovery and regression tests in the default suite.
Start with focused checks during implementation. Run the complete normal suite
for engine, render, shared model/schema, routing/persistence, concurrency, safety
or broadly reused component changes and before publication.

Historical research, auditions, exhaustive matrices, long benchmarks and
one-time evidence renderers are opt-in once their evidence is recorded, unless
they protect current production behavior. Run them when their assumptions or
protected behavior changes, or when explicitly requested. Document on-demand
commands. Reclassify slow one-time tests encountered in scoped work instead of
repeatedly imposing them. Report test classes run and intentionally skipped.

Normal commands are in docs/VALIDATION.md. No tests may open hardware implicitly.
Use the pinned toolchain. Keep changes standalone and avoid sibling dependencies.

## Publication

Follow docs/PUBLICATION.md before committing or pushing. Keep user state, private
media and generated artifacts untracked. Enable the versioned hooks when absent;
review new scripts in scripts/publication-policy.json and run the complete-index
publication guard. One-off bench runners belong in ignored artifacts/.
