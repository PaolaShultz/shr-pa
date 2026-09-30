# DSP contract and reference behavior

`config` owns validation/persistence; `control` owns the bounded handoff;
`dsp` owns synchronous processing;
`offline` owns sources/WAV I/O; `transport` owns ALSA/conversion; `ui` and
`commands` own terminal/control operations. No general routing graph is compiled.

## Versions and state ownership

The application release is **0.2.0-alpha.1**. It does not change the processing
schema **v3** or library/working envelope **v2** introduced before this release.
Serde types in [config.rs](../src/config.rs) and [library.rs](../src/library.rs),
plus their validators, define the accepted JSON; no separate JSON Schema file
is shipped. `init` emits a complete default processing document.

| State | Owner | Persisted / applied |
| --- | --- | --- |
| Processing parameters | Controller desired Config; audio Engine applied Config | v3 standalone JSON or embedded in envelopes; prepared transaction at block boundary |
| GEQ manual arrays and PEQ restore points | Controller library state | v2 slots and working record; absent from standalone export |
| Selected slot, active ID and saved comparison baseline | Controller | Working record; selection alone does not recall |
| Filter/delay history and dynamics envelopes | Audio Engine | Never serialized; unchanged sections survive edits |
| Mute targets and fault latches | Runtime control/audio state | Never serialized; startup muted, faults require reconstruction/restart |
| Device map and generator request | Explicit live command | Never restored from a preset or working record |

The displayed desired snapshot can lead the applied engine while pending/busy.
Saving records desired processing, not a claim that audio has settled. See
[transactions](#transactions-and-transitions) and [library state](#library-and-working-state).

## Prepared render

`Engine::new(Config)` validates every parameter, computes f64 coefficients,
allocates and initializes delay lines, and starts all six mutes closed.
`render(&[[f32; 2]], &mut [[f32; 6]])` accepts equal frame counts up to the prepared
maximum, including short final blocks. It performs bounded scalar processing
without allocation, deallocation, locks, filesystem, terminal, or analysis work.
The allocation regression includes full EQ/delay processing, mute commands and
numerical faults. Coefficients/state use f64; inputs/outputs use f32. Filter state
below 1e-30 is cleared to prevent persistent denormal tails.

The chain is input peak/clip meter → input mode → gain → 31-band GEQ → eight
input PEQs per channel → stereo-linked compressor → input delay → crossover →
band gain/polarity → eight paired output PEQs → paired limiter → alignment
delay → linked ceiling guard → mute ramp → output peak meter.
PEQ uses the [Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/)
bell and shelf formulas. Zero dB is flat. Module details and transitions follow below.

## Layouts and crossover

Processing schema v3 makes the phase policy explicit. `crossover: "layout_lr24"`
is the default and uses the original table below, including the compensated
three-way tree. All six templates and migrated v1/v2 presets select this mode.

Pairs are always high (0/1), mid (2/3), low (4/5).

| Preset `layout` | High | Mid | Low |
| --- | --- | --- | --- |
| `full_range` | Direct program | Zero | Zero |
| `external` | Direct feed to external crossover | Zero | Zero |
| `two_way` | LR24 HP at `low_hz` | Zero | LR24 LP at `low_hz` |
| `three_way` | HP(low) then HP(high) | HP(low) then LP(high) | LP(low) then AP(high) |
| `six_full_range` | Direct | Direct | Direct |
| `four_plus_subs` | HP(low) | HP(low), own pair processing | LP(low) |

Mains+subs and bi-amped mains without subs share `two_way`; choose the split
frequency for the application. The default `four_plus_subs` has mains high-passed. Independent mode below
allows bypassed mains edges and independent overlap/gaps.

Every LR24 edge is two cascaded second-order Butterworth sections. A two-way
LP+HP sum is unity magnitude with second-order all-pass phase. For three-way,
splitting the upper branch again adds the upper crossover's all-pass phase.
**The low branch therefore receives the equivalent upper second-order all-pass.**
The total is AP(low) × AP(high), rather than an uncompensated low band plus
phase-shifted upper bands. This introduces frequency-dependent phase/group delay,
not an extra audio block or fixed lookahead delay. Crossovers preserve normal
polarity; intentional pair polarity/gain/EQ/delay changes can change the sum.

`tests/engine.rs` computes complex references from analog Butterworth prototypes
with bilinear frequency warping. It compares each band and the complete sum's
magnitude **and phase**, using impulse DFTs at 44.1/48/96 kHz. It does not call
production coefficient-design code. Flat summed magnitude requires coherent
stereo bass and equal/flat pair processing; mono bass changes stereo sources.

`stereo` preserves L/R. `mono_left` selects L for both input chains, independently
of bass summing. `mono_bass` uses `(processed_L + processed_R) / 2` for the low
branch in crossover layouts. Correlated inputs retain their level; opposite
polarity inputs cancel. It does not turn six full-range feeds into a bass system.

### Independent edges (D08)

`crossover: {"independent": [HIGH, MID, LOW]}` stores three pair objects. Each has
required `hp` and `lp` objects with `bypass` (boolean), `hz`, `family`
(`butterworth` or `linkwitz_riley`) and `slope` (dB/octave). Butterworth supports
6, 12, 18, 24, 30, 36, 42, 48; LR supports 12, 24, 36, 48. Every edge, including
bypassed/inactive ones, validates independently at 16 Hz to min(20 kHz, 0.45 ×
rate). There is no ordering constraint between edges or pairs. Stored shortcut
frequencies still require low < high; they do not control independent edges.

Each pair receives direct processed program (or the existing mono-bass average
for the low pair in crossover layouts), then HP, then LP. Bypass is identity,
retaining the cutoff/family/slope for later use. Layouts still determine active
outputs; inactive channels are explicitly silent. Layout edits in independent
mode retain the stored edges. Physical channel mapping never changes processing.

Selecting independent mode in the editor seeds LR24 edges from the layout:
H=HP(high), M=HP(low)+LP(high), L=LP(low) for three-way; H=HP(low), L=LP(low) for
two-way; four-plus-subs also seeds M=HP(low). Other edges start bypassed.
**Independent three-way removes the shared HP(low) from H and AP(high) from L.**
It has no automatic phase compensation. The mode is displayed before editing;
changing modes uses mute/reconfigure and retains all gain, polarity, EQ, delays,
dynamics and runtime mute choices. Returning to layout mode discards custom
edges and reinstates the original tree. Entering independent again reseeds edges.
Save a slot first to retain a custom crossover across these deliberate resets.

Split shortcuts (`[]`, `{}` or the split editor fields) work only in layout mode;
in independent mode they reject with a mode instruction. They never rewrite a
custom edge implicitly. Edge fields require selecting independent mode first.

Butterworth uses the bilinear transform of the normalized order-N analog
prototype, with prewarped cutoff. Odd orders include one first-order section.
LR doubles the corresponding half-order Butterworth cascade. Butterworth edges
are −3.0103 dB at cutoff; LR edges are −6.0206 dB. Slopes are asymptotic analog
slopes; digital frequency warping steepens the response near Nyquist.

| Matched two-way HP/LP at the same cutoff | Required relative polarity for unity magnitude |
| --- | --- |
| LR12, LR36 | Invert one pair manually |
| LR24, LR48 | Same polarity |
| BW6, BW18, BW30, BW42 | Same polarity gives unity magnitude; opposite also preserves magnitude with different phase |
| BW12, BW24, BW36, BW48 | Neither polarity gives a flat sum |

These guarantees assume coherent input, only the two matched complementary edges,
equal gain, flat EQ, equal delay and no dynamics reduction. Four-plus-subs follows
the same rule for **one** mains pair plus the sub pair; adding both identical mains
pairs doubles the mains contribution. Layout LR24 three-way retains the original
AP(low) × AP(high) guarantee. Independent three-way, cascaded bandpass edges,
unmatched frequencies/slopes, bypass combinations, overlaps/gaps, arbitrary
polarity/gain/EQ/delay changes and mono-bass stereo mixtures have no flat-sum
guarantee. No gain or polarity is changed automatically to hide a difference.

`tests/crossover.rs` multiplies analog prototype poles independently, then compares
impulse DFT magnitude and phase for every order and both edges at 44.1/48/96 kHz.
It also checks warped stopband slopes, cutoff levels, bypass, extreme frequencies,
stereo pairing, layout silence, mono bass and matched sums with explicit polarity.

## Delays, mutes, meters and faults

Input delay is 0–100 ms; each band alignment delay is 0–10 ms, rounded to the
nearest sample. Independent per-channel circular buffers preserve stereo state.
There is no additional delay when a setting is zero. Maximum-capacity delay
lines are allocated at construction, even for zero delay: 12,488 f64 slots at
48 kHz (about 98 KiB). Live edits crossfade two read taps over 20 ms, retaining
the existing history; there is no moving read head or pitch glide.

Mutes ramp linearly over 5 ms, per logical output, and take effect at the next
render call. Reversing a ramp continues from its current level. Inactive outputs
are zero. Input/output peak meters reset each block; input clip flags latch.
Limiter reduction is the maximum dB reduction within the block. The live UI
polls approximate atomic meter snapshots at 10 Hz and reports processed blocks;
no audio or meter queue can hold the audio thread. The terminal shows input/output
peaks, held input clip bits, compressor and limiter reduction, transaction counts,
pending/busy state and faults. These are approximate per-block snapshots, not
coherent measurement records or held peak displays.

Non-finite input, non-finite output-stage values, or internal magnitudes over
1e12 latch a fault. The entire affected output block and all later blocks are
zeroed. Reconstructing the engine is the explicit reset. Bad render lengths
return an error and clear the supplied output slice. Invalid presets never
replace the current engine. No fault bypasses protection.

## Initial limiter

Three stereo-linked, hard-knee **sample peak** limiters are always enabled.
Threshold is −60…0 dBFS; default −1 dBFS. Release is 1…2000 ms; default 100 ms.
For each pair and sample, `required = min(1, threshold / max(abs(L), abs(R)))`.
Gain is `min(required, 1 - (1 - previous_gain) * exp(-1/(rate*release_seconds)))`.
Attack is immediate, release approaches unity exponentially, and both channels
receive the same gain. There is no lookahead and no overshoot above the configured
sample ceiling, apart from final f32 rounding. Alignment delay edits use convex crossfades. A final stereo-linked ceiling guard
protects samples already in delay history when the ceiling is lowered. The guard
uses the same instantaneous threshold; both channels receive its minimum required
gain. The displayed reduction is the block maximum from either limiter or guard.
Mutes follow protection. Inactive layout outputs are explicitly zero, including
filter tails retained across a layout edit.

This is deliberately an initial limiter: it is not a true-peak/inter-sample
limiter, soft-knee model, thermal/RMS protector or calibrated loudspeaker safety
system. It can distort sustained overload. Tests cover bursts, sample ceiling,
stereo gain linking and the one-time-constant release value. Physical amplifier
and speaker calibration remains necessary before claiming protection.

## Graphic EQ and shelving PEQ

GEQ is 31 cascaded bell sections at nominal ISO third-octave centers:
20, 25, 31.5, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400, 500,
630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000,
10000, 12500, 16000, 20000 Hz. Each uses Q=4.318 and ±12 dB gain.
Neighboring filters interact; gains are individual section center gains, not an
interpolated target curve. `geq.enabled` bypasses processing while retaining
settings. `linked=true` uses the stored left settings for both channels, with
independent state; unlinking restores the retained right settings. Bands above
0.45 × sample rate become identity sections, retaining their stored values.
They are not folded or moved to lower frequencies. GEQ defaults bypassed/flat.
Manual/flat and original tonal curves are controller operations; see below.
Automatic EQ remains pending.

Input PEQ has eight independent sections per channel; output PEQ has eight
sections per stereo pair, with separate channel state. `kind` is `bell`,
`low_shelf` or `high_shelf`. `hz` is the bell center or shelf midpoint frequency;
`db` is bell center gain or shelf asymptotic gain, ±12 dB. Bell `q` is
0.1–15.909. Shelf `slope` is the dimensionless Cookbook S, 0.1–1; S=1 is the
steepest monotonic shelf in this implementation. It is **not** Q or dB/octave.
Unused Q/S values are retained. The PA2's slope-unit compatibility is pending.
Input EQ and each pair EQ have their own `eq_enabled` bypass. Frequency bounds
remain 20 Hz to min(20 kHz, 0.45 × rate). Independent warped analog shelf
references check complex response, including midpoint and asymptotic behavior.

## Stereo-linked compressor

Compression follows input EQ and precedes the input delay/crossover. The detector
is `max(abs(L), abs(R))` each sample, with a −600 dB floor for logarithms.
Threshold is −60…0 dBFS, ratio 1…100:1, knee width 0…24 dB, makeup ±20 dB,
attack 0.1…200 ms and release 1…2000 ms. This is an original sample-peak model,
not an RMS detector, proprietary OverEasy model or infinite-ratio limiter.

For input level L, threshold T, ratio R and knee width W, let x=L−T and
s=1/R−1. Desired gain reduction in dB is zero below −W/2, s*x above W/2,
and s*(x+W/2)^2/(2W) inside the knee. W=0 gives a hard knee. The gain envelope
in dB is `target + exp(-1/(rate*time_seconds)) * (previous-target)`, using
attack when more reduction is needed and release otherwise. One time constant
moves 63.2% toward a constant target; it does not specify time to full settling.
Both channels receive `10^((envelope_db + makeup_db)/20)`.

Bypass defaults on. The detector continues tracking while bypassed; enabling or
bypassing linearly crossfades dry/compressed gain over 20 ms. Parameter edits
ramp threshold, ratio, knee, makeup and prepared time coefficients over 20 ms.
Reduction meters exclude makeup. Compressor attack allows transients through;
output limiters remain active and are the sample-ceiling protection.

## Transactions and transitions

`Prepared::new` validates an entire v3 snapshot and computes all EQ/crossover
coefficients, gains, taps and time coefficients on the controller. `Handoff`
contains one fixed-size Copy slot with atomic ownership. Publishing to an occupied
slot returns explicit busy. The terminal retains one latest desired snapshot,
coalesces further edits there and retries; it never drops unrelated settings or
builds an unbounded command backlog. A single logical controller owns the snapshot;
the slot is memory-safe with competing callers, but is not a multi-client revision
protocol. Remote writers and stale-revision rejection remain pending.

The audio thread services at most one transaction at each block boundary, only
when its prior transition has completed. The slot uses Acquire/Release ownership,
with no blocking locks, heap ownership transfers or deferred destructor work.
Accepted/rejected counters are acknowledgements; accepted means transition started,
not settled. `busy` includes mute/reconfigure and transition time. There is no
extra audio queue, render allocation/deallocation, I/O or coefficient design.

Gain and signed polarity targets ramp linearly for 20 ms (inversion passes through
zero). Changed biquads run old and new coefficients with a 20 ms linear output
crossfade; coefficients are never interpolated. New filter state starts at zero;
unchanged sections retain their exact state. Changed delays crossfade two taps of
one retained history buffer. Limiter ceiling (linear amplitude) and prepared
release coefficient ramp over 20 ms; the documented ceiling during transition
is the instantaneous ramp value, reaching the requested ceiling after 20 ms.
All related transitions start on the same sample. Rapid requests cannot interrupt
an active crossfade. These transitions may produce audible tonal/phase changes
or temporary cancellation; transparent arbitrary EQ/delay edits are not claimed.

Worst temporary filter work is two evaluations per changed section. With all
modules active, layout three-way has 126 EQ and 18 crossover/all-pass sections.
Independent mode has 48 fixed crossover slots (four per edge per channel), for
a maximum of 348 filter evaluations per stereo frame during a complete edit.
Unused slots are identity. Both mode banks have fixed storage; only one mode
processes. Independent coefficients are checked for finite values and strict
second-order stability during preparation. Mode switches reset only crossover
history under mute; individual edge edits retain all unchanged section state.
Each delay uses at most two reads. Additional storage/work is fixed at startup.

Layout, input-mode, mono-bass, crossover mode/edge/frequency changes and explicit recall
first ramp outputs to zero over 5 ms. At the first block boundary with all ramps
zero, install the transaction, run its 20 ms transitions while held muted, then
resume the current runtime mute targets over 5 ms. The terminal's preset recall
sets all runtime mutes and requires a fresh unmute after completion. Other topology
edits resume the user's existing mute choices. Unmute during pending/busy states is
rejected in the terminal with a retry message. Rate/block changes require restart.
Fault latches take precedence over all transactions, recalls and unmute requests;
no transaction can clear a fault. Transport faults still stop both streams and
require explicit restart; automatic reconnection remains unimplemented.


## Library and working state

`library` owns controller-only persistence and EQ history. Standalone Config JSON
is **v3**. Library presets and working records use **v2 envelopes**, with strict
required fields and unknown-field rejection. They embed validated v3 snapshots;
there is no implicit reinterpretation of legacy presets. `migrate OLD NEW` accepts
v1/v2 standalone processing or v1 slot/working envelopes. It converts both saved
and working processing snapshots, preserves EQ history and metadata, validates
the entire result and publishes to a new destination without replacing existing
files (including symlinks). Sources remain untouched. Legacy recovery is blocked
until explicitly migrated; see the [migration procedure](RUNNING.md#existing-presets-and-library-migration). To put an existing JSON
preset in the library, import with `l`, select a user slot and save. The import
initializes manual EQ history from the imported gains. Export writes effective
processing only; full restore history stays in library/working envelopes.

Each user slot has its own `U1.json`…`U75.json`. Templates are compiled immutable
values. `working.json` holds current processing, GEQ mode/manual arrays, per-scope
PEQ restore points, selected slot, active baseline ID, and saved processing/EQ
baseline for the modified comparison. No transport/device identity, mute mask,
fault state or generator intent is serialized. Recovery compares rate and requested
block size against startup configuration before installation. It cannot clear a
fault; every new engine/session still starts muted.

Writes use a unique create-new temporary file, file sync, rename, and directory
sync on the same filesystem. The library holds an OS file lock for the editor's
lifetime, released on exit/crash; a competing editor cannot write it. External
manual file changes while editing are not a supported multi-writer protocol.
Each file replacement is atomic; slot save and working checkpoint are separate
commits. A crash between them may recover an older working baseline, while the
new saved slot remains available. Temporary files never take precedence over the
last committed record. Invalid recovery blocks automatic replacement until an
explicit archive/reset. Archive and subsequent checkpoint are separate operations;
a crash between them leaves the rejected bytes in the archive. Storage durability
depends on the filesystem/device honoring sync; power-cut hardware was not tested.

GEQ curves set the same 31 center gains on both channels, retaining the existing
link and enable settings. They are original starting points, not room correction,
PA2 coefficients or measured speaker profiles:

| Mode | Center gains |
| --- | --- |
| Flat | 0 dB at every center |
| Speech | −3 dB below 125 Hz; +1.5 dB from 1–4 kHz inclusive; 0 elsewhere |
| Warm | +1.5 dB through 160 Hz; −1.5 dB from 4 kHz; 0 elsewhere |
| Gentle | `clamp(-0.5 * log2(hz / 1000), -2, 2)` dB |

Switching away from manual captures both channel arrays once; switching among
curves/flat retains that capture. Manual restores both arrays exactly, including
the right array retained while linked. A band-gain edit while auditioning first
restores manual settings and then edits the selected band. Enable/link edits do
not change the curve or the retained manual gains. The normal GEQ bypass and
unavailable-band rules still apply. Curves are section gains; the cascaded response
is not an interpolated target. No automatic gain compensation is added.

PEQ flatten affects gains only for the explicitly named input channel or output
pair. It stores all eight sections, retaining frequency/type/Q/S for restoration.
Repeated flatten does not replace the original restore point. Restore consumes
that point; a later flatten starts a new one. Edits made while flat are replaced
by an explicit restore. Module bypass, other channels/pairs, gains, delays,
crossover and dynamics settings are retained. Automatic/manual AutoEQ history
remains future work.

These operations materialize ordinary validated Config snapshots outside audio.
They use the same one-slot handoff, latest-desired retry, 20 ms transitions,
recall mute sequence and fault precedence as existing controls. No library or
history object crosses into render. Unchanged filters, delays and dynamics keep
their existing DSP state. Metadata-only changes require no audio transaction.
