# Optional live input EQ extension — task0018 candidate

The independent [EQ v1 header](../src/shr_pa_eq_v1.h) adds seven optional symbols.
Hosts must resolve the complete set and validate its 32-byte capabilities and
64-byte status layout. Missing, partial or incompatible extensions leave the
original v1 and configurable v2 loading and muted configuration workflow usable.
The original headers, graph schema and admission interpretations are unchanged.
This candidate requires independent coordinator acceptance before consumer freeze.

## Narrow ownership

`prepare` is exclusive controller work. Its strict bounded JSON has `version:1`
and exactly two distinct `inputs`. Each entry has only `input_index`, `eq_enabled`,
`eq` (eight original EqBand objects), `geq_enabled`, and `geq_db` (31 gains).
Duplicate/unknown keys, nonfinite values, invalid indices/ranges and unstable banks
are refused. PEQ uses the original RBJ bell/shelf S semantics; GEQ uses Q=4.318.
GEQ centres above 0.45×sample rate are identity, retaining their stored settings.
Gain, dynamics, delay, route, crossover, speaker EQ, protection and mutes cannot
appear in an EQ patch. There is no graph patch hidden in the extension.

A token pins the unique monotonic owner lifetime identity, graph generation,
independent EQ generation, epoch and exact prospective source frame. Both owner
instance and generations are checked at apply. Identity counters never wrap.
A replaced or recreated owner cannot accept an old token even at the same numeric
generations and frame. A finite completion span is reserved before accepting a
non-identical patch. Tokens and the library stay alive until explicit consumption
or controller destruction. Callers must provide valid aligned disjoint allocations;
span checks do not establish backing allocation validity or permit concurrent access.

The optional extension reserves 80 extra per-frame work units (two extra 39-section
banks and blend work) within the existing finite budget. It does not lower legacy
graph admission. A legacy graph that leaves insufficient extra capacity still loads;
its live EQ status reports ineligible. These are finite work bounds, not measured
hardware deadlines or physical channel qualification.

## DSP and retirement

Apply installs both committed settings together without allocation/destruction.
For each changed selected input, old filter histories continue; the fresh target
bank starts with zero histories. Only the EQ outputs blend, after existing input
gain and before the single continuing compressor and input delay. One clock advances
once per source frame outside the input loop. At 48 kHz its 240 frames have weights
0/239 through 239/239: an exact old start and exact target endpoint. At other rates
the duration is ceil(rate/200). No coefficients interpolate, no state is transplanted,
and the graph is never crossfaded. Unchanged selected inputs preserve their banks
and histories; unselected inputs and all downstream state remain in place.

Fully identical settings acknowledge successfully and increment the independent
EQ generation, without processing a fade or resetting histories. They still occupy
the reserved retirement slot until controller retirement. This makes request/ack
semantics consistent with an accepted transaction while preserving sample identity.

Endpoint bank swaps retain old banks in the same reserved token. `retire` transfers
finished or faulted ownership to an initially NULL caller slot; `destroy` is offRT.
Occupied transition/retirement refuses retargeting. No chain or queue is built.
Full graph replacement under quiescence transfers all active/retired extension
resources into the retired whole graph, which the host destroys offRT. Separately
prepared tokens remain caller-owned and become stale on the new owner instance.
Already-muted graphs can therefore still use the original full replacement path.

Persistent mute and numerical/timeline faults retain precedence. EQ never rearms
or unmutes. Faulted unfinished transitions retain honest prior-current settings
through retirement; committed target remains recovery intent. Readback reports
fault and `settled:false`, even if no transition storage remains. Fresh recovery
prepares target settings with zero histories, muted/disarmed, without replay.

## Owner display evidence

Controller-only `readback` returns bounded JSON with actual current and committed
target settings, sample rate, both generations, fingerprints and normalized banks.
Each of the 39 coefficient rows is `[b0,b1,b2,a1,a2]` with denominator
`1+a1*z^-1+a2*z^-2`. FNV-1a64 fingerprints cover compact serde JSON of the two
settings objects in requested order; artifact/fixture provenance separately uses
SHA-256. This is a static EQ response description, not a measured compressor or
mid-transition system transfer function. Consumers label current/target separately.

`owner-allocation-guard` builds a separately instrumented owner library. Its two
exported controller calls bracket same-thread apply/render/status/retire tests and
count allocations, reallocations and deallocations inside the loaded library.
The guard is disabled outside the bracket and is an opt-in evidence build; normal
integration allocation tests retain their existing allocator. Do not combine the
feature with integration-test crates that declare their own global allocator.

Focused checks: `cargo test --locked --test live_eq --test ffi_eq --test render_allocation`.
Normal checks and device-free controls remain in [VALIDATION](VALIDATION.md).
Physical audio, listening, acoustic acceptance and measured deadlines remain separate.
