# PA measurement and alignment

**Pending SHR PA development task, intended for later GigPies integration.**
Recorded on 2026-10-02 from the user's proposal to use a measurement mic and pink
noise to identify phase problems and correct arrival-time differences with small
delays. This document adds no implementation or hardware acceptance claim.

Develop and validate this capability here in SHR PA. The user's system is modular:
GigPies is intended to integrate the finished PA module. Keep the PA DSP, measurement
and alignment rules owned here rather than creating a second implementation in
GigPies. SHR PA must remain independently usable. Fixed full-range processing
now has an [embedding interface](EMBEDDING.md); the measurement integration
interface and packaging will be defined when that capability is built.

## Pending work

- [ ] P4: add isolated setup-mic capture and a synchronized excitation reference,
  with timing, clipping and dropped-data reporting.
- [ ] P4: implement transfer-function magnitude/phase, arrival estimates and
  confidence/coherence analysis outside the render callback. RTA alone is insufficient.
- [ ] P5: build a reviewable delay/polarity alignment stage using the existing
  pair controls, including an unchanged result and clear unsupported cases.
- [ ] P5: verify combined response across relevant frequencies and several sequential
  microphone positions; preserve settings on cancellation or inconclusive evidence.
- [ ] Complete the production-DSP regressions and separately authorized physical
  validation listed below. Record actual supported layouts, latency and limits.
- [ ] Once the module is ready, define and test the GigPies integration contract
  without duplicating PA algorithms or starting a competing audio-device owner.

These extend P4/P5 and manual commissioning; they do not require general matrix
routing or the deferred eight-microphone system. This is our requested extension,
not a claim that the PA2 reference implements automatic phase alignment.

## Existing SHR PA implementation

The 2026-10-02 inspection covered `AGENTS.md`, `README.md`, `docs/STATUS.md`,
`docs/ARCHITECTURE.md`, `docs/DSP.md`, `docs/DRIVERACK_MAP.md`, generator evidence,
and relevant source/tests in this repository. The inspected version reports
0.2.0-alpha.1, processing schema v3. This dated record must be rechecked in future work.

| Capability | Inspected behavior and owner |
|---|---|
| Test source | Seeded pink/white noise, sine, impulse and sweep; pink generation is in `src/offline.rs`. Explicit level and live on/off controls have software evidence. |
| Processing layout | Two program inputs and six logical outputs, with fixed layouts. Physical channel availability is separate. |
| Alignment delay | 0–10 ms per stereo band pair. `src/config.rs` owns `Band.delay_ms`; `src/dsp.rs` rounds it to the nearest sample and uses separate channel histories. |
| Delay resolution | One sample, about 20.8 microseconds at 48 kHz. The terminal edits pair delay in 0.1 ms steps. No fractional-sample delay is implemented. |
| Polarity | `Band.inverted` controls the pair's sign. Settings are linked across L/R; these are not six independently adjustable delay/polarity controls. |
| Input delay | 0–100 ms for the complete program before crossover. It cannot replace relative output alignment. |
| Crossover phase | Default LR24 three-way layout has designed phase compensation. Independent crossover edges have explicit phase/polarity behavior; this is not measured acoustic calibration. |
| Transitions | Delay changes crossfade existing taps over 20 ms. Polarity/gain edits also transition; arbitrary edits are not guaranteed inaudible. |
| Missing measurement | Separate setup-mic capture, RTA and automatic setup remain pending. No transfer-function/coherence analyzer or automatic acoustic alignment selector was found. |

`tests/engine.rs`, `tests/live_processing.rs`, `tests/crossover.rs` and
`tests/generator.rs` cover processing behavior. They were inspected, not rerun for
this documentation task. Existing transport trials do not establish acoustic
alignment: the recorded physical trials had no analog loopback and an unpowered amp.
Analog latency, physical response and acoustic protection remain unmeasured.

## Measurement needed

A basic RTA displays frequency levels. Pink noise and that display alone do not
identify phase or propagation time. Compare a synchronized copy of the actual
excitation signal with the microphone recording to estimate a complex transfer
function, impulse response and measurement coherence. This is a dual-channel
measurement: one reference plus one mic, not a requirement for two microphones.
See [Rational Acoustics' measurement explanation](https://support.rationalacoustics.com/support/solutions/articles/150000190431-measurement-101-types-of-measurement).

The reference could be a correctly timed internal signal tap or an electrical
loopback. Its insertion point, output/input clock relationship, converter/transport
latency and timing offsets must be established. An internal generator does not by
itself provide a validated reference capture. Keep the analyzer's delay compensation
separate from the delay sent to the PA processor. Preserve a common time origin
across measurements; independently zeroing each arrival would discard the relative
timing we need.

The measurement mic must stay outside the program/monitor mix. Capture analysis
through bounded taps and perform FFT/correlation work outside the audio callback.
Missing samples, clock drift, clipping or insufficient usable signal invalidate
affected measurements. Coherence supports confidence; a high value alone does not
prove that reflections are absent or a correction is useful.

## What a delay can address

Small relative delays can improve summation between mains/subs or mains/fills when
arrival-time differences explain the interference. Measure the individual paths and
their sum, especially across the crossover overlap. Loudspeaker setup guidance
already treats measured delay and polarity as alignment controls; see
[Meyer Sound's ULTRA-X20 instructions](https://docs.meyersound.com/products/en/operating-instructions---ultra-x20.html).

- Delay changes phase by `-360 × frequency_hz × delay_seconds` degrees. One
  millisecond means 36 degrees at 100 Hz and 180 degrees at 500 Hz. It cannot
  independently choose a correction at each frequency.
- A polarity inversion changes sign across frequencies; it is not equivalent to a
  fixed delay. Check polarity separately and retain the crossover's intended behavior.
- Different driver/crossover phase shapes may need additional filter design. Do not
  force a delay-only solution or silently alter crossover/protection settings.
- Reflections and cancellations vary across the audience. A single position cannot
  establish a room-wide fix. Delaying an entire speaker also delays its reflections.
- A causal delay can hold back an earlier arrival, not advance a later one. Report
  added latency and the selected acoustic reference. Far fills may exceed the current
  10 ms output range; linked L/R settings may prevent a required independent change.

Fractional-sample delay, independent output settings, longer delay ranges and
additional phase filters are possible later extensions. Measure a concrete need
before changing those contracts. Existing designed all-pass compensation is not a
general automatic room-phase correction engine.

## Proposed setup sequence

1. Freeze the existing PA configuration and identify actual speaker/output mappings,
   crossover regions, measurement positions and the reference signal path.
2. Measure relevant speaker paths separately using the same reference, mic position
   and timing basis. Record magnitude, phase, arrival estimates and data confidence.
3. Propose bounded pair delay/polarity changes only where evidence supports them.
   Compare phase over the useful overlap, rather than matching one impulse peak or
   one frequency. Retain the unchanged configuration as an eligible outcome.
4. Predict summation, then verify the actual combined response. Check additional
   representative positions before accepting a correction; report conflicting results.
5. Save a reviewable proposal with before/after settings, confidence, scope and
   latency consequences. Apply through validated processing transactions with explicit
   operator control. A failed or cancelled measurement preserves the last valid setup.

PA calibration and artistic instrument tone maps have different purposes and state.
Do not change instrument faders, source timing or channel makeup to compensate for
speaker alignment. Keep limiter/protection settings intact. If later EQ/crossover
changes affect phase, recheck the alignment rather than treating it as permanent.

## Modular integration with GigPies

GigPies' intended Brain can coordinate setup and review while its Stagebox owns
the live audio path. SHR PA owns the PA engine and this measurement/alignment
capability. The finished module should expose validated configuration, measurement
results, proposals and bounded application of settings through a documented contract.
Preserve the standalone SHR PA host and controls. Do not make GigPies a prerequisite
for developing or testing PA behavior.

The [version 1 embedding interface](EMBEDDING.md), implemented after this plan,
now exposes fixed full-range PA processing. Measurement/configuration integration
still needs its own contract; version 1 does not implement those capabilities.
Keep schemas, latency, protection, cancellation, state ownership and fault behavior
explicit. Select one audio-device owner in an integrated deployment; launching
separate hosts does not establish synchronized operation. No cross-repository
dependency or GigPies integration is introduced by this task note.

## Validation before acceptance

- Known pure delays, polarity inversion and crossover phase differences, measured
  through the actual production DSP. Already aligned paths must retain their settings.
- Silence/noise, clipped or dropped capture, clock/timestamp errors and ambiguous
  periodic correlation peaks must withhold unreliable proposals.
- Nonconstant phase differences and reflected paths must not be misreported as a
  single correctable delay. Test several virtual microphone positions and conflicting
  improvements across them.
- Preserve relative timing when compensating analysis delay. Cover sample rounding,
  zero/maximum delay, unsupported independent L/R requests and out-of-range results.
- Verify mic-to-program isolation, bounded capture/worker behavior, cancellation,
  state recovery and continued output protection during parameter transitions.
- Record a separate physical loopback and multi-position acoustic validation when
  authorized. Offline checks cannot establish room performance or hardware safety.

No new recordings, playback, host changes, implementation or builds were performed
for this note. Listener evaluation and hardware acceptance remain separate steps.
