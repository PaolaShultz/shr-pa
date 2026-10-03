# Standalone transfer pacing — 2026-10-03

Started from clean `24edd3ba4244ecc3b16db8292cc272253270e745`. This change
addresses a host scheduling defect found while reviewing GigPies hardware
intervals. It changes SHR PA's standalone ALSA transport and its pure transfer
regressions. DSP, configuration schemas and the versioned C ABI are unchanged.

## Defect and fix

A nonblocking capture read can return the frames that arrived during processing
instead of the requested whole block. The old loop consumed them immediately,
then called ALSA wait for the rest. The configured `avail_min` still required a
full period. With 384-frame periods and 48 frames consumed early, the next
period interrupt leaves only 336 available: wait can skip it, even though those
336 frames would complete the block. This is a concrete avoidable delay.

The transfer driver now queries available frames before I/O and waits until
the complete remaining block is available. Availability queries use
`snd_pcm_avail_update` through the existing ALSA wrapper. Genuine short I/O still
advances exact frame offsets; no prefix repeats. Availability and I/O faults
share the existing retry/error classification. Stop checks, 20 ms maximum waits,
the two-second no-progress deadline and explicit restart on faults remain.
Byte IO avoids repeated typed-format preparation in the transfer loop.

The primary deterministic regression models 48 frames arriving during 1 ms of
processing and 384-frame period interrupts. Its eager-read control skips the
first interrupt and finishes at produced frame 768. The fixed driver waits
before reading and completes at frame 384. Both preserve every sample in the
requested block. Additional checks cover partial offsets after an advertised
full block, availability/wait interruption, availability faults and cancellation
while insufficient frames are available. These are normal production regressions.

## Latency boundaries

The existing startup policy still primes `buffer - period` frames. The default
128/512 request therefore queues 384 frames (8 ms) before other physical timing.
Larger buffer capacity does not require that much prefill in principle; changing
that policy requires separate startup/underrun validation. This pacing fix does
not claim minimum safe buffers or measured standalone latency.

The earlier GigPies 384/3072 bench primed 2688 frames (56 ms), dominating its
roughly 57 ms measured electrical reference/capture offset. The user rejected
that delay for live use. Its sample-continuity evidence remains valid for its
conditions, while live latency acceptance remains open. GigPies owns its own
host fix and subsequent hardware comparisons in `docs/AUDIO_HARDWARE.md`.

## Validation and preservation

Pinned Rust 1.97.1 with committed lockfile and `CARGO_INCREMENTAL=0`. Focused
contract checks passed all 10 tests before the complete normal suite passed all
67 Rust tests. Formatting, Clippy, release build, both normal terminal and
explicit software-null live-control checks, 4 Python publication regressions,
documentation links and the complete-index publication guard also passed.
Reproducible commands are in [validation](../VALIDATION.md); the focused command
is `cargo test --locked --test contracts`.

The measured `390a45f` shared library was preserved before rebuilding under
ignored `artifacts/measured-libraries/390a45f08776693a1078d285048f37d051a4e5d4/`.
Its SHA-256 remains
`0c69949d99d209f4b23c0256a1b80f8b3b9d169d4d09d91ce9f064ea5bfd75a4`.
The rebuilt shared library is byte-identical. The new standalone executable's
SHA-256 is `df5cb60448adb28930611158812cfa9024cc6276915547af357d0660f8bf5011`.
Existing recordings, failure evidence and source history were retained. Free
space was about 35 GiB and the normal target directory about 1.1 GiB before
building. No separate target directory or disposable media renderer was created.

No physical device was opened. Hardware timing, acoustic work, long workload
benchmarks and unrelated historical renderers were intentionally skipped.
No public push or release was made.
