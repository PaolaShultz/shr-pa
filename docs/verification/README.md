# Verification records

Records preserve what was measured at the stated date and workload. Their test
counts and statements such as “not pushed” or “pending” are historical, not the
current release state. [STATUS](../STATUS.md) summarizes today's implementation.

| Record | Scope | Subsequent history |
| --- | --- | --- |
| [0001 — scaffold](0001-scaffold.md), 2026-09-29 | Offline terminal and lifecycle; no DSP/audio | Foundation and resize fixes precede `488c682` |
| [0002 — PA2 plan](0002-driverack-plan.md), 2026-09-29 | 42-ID source audit and documentation | Recorded in `488c682`; implementation followed |
| [0003 — engine](0003-engine.md), 2026-09-29 | Offline DSP and short physical AudioBox stereo trials with 64 PEQs | Engine and later controls committed together in `b955027` |
| [0004 — live controls](0004-live-controls.md), 2026-09-29 | 126-EQ/compressor/transition software tests and offline workload; hardware open failed busy | Committed in `b955027`; library/crossover work followed |
| [0005 — library](0005-preset-library.md), 2026-09-30 | Slots, recovery and EQ histories; software-null only | Committed with D08 in `142db7f`; its v2 processing/v1 envelopes were upgraded |
| [0006 — crossover](0006-crossover.md), 2026-09-30 | Independent BW/LR edges, v3 processing/v2 envelopes; software-null only | Committed in `142db7f` |
| [0007 — 0.2 alpha](0007-0.2-alpha.md), 2026-09-30 | Documentation/release audit and publication validation | Application `0.2.0-alpha.1` |
| [0008 — pink generator](0008-pink-noise.md), 2026-09-30 | M03 deterministic pink source, spectrum, allocations and software-null integration | Application/schema versions unchanged |
| [0009 — generator level](0009-generator-level.md), 2026-09-30 | M03 session-start source peak level, rejection, six-output scaling/limiting and software-null checks | Application/schema versions unchanged |
| [0010 — runtime generator level](0010-runtime-generator-level.md), 2026-09-30 | M03 live level controls, bounded ramps and software-null integration | Application/schema versions unchanged |
| [0011 — generator off/on](0011-generator-capture-restore.md), 2026-09-30 | M03 bounded capture restoration, continuing source state and software-null controls | Application/schema versions unchanged |
| [0012 — phase plan and publication](0012-phase-plan-publication.md), 2026-10-03 | Pending alignment task, modular GigPies intent, publication protection and normal checks | Application/DSP unchanged |
| [0013 — native f64 embedding](0013-embedding.md), 2026-10-03 | Shared DSP f64 boundary, versioned C ABI, faults, precision and normal checks | Fixed full-range embedding; physical integration owned by GigPies |
| [0014 — USB integration checkpoint](0014-integration-checkpoint.md), 2026-10-03 | Coordinator-reported AudioBox USB integration, exact dry/recording checks and faults | 8 ms target failed; 16 ms with larger buffers passed 600 s; bounded left electrical return verified, right unresolved; GigPies owns acceptance |
| [0015 — standalone transfer pacing](0015-transfer-pacing.md), 2026-10-03 | Full-block availability before transfer, partial/retry/stop regressions and normal checks | DSP/ABI unchanged; physical low-latency acceptance remains separate |
| [0016 — low-latency integration diagnosis](0016-low-latency-integration.md), 2026-10-03 | Smaller-buffer GigPies trials and traced locked-page migration during PA processing | H8 passed digital checks for 600 s at 6 ms wet admission; physical qualification remains open with two weak windows at offset changes |

0003 contains standalone physical audio trials; 0014 and 0016 record later GigPies
integration checkpoints including bounded left electrical returns. They do not
qualify the complete current workload or independent crossover controls. None
of these records isolates converter latency or establishes acoustic response, speaker
protection, power-cut durability or general live reliability. UMC1820 remains untested.
