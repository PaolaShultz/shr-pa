# DriveRack function map — 2 inputs × 6 outputs

**Reference baseline: dbx DriveRack PA2. The initial DSP/offline/ALSA slice is
implemented; the inventory below includes substantial remaining work.** This maps the PA2's documented functions, including setup,
operation and maintenance. It does not claim coverage of every other DriveRack
model or identical proprietary algorithms.

The current scope is a fixed 2×6 processor. Matrix routing, advanced patching,
eight-point positional measurement and the later nine-channel arrangement are
in [future work](FUTURE.md). They are not dependencies of this plan.

## Sources and reading guide

The source is the manufacturer's **DriveRack PA2 Owner's Manual**, checked on
2026-09-29: [official document](https://dbxpro.com/en-US/product_documents/driverack_pa2_manual_5044138-apdf),
[manufacturer PDF hosted by Full Compass](https://www.fullcompass.com/common/files/43013-DriveRackPA2UserManual.pdf).
The official download returned HTTP 403 during this audit; the mirror was readable.
Page references below use printed manual pages, not PDF viewer page numbers.
Also checked: [product specifications](https://dbxpro.com/en-US/products/driverack-pa2)
and [Harman's power-up mute explanation](https://help.harmanpro.com/en_US/driverack-pa2/driverack-pa2-powerup-mute-state).

The tables preserve complete target requirements. They are not current CLI/API
contracts: for example, distance entry, limiter bypass/knee and automatic EQ
history remain planned. Use [DSP](DSP.md) for implemented parameter ranges and
the completion summary below for partial coverage.

Each row has a stable ID, a source location, our implementation plan, and an
acceptance check. P1–P8 refer to the [implementation sequence](ROADMAP.md).
A row is complete only when its implementation and evidence are linked here.
Hardware-specific and vendor-specific differences are recorded rather than
silently omitted. Parameter limits below are compatibility targets; algorithm
behavior must be measured independently.

## A. Configuration and audio structure

| ID / reference | SHR PA plan | Acceptance / stage |
| --- | --- | --- |
| C01 — input mode; pp. 14, 50–55 | Stereo L/R and single-input mono from L feeding the paired chains. A separate, explicitly labelled L+R averaging operation supports mono bass. Do not confuse a single mono input with summing two inputs. | Impulses on L/R prove source selection; correlated and inverted inputs prove sum gain/cancellation. P2 |
| C02 — output configurations; pp. 50–55 | Full-range, sub/satellite, two-way, three-way and the full-range variations listed below. Six named logical outputs; inactive outputs produce zero. | Reference output for every configuration and source mode, including unused outputs. P2 |
| C03 — paired processing; pp. 29, 39–44, 60 | Low, mid and high stereo pairs share their band settings; each physical channel has its own DSP state and mute. GEQ can use linked or separate L/R settings. | No state leakage across channels; linked edits apply together. P2 |
| C04 — signal order; p. 60 | Preserve the reference processing order described in Architecture. Measurement input is analysis-only and never enters the PA mix. | Whole-chain reference render; mic-to-output isolation; noise-generator insertion test. P2/P4 |

### Configurations to support

Logical output names: **H-L, H-R, M-L, M-R, L-L, L-R**. These are logical labels;
physical sockets are chosen explicitly from the available interface channels.
A stereo card does not reduce the six logical outputs. UMC1820 is a future target.

| Configuration | High pair | Mid pair | Low pair |
| --- | --- | --- | --- |
| Full-range mains | Full range | Off | Off |
| Powered sub/satellite with external crossover | Full-range feed to external system | Off | Off |
| Mains + subs | Mains band | Off | Sub band |
| Bi-amped mains without subs | Upper main band | Off | Lower main band |
| Bi-amped mains + subs / three-way | High band | Mid band | Low band |
| Six full-range feeds | Full range | Full range | Full range |
| Four full-range feeds + subs | Full-range/main band | Same source with its own band settings | Sub band |

Provide mono-input and stereo-input variants where applicable. Bass may be stereo
or a mono sum. For a single mono sub, use L-L and mute L-R. The implemented sum is `(processed_L + processed_R) / 2`, with documented gain. Its scaling is our design
choice, not a claim about undocumented dbx coefficients.

These are fixed configurations and band options within 2×6, not a user-editable
routing matrix. Preserve the manual's preset families without copying unverified
vendor speaker tunings or assuming their crossover frequencies suit our speakers.

## B. Processing

| ID / reference | SHR PA plan | Acceptance / stage |
| --- | --- | --- |
| D01 — graphic EQ; pp. 27–28 | 31 fixed bands, linked stereo or dual mono; bypass; flat/manual and curves for small-venue music, speech, larger-system performance and recorded music. Preserve manual settings when auditioning curves. Author our own curve coefficients. | Individual/combined response, stereo links, gain headroom and curve/restore behavior. P3 |
| D02 — input room EQ; pp. 29–30 | Dedicated eight-band PEQ before crossover, independently bypassable from GEQ. Bell and both shelving types; editable result plus stored automatic and manual versions and a flat view. | Frequency/gain/width response, stable extremes, correct restore source. P3/P5 |
| D03 — speaker EQ; pp. 29–30 | Eight-band PEQ for each active output pair; same filter types, bypass, flatten and restore. Keep speaker corrections separate from venue corrections. | Per-band responses and pair isolation; flatten/restore round-trip. P3 |
| D04 — automatic feedback suppression; pp. 31–34 | Twelve filter slots with configurable fixed/live allocation, speech/mixed/music width policies, bypass that also stops detection, inspection of each filter’s frequency/Q/depth, protected no-clear/live-only/all clearing, live replacement and timed gradual lifting. Detection works from program inputs; no RTA mic required. | Feedback onset, false triggers, all-slot exhaustion, allocation changes, clear/lift and anti-phase input cases. P6 |
| D05 — subharmonic synthesis; pp. 35–36 | Mono-derived bass generation mixed into stereo dry audio, two low-frequency regions, overall and separate band amounts, bypass and three effect meters. Implement an original octave-down generator. | Expected subharmonic energy, no DC, bounded output, stereo dry preservation, artifact listening tests. P6 |
| D06 — input compression; pp. 37–38 | Broadband stereo-linked compressor with threshold, ratio, makeup gain, adjustable hard/soft knee, bypass, gain-reduction and threshold-region meters. Choose and document envelope timing rather than inventing dbx timing constants. | Static transfer, knee continuity, bursts, stereo-image stability and bypass transitions. P3 |
| D07 — input/backline delay; pp. 39–40 | Delay the complete stereo program before crossover; enable and time/distance entry. Keep it separate from driver alignment. | Sample timing, unit conversion, bypass and transition behavior. P2 |
| D08 — crossover; pp. 41–42 | Per-pair high-pass and low-pass with independent frequencies, bypassed edge options, gain and polarity. **LR24 first and default.** BW6–48 and LR12/24/36/48 are implemented in independent mode; retain the original layout phase policy as a separate default mode. Allow intentional overlap/full-range settings. | Each edge's response, polarity, bypass, and complete two/three-way sums. P2/P3 |
| D09 — output limiting; pp. 43–44 | Three stereo limiter groups, bypass, threshold and knee controls, activity and gain-reduction meters. Calibrate thresholds against the actual interface/amplifier chain. | Burst overshoot, sustained overload, release, pair linking and threshold calibration. P2/P3 |
| D10 — driver alignment; pp. 39–40 | Separate output delay per band pair, time/distance entry and bypass. Never replace it with the input delay. | Relative alignment across all six outputs, buffer wrap and click-free changes. P2 |
| D11 — processing state; pp. 24, 27–44 | Direct module access; select active band; edit values; explicit bypass; visible current values. Preserve state through unrelated edits and preset preview. | Command/UI consistency, finite/range validation and state restoration. P3/P7 |

### Initial control limits

These limits make the plan concrete. Public parameter ranges do not establish
proprietary coefficient curves, detector thresholds or time constants.

| Module | Planned compatibility range |
| --- | --- |
| GEQ | 20 Hz–20 kHz bands, ±12 dB, 0.1 dB edits |
| PEQ | Eight bands; 20 Hz–20 kHz; ±12 dB; bell Q 0.1–15.909; shelf slope control covering 3–14.295 dB/octave |
| Compressor | Threshold −60–0 dBFS; ratio 1:1 to limiting; makeup ±20 dB; hard knee or ten increasing soft-knee settings |
| Limiter | Threshold −60–0 dBFS; hard knee or ten increasing soft-knee settings |
| Input delay | 0–100 ms; nearest-sample resolution, with ms/metres/feet display |
| Output delay | 0–10 ms per pair; nearest-sample resolution, with the same units |
| Crossover | 16 Hz–20 kHz or edge disabled; band gain −60 to +20 dB; normal/inverted polarity |
| Implemented crossover slopes | Butterworth 6 through 48 dB/octave in steps of 6; LR 12, 24, 36 and 48 dB/octave |
| Feedback filters | 12 total positions; fixed allocation 0–12; live-lift timer 5 seconds–60 minutes |
| Bass synthesis | 24–36 Hz and 36–56 Hz output regions; each amount and overall amount 0–100% |

Shelf-slope translation needs reference response tests; it must not be treated as
an arbitrary Q knob. LR slopes require their appropriate polarity and phase
handling. Do not assume the LR24 sum test proves other slopes correct.

No claim of identical OverEasy, PeakPlus, AFS or subharmonic algorithms is made.
Our processors will have documented behavior and measured limits. The limiter
plan includes a measured overshoot bound; lookahead, if needed, gets an explicit
latency cost. Unknown proprietary constants are not fabricated.

## C. Measurement and guided setup

| ID / reference | SHR PA plan | Acceptance / stage |
| --- | --- | --- |
| M01 — measurement input; pp. 4, 15–16 | One setup microphone channel, separate from the two program inputs. Flat response or a verified calibration file; level/clipping indication. | Known calibration curve, clipping rejection and no monitor-to-PA route. P4 |
| M02 — real-time analyzer; p. 45 | 31 displayed bands, slow/fast response, graph offset, peak hold and selectable bar/peak views. Track the reference's six display choices; adapt their presentation to the terminal. | Pink-noise flatness, sine location, offset/hold timing and small-screen legibility. P4/P7 |
| M03 — test generator; p. 45 | Pink and white noise, output level, off/on state and immediate stop. The generator occupies a defined program-input insertion point before processing; stop restores the program source. | Noise statistics/spectrum, level, startup-off, cancel/fault-off and insertion-path test. P4 |
| M04 — automatic EQ; pp. 2, 15–16, 29 | Swept measurement and an eight-band room-EQ fit. Begin with one microphone and support the PA2-style sequence of up to four positions. Target choices: flat, bass-shaped PA response, and a more damped high end for reflective rooms. | Known transfer functions, poor-data rejection, repeat measurements and independent post-EQ verification. P5 |
| M05 — level balancing; pp. 15–16 | Compare L/R and active band levels; show amplifier-adjustment guidance. Optional small automatic trims, displayed and removable in our UI. Run alone or with automatic EQ. | Known imposed level errors, headroom checks, repeatability, trim removal and cancellation. P5 |
| M06 — system setup; pp. 14–17 | Select mono/stereo, GEQ linking, speaker arrangement, passive/bi-amped mains, subs and amplifier/speaker profiles. Offer a manual/unlisted path. Generate a reviewable configuration and retain selections. | Every 2×6 configuration, partial rerun, missing profile, back/cancel and repeat setup. P5 |
| M07 — wizard orchestration; pp. 14–16 | Run setup, measurement and feedback stages together or separately; choose existing/new settings. Preserve completed work when cancelling a later stage. Optional generated preset names. | Resume/abort, microphone absence, bad measurement and unchanged unrelated settings. P5/P6 |
| M08 — guided feedback ring-out; pp. 15, 23, 31–34 | Explicit setup mode using fixed filters, completion/abort, then live tracking. Show occupied slots and keep manual ring-out available. | Controlled feedback model, fixed-to-live handoff, cancellation and no automatic restart. P6 |
| M09 — manual commissioning; pp. 18–23 | Provide the manual equivalents: crossover setup, polarity/alignment, gain and limiter calibration, band balance, room EQ and feedback checks. Wizards must not be the only way to operate. | A complete setup can be completed without a profile or automatic analysis. P5/P7 |

Plan RTA display offset 0–40 dB and peak hold 0.5–5 seconds; generator gain
−60–0 dBFS, with a conservative initial setting. For level balancing, support
small trims up to 3 dB and a 1 dB comparison tolerance as initial compatibility
targets. Show failed/noisy measurements instead of applying a guessed correction.
Use independently documented target curves and calibration data.

The four-position sequence is ordinary PA2 measurement coverage using one moved
microphone. The requested **eight-point positional RTA**, including possible
simultaneous microphones, is separate future work.

The requested [phase/delay alignment extension](PHASE_ALIGNMENT.md) adds a
reference/mic transfer-function measurement in P4 and a delay/polarity proposal
stage in P5, using D10 and supporting M09. It is our additional development task,
not an assertion of automatic phase alignment in the PA2 reference. SHR PA owns
this work for its standalone processor and eventual GigPies module integration.

## D. Presets, controls and system functions

| ID / reference | SHR PA plan | Acceptance / stage |
| --- | --- | --- |
| O01 — metering; pp. 5, 13, 35, 37–44 | Two input and six post-mute output meters; held clip indication; compressor/limiter threshold and reduction; bass-generator meters. Show the active preset and modified state. | Meter tap locations and thresholds, held peaks, mute visibility and stale-data detection. P2/P7 |
| O02 — output mutes; pp. 5, 46–47 | Six independent mutes, global rather than preset-owned. Provide remembered or all-muted startup policy, plus one-time forced-muted startup. Default to all muted. | Recall cannot unmute a muted output; persistence and restart combinations. P2/P7 |
| O03 — navigation/home views; pp. 4, 11–13, 24 | Signal-chain, dynamics, RTA and system-info views; return/back, quick module access and band stepping. Keep keyboard/touch commands equivalent. Remember chosen home view. | All functions reachable at 40×13, resize recovery, no accidental edits during navigation. P7 |
| O04 — preset management; pp. 25–26, 55 | At least 75 user slots plus read-only factory templates; select-before-recall, edit, name, save, quick-save and copy. Keep an unsaved working state and explicit modified indication. | Recall/cancel, overwrite/copy isolation, atomic save, corrupt files and power-loss recovery. P3/P7 |
| O05 — state ownership; pp. 5, 25, 46 | Presets own processing and setup selections. RTA preferences, utility settings and mutes are global. Recover the working edit independently of a stored preset; never persist a running test signal as auto-start intent. | Restart/recall combinations; ownership schema tests; pending writes handled on shutdown. P3/P7 |
| O06 — speaker/amplifier profiles; pp. 17, 55–56 | Validated local profiles containing documented EQ/crossover/polarity/delay/limiter data and provenance; manual entry; optional catalog import/update. Generic configurations work without a vendor profile. | Units/version checks, unavailable catalog, invalid data and known profile application. P5/P8 |
| O07 — system information; pp. 13, 46 | Version, device name, audio identity/settings and network information. | Values come from the actual running system; unavailable data is labelled. P7 |
| O08 — display preferences; p. 46 | Readability/theme setting, optional hardware brightness/contrast adapter, home timeout with disable, and optional demonstration/banner view. Track all reference utility options even where terminal presentation differs. | Preference persistence, timeout does not discard an edit, demo never opens audio. P7 |
| O09 — control lockout; p. 48 | Unlocked, locked, locked with feedback-clear access, or feedback-clear plus mutes. Define permissions centrally for local/remote commands, with a documented recovery entry point. | All four modes, rejected writes, allowed clear/mutes and recovery. P7 |
| O10 — resets; p. 49 | Reset global preferences while retaining user presets; separate full reset of preferences/presets. Show affected state and allow cancellation. | Cancellation leaves data intact; partial reset preserves presets; full reset returns known defaults. P7 |
| O11 — remote operation; pp. 6, 9, 56–58 | Network-capable control boundary with authenticated remote terminal access first. Remote clients can view meters/RTA, operate processing, run setup and manage presets through the same commands. Local use remains self-contained. | Disconnect/reconnect, stale edits, permissions, concurrent clients and no audio stalls. P8 |
| O12 — remote identity/security; pp. 46, 57–58 | Device name, visible address, credentials/access policy and connection diagnostics. Use OS network setup and secure transport; do not copy the vendor's default-password convention. | Unauthorized clients rejected; credentials do not enter presets/logs; network loss preserves audio. P8 |
| O13 — software maintenance; pp. 6, 59 | Versioned releases, local/removable-media or network installation, configuration migration, backup and rollback. Updates happen outside an active audio session. | Failed update/rollback, old schema migration and preservation of user settings. P8 |

Plan the home timeout choices at 10/30 seconds, 1/2/3/4/5/10 minutes, or disabled.
A physical contrast adapter exposes only controls actually supported by the screen;
the terminal theme remains available independently. The display-only banner/demo
option is included in the inventory and never starts a test signal.

Remote phone/tablet/computer control remains part of the function plan. A vendor
GUI, binary preset compatibility and the dbx control protocol are not claimed;
the terminal and an original control protocol provide our functional counterpart.
An online tuning service is optional infrastructure for O06, not required to boot
or load local tunings. Exact manufacturer tuning coefficients remain dependent on
verified, usable source data. Generic templates are not branded speaker presets.

## E. Hardware and vendor-specific counterparts

| ID / reference | Treatment in SHR PA | Evidence needed / stage |
| --- | --- | --- |
| H01 — analog I/O and converters; pp. 3–7, 64 | Eventually qualify the UMC1820 for two program inputs, one setup-mic input and six outputs. Develop DSP now on available interfaces with explicit partial physical maps. Logical count stays 2×6; measurement capture is separate. USB stream width may be larger. | Physical mapping, format/rate, full-scale voltage, noise/crosstalk/latency; no borrowing PA2 measurements. P1/P8 |
| H02 — input sensitivity and ground-lift controls; p. 7 | Document actual interface gain/pad/line controls and cabling. Software cannot supply an analog ground lift or undo ADC overload. | Bench calibration and wiring guide for the actual hardware. P1 |
| H03 — microphone power and response; pp. 4, 16, 64 | Use the interface's actual phantom-power controls and verified microphone calibration. Do not assume dbx's phantom voltage or RTA-M correction is present. | Mic compatibility and actual power grouping confirmed. P1/P4 |
| H04 — Type IV conversion; pp. 3, 64 | Explicit hardware difference: the UMC1820 does not become a dbx Type IV converter. Provide clipping/headroom indication and measured input calibration. | Record the distinction; no claim of matching proprietary ADC behavior. P1 |
| H05 — physical controls/enclosure; pp. 4–7, 63 | Touch terminal replaces the rack LCD/buttons; Pi/interface provide power, connectivity and enclosure requirements. No need to reproduce rack dimensions or rear-panel firmware USB. | Physical control, power/thermal and restart acceptance on the actual build. P7/P8 |

## Completion rules and audit coverage

Audit sections accounted for: front/rear controls; operating modes and home
views; all wizard options; manual commissioning; presets; every processing
module; RTA/generator; utility and power-up functions; application/preset families;
remote application/networking; updates; signal order; hardware specifications.
Warranty/service terms and vendor support contacts are documentation, not DSP or
control functions to implement.

Rows retain their complete target scope. The 0.2 alpha baseline covers C01 and
most C02/C03, D01 linked/separate 31-band GEQ with bypass and original manual/flat/tonal curves,
D02/D03 bell/shelf PEQ with bypass and scoped flatten/restore (automatic EQ history/PA2 slope units pending),
D06 linked compressor with hard/soft knee and explicit timing, D07 input delay,
D08 independent BW/LR edges plus default compensated LR24, D09 hard sample limiter
and D10 output alignment delay, plus
initial D11/O01/O02/O04/O05 controls and presets. C04 remains partial because
the other planned modules are absent. M03 has deterministic white and pink noise
and other bench sources; [pink-source evidence](verification/0008-pink-noise.md)
covers bounded generation, spectrum and offline/null integration. Session-start
generator level selection (−60…0 dBFS peak bound) is now implemented with
[level evidence](verification/0009-generator-level.md). In-session level edits
now use a bounded 5 ms ramp; see [runtime evidence](verification/0010-runtime-generator-level.md).
Runtime off/on now crossfades to/from mapped capture; see
[capture restoration evidence](verification/0011-generator-capture-restore.md).
M03 remains partial: changing source type during a session is pending, and
physical transition acceptance remains unmeasured. RTA/setup-mic workflows remain pending.
Live transactions, filter/delay transitions and compact module controls extend
D11/O01/O04/O05; see [new evidence](verification/0004-live-controls.md).
The [preset/EQ slice](verification/0005-preset-library.md) extends D01–D03/D11
and O04/O05 with 75 named user slots, six immutable templates, explicit copy/save,
select-before-recall, independent working recovery and preserved manual EQ state.
Future profile/setup selections, automatic input-EQ sources, utility preferences
and remote writers remain pending. D08 now has offline response/phase, migration and software-null live-control
[evidence](verification/0006-crossover.md); physical response remains unmeasured.
No broader row is claimed complete. H01 has short stereo AudioBox transport
evidence; six-channel physical qualification and calibration remain pending.
See [DSP contract](DSP.md), [status](STATUS.md),
[tests](../tests/engine.rs) and [bench evidence](verification/0003-engine.md). Exact proprietary response,
undocumented app-only behavior and vendor profile data require separate evidence;
this is a complete map of the public manual's function categories, not a claim
of reverse-engineered firmware equivalence.

Future implementation changes should link commits/tests beside these IDs rather
than replacing the inventory with a claim that the system is “equivalent.”
