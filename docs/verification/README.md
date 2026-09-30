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

Only 0003 contains successful physical audio trials. They do not qualify the
later full workload, independent crossover controls or physical response. None
of these records establishes analog loopback latency, acoustic response, speaker
protection, power-cut durability or long-soak acceptance. UMC1820 remains untested.
