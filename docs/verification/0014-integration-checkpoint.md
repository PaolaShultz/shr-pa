# GigPies USB integration checkpoint — 2026-10-03

This record captures coordinator-reported findings from the ongoing GigPies
hardware task. GigPies owns the measurements and final acceptance in its
`docs/AUDIO_HARDWARE.md`; private commands, recordings and hashes remain in its
`artifacts/audio-hardware/2026-10-03/` evidence directory. This documentation
checkpoint adds no new hardware run or source change in SHR PA.

Measured PA implementation: `390a45f08776693a1078d285048f37d051a4e5d4`.
The shared library remains the one verified in [0013](0013-embedding.md), SHA-256
`0c69949d99d209f4b23c0256a1b80f8b3b9d169d4d09d91ce9f064ea5bfd75a4`.
Later documentation commits do not change that measured library identity.

## Configuration exercised

PreSonus AudioBox USB 96 on Pi 5, two physical capture/playback channels,
48 kHz S32_LE, 192-frame periods and 768-frame buffers. The USB descriptor says
24 valid bits, while negotiated ALSA significant bits reports 32. The host
explicitly extracts the upper 24 bits and validates their bytes against the
recording; it does not claim 32-bit converter precision or pad a 16-bit source.

Two native f64 PA instances run the fixed full-range preset: one audits local
dry output; the final instance processes source plus wet return scaled by 0.25,
followed by a separate conservative host bench ceiling. The exercised source
combines ADC scaled by 0.125 with distinct 997/1499 Hz probes at −36 dBFS.
Generated-only smoke preceded capture with the probe. PA gain, startup ramp
and sample limiter remain those of the [embedding contract](../EMBEDDING.md).

The eight recorded channels are raw ADC L/R, PA source L/R, dry PA L/R and
host DAC submission L/R. They represent four taps of two physical channels.
Pi 4 supplies real source-following FX; its existing DSP is f32 internally.
Recording and network workers remain independent of local PA processing.

## Reported results at this checkpoint

| Experiment | Finding |
| --- | --- |
| 3 s, 10 s and 30 s hardware trials | Recorded hashes and independent dry/FX-to-DAC sample replay matched |
| First 600 s trial, 8 ms wet admission | 28,800,000 frames; zero xruns or recorder gaps; exact dry replay; one late wet return, so the admission target failed |
| Brain 250 ms stall and process termination/restart | 15 s / 720,000 frames of exact dry and recording continuity; both-channel 5 ms wet fade matched within one PCM24 LSB |
| Forced 100 ms host-driver stall | One xrun stopped the session; the retained 96,000-frame take was correctly marked incomplete |
| Fresh session after that xrun | 30 s recovery trial passed |
| Explicitly revised 16 ms wet admission | 30 s / 1,440,000 frames passed exact replay with no loss |
| Repeated faults and final 600 s trial at 16 ms | Pending when this checkpoint was written |

The revised admission budget is explicit; the failed 8 ms target remains in
the evidence. Neither budget is measured capture-to-speaker latency. The PA
preset itself adds zero fixed algorithmic delay; host queues and FX delay are
separate. Exact recorded samples establish the audited software/device-transfer
path, not analogue DAC output or speaker response.

## Remaining limits

No physical return connection establishes analogue round-trip latency or
channel wiring. The amplifier is disconnected; acoustic alignment and physical
speaker protection remain unmeasured. The Superlux USB device is unavailable,
so this checkpoint includes no independent second-device clock measurement.
The fixed full-range integration does not qualify the complete PA workload,
all standalone UI controls, other layouts or venue reliability. Final soak and
recovery acceptance must be read from the owning GigPies report.

Only documentation links, the staged diff and the complete-index publication
guard were checked for this follow-up. Builds, unit tests and hardware runs
were deliberately not repeated during the coordinator's timing measurements.
