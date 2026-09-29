# SHR PA working agreements

## Scope

Highly experimental Rust PA management project for Raspberry Pi 5 and Linux Lite.
UMC1820 is the intended interface. Preserve the distinction between implemented,
planned and hardware-verified behavior. Do not infer channel assignments: up to
eight measurement microphones are a possible venue-setup use; later operation
uses nine channels whose roles remain unspecified. 4x8 is an example, not a fixed
architecture. Do not add live audio or change host audio settings as scaffold work.

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
