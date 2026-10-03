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

## Initial configuration exercised

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

## Initial checkpoint results

| Experiment | Finding |
| --- | --- |
| 3 s, 10 s and 30 s hardware trials | Recorded hashes and independent dry/FX-to-DAC sample replay matched |
| First 600 s trial, 8 ms wet admission | 28,800,000 frames; zero xruns or recorder gaps; exact dry replay; one late wet return, so the admission target failed |
| Brain 250 ms stall and process termination/restart | 15 s / 720,000 frames of exact dry and recording continuity; both-channel 5 ms wet fade matched within one PCM24 LSB |
| Forced 100 ms host-driver stall | One xrun stopped the session; the retained 96,000-frame take was correctly marked incomplete |
| Fresh session after that xrun | 30 s recovery trial passed |
| Explicitly revised 16 ms wet admission | 30 s / 1,440,000 frames passed exact replay with no loss |
| Repeated faults and final 600 s trial at 16 ms | Pending at the initial checkpoint; later results below |

The revised admission budget is explicit; the failed 8 ms target remains in
the evidence. Neither budget is measured capture-to-speaker latency. The PA
preset itself adds zero fixed algorithmic delay; host queues and FX delay are
separate. Exact recorded samples establish the audited software/device-transfer
path, not analogue DAC output or speaker response.

## Final device-transfer soak and recovery

The later accepted host configuration uses **384-frame periods, a 3072-frame
buffer and 768 frames (16 ms) of wet admission**, still stereo 48 kHz S32_LE.
The smaller-buffer failures and failed 8 ms admission target remain in the
owning host evidence; this result does not qualify those configurations.

The `soak-600s-buffer8-v7` evidence directory under the same private GigPies
artifact root records 600 s / 28,800,000 frames with zero xruns, missing or
expired wet packets, or queue errors. All 600,000 return packets arrived.
All eight PCM channel hashes and direct ADC hashes matched; independent dry
and DAC sample replay and the recording journal were exact.

| Timing measure | p99 | Maximum |
| --- | --- | --- |
| Bounded render section | 1.123 ms | 4.948832 ms |
| Host service cycle | 1.290 ms | 5.232607 ms |
| Network round trip | 0.565 ms | 1.977299 ms |

The period deadline is 8 ms. These are observed software timings, separate from
converter latency. PA implementation and library hash remain as recorded above;
the FX implementation is `6510ead9`. The measured host binary SHA-256 is
`c4b8627790797cda04dc95ff5f3c4454c46ba142f77772f8557123b50694d7c5`.

Repeated v7 packet-fault and Brain-restart trials each ran 15 s with continuous
dry/recorded samples, zero xruns and recovery on both wet channels. The deliberate
100 ms driver stall retained an incomplete 96,000-frame take. These distinguish
independent network recovery from a hardware-driver deadline failure that stops
the local session. GigPies `docs/AUDIO_HARDWARE.md` owns the complete acceptance,
failure history and reproducible experiment records.

## Bounded electrical-return check

After the device-transfer trials above, the user connected both physical outputs
to inputs, set both input gains to minimum, Mixer to Playback and Main to noon.
Separate generated-only PN probes ran for 8 s each at −72 then −54 dBFS with
384/3072-frame period/buffer settings and no errors. No captured sample was
routed back to output. Two distinct bursts verified **left output to input 1**,
with correlation 0.799 and a reference/capture offset of 2739 frames (57.0625 ms).
That offset includes 2688 frames of playback prefill, start-call uncertainty,
USB transport and converters. It is not isolated converter or capture-to-speaker
latency.

The unchanged v7 integrated host then ran generated-only PA/FX/recording for
12 s / 576,000 frames. All eight PCM/direct-ADC hashes, dry/DAC replay and
the journal matched exactly, with zero xruns or wet errors. The left 997 Hz
return gain was −0.674 dB; the right 1499 Hz return was −69.440 dB, about
68.77 dB below the left. The right route is unusable for acceptance under these
conditions. The user suspects its cable and directs use of the working channel;
the measurements do not establish a definite cable or analogue hardware fault.

## Remaining limits

Only the bounded left electrical-return route is verified. The right route,
isolated converter latency and capture-to-speaker latency remain unresolved.
The amplifier remains disconnected; acoustic alignment and physical
speaker protection remain unmeasured. The Superlux USB device is unavailable,
so this checkpoint includes no independent second-device clock measurement.
The fixed full-range integration does not qualify the complete PA workload,
all standalone UI controls, other layouts or venue reliability. Any subsequent
physical measurement must be read from the owning GigPies report.

Only documentation links, the staged diff and the complete-index publication
guard were checked for this follow-up. Builds, unit tests and hardware runs
were deliberately not repeated during the coordinator's timing measurements.
