# SHR PA: plan and implementation

SHR PA owns speaker processing/protection, measurement, alignment and prepared
embedding. GigPies owns program mixing, physical audio devices and final integration.

## Delivered contract and evidence

PA-01 fixed-v1 descriptor/status and PA-02 configurable C-PA v2 have software
acceptance. V2 supports dynamic program inputs, weighted DAG routing and independent
speaker outputs, EQ/dynamics/delay/polarity and sample protection; v1/standalone
compatibility remains. [Embedding v2](EMBEDDING_V2.md),
[owner verification](verification/0017-configurable-embedding.md) and the GigPies
modular acceptance record own exact evidence. Acoustics and physical protection
remain unqualified; sample limiting is not true-peak or speaker calibration proof.

## Next work

Measurement and delay/polarity alignment have one owning task plan:
[PHASE_ALIGNMENT](PHASE_ALIGNMENT.md). Update its plan/checklist/evidence in place;
do not copy its P4/P5 tasks here or into GigPies. Its later host adapter is a distinct
integration task only after the measurement contract exists. Physical graph
qualification contributes to shared GP-H1. Other standalone PA features stay in
[the standalone roadmap](ROADMAP.md), outside GigPies's integration queue.

## Tracking and verification

This is the owning plan for module-only GigPies work. Keep each new task's plan,
implementation state, checklist, evidence and next action together here. Shared
integration tasks live only in the [GigPies integration plan](https://github.com/PaolaShultz/gigpies/blob/main/docs/MODULE_IMPLEMENTATION_PLAN.md);
link to their cards instead of copying status. Follow its task lifecycle and this
repository's AGENTS.md. STATUS/acceptance files hold dated evidence, not another queue.
Preserve closed milestone details in the linked archive; do not reopen old launch cards.
A documentation reconciliation does not rerun tests or qualify hardware.

[Historical plan and milestone evidence](archive/tracking-before-2026-10-09/GIGPIES_IMPLEMENTATION.md). Its launch instructions are retired.
