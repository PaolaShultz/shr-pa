# 0002 — complete DriveRack plan in a fixed 2×6 system

Status: accepted scope, 2026-09-29. Supersedes decision 0001's open starting topology.

The user specified that the current plan should cover all DriveRack functions in
simple two-input/six-output configurations. Matrix mixing, advanced routing and
eight-point positional RTA belong to future work. The previously mentioned
nine-channel arrangement also remains future work until its roles are supplied.

Use the existing PA2 reference for the function audit. Include all its documented
processing, setup, measurement, operating and maintenance functions; do not limit
the plan to EQ/crossover/delay/limiter. Record hardware substitutions and proprietary
algorithm/data boundaries explicitly. This is not a claim to cover the entire
DriveRack product family.

Implement fixed full-range, two-way and three-way configurations before any graph
compiler. LR24 remains first/default; additional reference filter options stay
mapped. One setup mic and the ordinary PA2 measurement sequence are part of the
baseline. Eight-point positional/multi-mic measurement is deferred.

The function inventory is docs/DRIVERACK_MAP.md; implementation stages are in
ROADMAP.md and future scope in FUTURE.md. Only scaffold labels/documentation change
in this update. DSP and audio devices remain unopened.
