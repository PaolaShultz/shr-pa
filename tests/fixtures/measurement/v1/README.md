# Invented measurement v1 interoperability corpus

These small JSON results were emitted by the real `shr-pa-measure` offline CLI.
They contain no recordings or sample arrays. Inputs are invented 32768-sample
xorshift64 noise: seed 17; shifts left13, right7, left17 modulo 2^64; interpret each
state as signed i64, divide by i64::MAX and multiply by 0.03. Anchor is identical;
target is delayed 24 samples and inverted, with zero padding. Both carry the exact
metadata retained in the result files. Options are the documented defaults.

- `anchor-result.json`: eligible zero-arrival measurement.
- `target-result.json`: eligible 24-sample inverted measurement.
- `proposal.json`: real proposal for two full-range independent outputs sharing
  input0; target already has delay0.5ms and inverted=true. The proposal delays the
  anchor24 samples and toggles target inversion off. The complete base graph is
  retained, including limiter/protection and mute state.
- `candidate.json`: owner-validated candidate operation on the proposal/current
  base graph; retains basis identity and returns serialized graph for muted review.
- `refusal.json`: duplicate-key request rejected by the real decoder, exit2.

Floating-point fields are interoperability examples, not bit-identical numerical
requirements across architectures. Normal tests deserialize and validate the
corpus, compare the proposal semantics, and independently validate production DSP
numerics. Generator input/request files remain private disposable artifacts.

See [the owner contract](../../../../docs/MEASUREMENT_V1.md). These software-only
results assert no physical timing, audio capture or acoustic success.
