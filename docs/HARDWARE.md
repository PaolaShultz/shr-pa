# Hardware capabilities and qualification

The UMC1820 is a future target, **not a prerequisite**. Develop and run the logical
2×6 engine on any supported channel capability; select physical output mappings
explicitly and retain unmapped outputs for offline work. See [running](RUNNING.md).

## Compatibility conclusion

**The UMC1820 is a suitable class-compliant Linux audio candidate. The specific
unit and its low-buffer duplex behavior on this Pi remain unverified.**

The manufacturer's guide documents driverless macOS use. First-hand Linux
reports show the generic `snd-usb-audio` driver. Together these support using
Linux's USB audio stack without a vendor driver; they are not a certification
of every hardware revision, control, clock mode or minimum buffer setting.

Sources:

- [Behringer product page](https://www.behringer.com/en/products/0805-AAN).
- [Manufacturer quick-start guide, hosted by B&H](https://www.bhphotovideo.com/lit_files/155647.pdf), controls and specifications.
- [First-hand Linux device/driver report](https://forums.linuxmint.com/viewtopic.php?p=2624376).
- [First-hand ALSA UMC1820 format report](https://github.com/alsa-project/alsa-lib/issues/135).

## Documented interface facts

The UMC1820 provides eight analog inputs and ten analog outputs, plus digital
I/O. At 44.1/48 kHz in ADAT mode, the published totals are 18 inputs and 20
outputs. At 88.2/96 kHz, optical S/MUX capacity falls to four channels each way.
It uses USB 2.0. Its analog gain and monitoring controls are physical; phantom
power is grouped across inputs 1–4 and 5–8. Main outputs 1–2 have monitoring/level
controls; line outputs 3–10 are separate sockets. Optical mode changes require
care because the device resets. Check the guide before cabling or changing mode.

The current logical allocation is two program inputs, one separate setup-mic
input and six outputs. Exact UMC1820 socket mapping still needs qualification.
The later nine-channel arrangement and possible eight-mic venue measurements
are future work; neither changes the initial 2×6 qualification target.

## Observed here — 2026-09-29

| Item | Observation |
| --- | --- |
| Board | Raspberry Pi 5 Model B Rev 1.1 |
| CPU architecture | aarch64 |
| Memory | Approximately 2 GiB |
| OS | Debian GNU/Linux 13 (trixie) |
| Kernel | 6.18.34+rpt-rpi-2712, PREEMPT |
| Connected audio | PreSonus AudioBox USB 96 |
| UMC1820 | Not present in USB/ALSA enumeration |

The original scaffold inspection was read-only. The subsequent implementation
trial opened direct stereo ALSA with explicit user authorization and the amp off;
see [measurements and limits](verification/0003-engine.md).
The OS in that record was Debian 13; the project target is 64-bit Linux Lite.

The Pi 5 has two USB 3.0 ports supporting simultaneous 5 Gbps operation, per
[Raspberry Pi's specification](https://www.raspberrypi.com/products/raspberry-pi-5/).
The UMC1820 still operates at its own USB 2.0 speed. Physical loopback measurements
will decide the useful port/controller arrangement and latency.

## Qualification checklist when the UMC1820 is attached

1. Record USB identity/revision, interface descriptors, driver binding and USB
   tree; retain the kernel and firmware versions with the evidence.
2. Read `/proc/asound/card*/stream*` and ALSA hardware constraints. Capture and
   playback must be inspected separately. The ALSA report above shows differing
   formats (`S32_LE` capture, `S24_3LE` playback) in one configuration; do not
   assume a shared format or that this report describes our future unit.
3. Enumerate mixer controls and current levels. Confirm whether any software
   output controls affect the signal. Do not blindly apply another user's settings.
4. Verify each physical input/output with a low-level loopback signal, including
   main/headphone/direct-monitor behavior and the effects of physical controls.
5. Test rates, optical modes and external clock lock only for needed configurations.
   If digital expansion is used, verify one coherent clock domain and loss-of-lock
   behavior. Record the channel map for each qualified mode.
6. Sweep supported duplex periods/buffer sizes and measure round-trip latency,
   xruns and thermal behavior. No claimed minimum before this test.
7. Test cold boot, restart, disconnect/reconnect, mode change and power loss with
   amplifiers disconnected; record output transients and recovery behavior.
8. Exercise USB MIDI separately if it is later used. Verify touchscreen input
   independently; USB audio compatibility does not validate a display/controller.

The initial direct ALSA harness is working and exercised on the AudioBox.
UMC1820-specific qualification remains future acceptance work.
See [validation](VALIDATION.md).
