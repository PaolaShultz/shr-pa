# Offline reference/microphone measurement v1

Implemented owner module `measurement` and standalone `shr-pa-measure` perform
bounded offline analysis at 48 kHz. They open no audio devices and never modify a
running graph, start a generator, capture a microphone, or rearm output. Fixed and
configurable processing ABIs are unchanged. The existing `cargo run` default is
still `shr-pa`. This completes an offline P4/P5 increment, not capture or acoustic
qualification. See [phase alignment](PHASE_ALIGNMENT.md) for those remaining gates.

## Execution and ownership

```sh
cargo build --release --locked --bin shr-pa-measure
# Input is operator-owned offline JSON, never a device selector.
target/release/shr-pa-measure /absolute/private/request.json
# Alternatively read one bounded JSON document until stdin EOF:
target/release/shr-pa-measure - < /absolute/private/request.json
```

One JSON value is written to stdout. Successful measurement/proposal execution
returns exit 0, including a valid proposal whose status is `refused` or `no_change`.
Malformed input or unusable capture returns exit 2 with
`{"contract":"C-PA-MEASUREMENT-REFUSAL","version":1,"status":"refused","reason":"..."}`.
There is no networking, application command or persistent analysis daemon.
The integrating caller owns cancellation, process deadline, EOF and private files;
it must discard late results after cancellation or identity/configuration changes.

The Rust API is `analyze(&CaptureMeta, &[f64], &[f64], &Options)`,
`propose(&GraphConfig, revision, &[PositionPair])`, and
`Proposal::candidate_configuration(current, revision)`. Analysis allocates and is
explicitly outside all render callbacks. The latter returns a reviewable graph
clone; it does not apply it. GigPies must retain its existing single capture owner,
keep reference/setup-mic taps isolated from program/monitor routing, and hand work
to a bounded offline worker. Measurement JSON is evidence, not control authority:
a caller must bind it to its own admitted capture and configuration revision.

## Strict request envelope

All fields shown are required, with no unknown or duplicate keys. Exact operation
names are `analyze`, `propose` and `candidate`; contract is `C-PA-MEASUREMENT`, version is integer
1. The complete document is at most 8 MiB; the parser limits object members to 64,
array entries to 65536 and strings to 256 bytes before typed admission. serde_json's
normal nesting bound remains enabled. Capture IDs below are ASCII graphic strings
of 1..128 bytes; output indexes are zero-based owner graph indexes, never physical
socket numbers. Counters are canonical unsigned decimal strings (no leading zero
except `"0"`). Source epoch, map revision and configuration revision must be positive.

An `analyze` request contains:

| Field | Type/meaning |
|---|---|
| `contract`, `version`, `operation` | Exact envelope described above |
| `capture` | Identity/quality object below |
| `options` | `{"max_arrival_samples":2048,"band_hz":[100.0,10000.0]}` by default |
| `reference`, `mic` | Equal-length arrays of finite f64 samples strictly between -1 and +1 |

Capture fields:

| Field | Type/meaning |
|---|---|
| `id` | Unique capture identity |
| `source_epoch`, `clock_domain` | Common device timeline identity |
| `map_revision` | Caller-owned source/output topology revision; changed mapping invalidates pairing |
| `reference_id`, `reference_tap` | Identity and actual reference insertion point |
| `reference_offset_frames` | Unsigned common timing-offset annotation; retained, not independently zeroed or applied by the analyzer |
| `first_frame` | First paired sample on that timeline |
| `sample_rate` | Integer 48000 |
| `output_index` | Independently measured graph output, less than 4096 |
| `position_id` | Fixed microphone-position identity |
| `configuration_revision` | Caller-owned positive revision of the exact measured graph |
| `dropped_frames` | Must be `"0"` |
| `clipped_reference`, `clipped_mic` | Must be false |
| `timing_verified` | Must be true; the caller supplies timing evidence, this boolean cannot establish clock synchronization |

Samples number 32768..65536 (about 0.68..1.37 seconds). This bounds CPU/storage and
provides multiple spectral windows. `max_arrival_samples` is 1..2048: at 48 kHz the
search covers up to ±42.67 ms, separately from the existing 10 ms output correction
limit. A boundary peak is not admitted for a proposal. `band_hz` must lie within
20..20000 Hz, ascend, and contain at least eight 4096-point DFT bins. Narrower bands
are explicitly unsupported in v1; this is not a product frequency limit.

A `propose` request replaces capture/options/sample arrays with:

- `configuration_revision`: canonical positive decimal string;
- `configuration`: exact validated [GraphConfig v2](EMBEDDING_V2.md);
- `positions`: 1..8 objects each containing `anchor` and `target` measurement results.

Both outputs must exist and have an assigned source. Every position must have the
same ordered output pair, source epoch/map/clock/reference/offset/rate/configuration
revision. Within each pair, position IDs agree; distinct pairs have distinct
position IDs and all capture IDs are unique. Capture frame ranges may differ
because measurements can be sequential. They never acquire independent timing
origins. Every pair must recommend exactly the same integer delay difference and
polarity correction; otherwise the entire proposal is refused.

## Owner-validated candidate operation

A `candidate` request contains the same envelope, `configuration_revision`, exact
current `configuration`, and a `proposal` object previously returned by this owner.
It calls the existing candidate validator and returns
`C-PA-ALIGNMENT-CANDIDATE` version1 with `basis_capture`,
`basis_configuration_revision` and `configuration_json` (the serialized validated
candidate graph). Refused, forged-change, stale-base and mismatched-revision
proposals return the normal exit2 refusal. This operation neither applies settings
nor rearms outputs. The host still verifies current epoch/map/reference identity
and controls the muted review transaction; JSON alone is not authorization.

## Algorithm and result

`C-PA-MEASUREMENT-RESULT`, version 1, algorithm `offline-h1-v1`, contains:
`capture`, `options`, `samples`, `arrival_samples`, `signed_correlation`,
`competing_peak_ratio`, `segments`, `spectrum`, `proposal_eligible`, `reasons`.
Spectrum entries contain `frequency_hz`, linear transfer `magnitude`, wrapped
`phase_radians` in [-pi,+pi], and `coherence` in [0,1]. All numbers are finite.
A measured transfer can remain informative when `proposal_eligible` is false;
this does not authorize a correction. Completely unusable capture has a refusal
envelope rather than invented zero arrival or zero coherence.

1. Verify shape, identity and capture-quality flags, reject clipped/nonfinite
   samples, remove DC, and require mean-square signal above 1e-10 in both inputs.
2. Search signed normalized correlation over a common 4096-sample support starting
   at sample 2048. The coarse search uses every fourth support sample for all
   integer lags; full-support refinement checks ±4 samples around its best peak.
   Report the signed peak and strongest competing absolute peak outside ±4 samples.
3. Compensate the estimated lag only for analysis. Compute selected-bin complex
   DFTs in 4096-sample Hann windows, 50% overlapping, using the entire remaining
   common record (at least 14 windows at the minimum record length). Select up to
   64 uniformly spaced frequency-bin indexes inside the requested band.
4. Average cross/auto power; use H1 = S_yx/S_xx and magnitude-squared coherence
   |S_yx|²/(S_xx S_yy). Restore the estimated arrival's linear phase in the reported
   transfer. Bins require reference power above 0.001 of the selected peak and
   microphone power above 1e-14. A single-window coherence estimate is never used.
5. Proposal eligibility requires absolute correlation >=0.9, competitor ratio
   <=0.5, at least eight usable bins and half the selected bins, and coherence
   >=0.95 at every reported bin. These conservative, versioned software thresholds
   do not establish absence of reflections or direct-path/acoustic certainty.

For each output pair, remove only their *relative* measured arrival from the phase
difference. Compare normal/inverted polarity over common bins; require at least
eight bins and residual phase RMS <=15 degrees. Otherwise refuse a delay/polarity
model. Reflections, narrowband periodic ambiguity, noise and nonlinear/filter phase
can therefore prevent proposals even when some transfer measurements are usable.
The caller must not change thresholds silently or discard unfavorable positions.

## Review proposal and unchanged outcomes

`C-PA-ALIGNMENT-PROPOSAL`, version 1, contains:

| Field | Meaning |
|---|---|
| `status` | `proposed`, `no_change`, or `refused` |
| `basis_configuration_revision`, `basis_configuration` | Exact graph/version fence |
| `basis_capture` | Complete first anchor capture identity, including source epoch/map/reference/time origin; other examined captures have the same shared basis |
| `measurement_ids` | Examined capture IDs; partial on refusal |
| `changes` | Zero to two independent output delay/polarity changes |
| `added_latency_samples` | Largest added output delay; zero for unchanged/refused |
| `verification_required` | Always true, including a no-change result |
| `reason` | Stable reason code, never a claim of physical success |

Each change contains `output_index`, `before_delay_samples`,
`after_delay_samples`, `before_inverted`, `after_inverted`. Delay conversion uses
round(delay_ms * 48), matching the existing graph's one-sample behavior. The earlier
arrival is delayed, never the later one advanced. Existing output delay plus added
delay cannot exceed 480 samples/10 ms. Polarity correction toggles only the target
output if required. A polarity-only edit retains the exact original delay value.
Already aligned paths return `no_change` and an empty changes array. Any refusal
returns no changes and preserves the supplied base graph.

`candidate_configuration` checks exact base graph equality and revision, fields,
output identities, before values, delay ranges and latency accounting. It clones
the graph and changes only identified output delay/polarity fields. Program input
processing, route, crossover, EQ, gain, limiter and mute fields remain identical.
Applications must additionally compare the retained `basis_capture` epoch, map and
reference identity against their current source owner, invalidating results on any
change. `candidate_configuration` checks graph/revision only and cannot observe a
host source owner. Applications must still use their validated **muted prepare/review/commit** path;
explicit output rearm remains separate. Do not deserialize an arbitrary proposal
and treat its revision or measurement IDs as authorization.

## Validation and limits

`cargo test --locked --test measurement` covers known integer delays/polarity,
actual production Graph rendering, independently predicted corrected sums and
analytic LR24 complex response, no-change/range/rounding, timing/clipping/drop
refusals, noise/periodicity/reflections, exact multi-position consensus, malformed
JSON and candidate configuration preservation. All generated signals are invented
in memory. Normal production gates remain in [VALIDATION](VALIDATION.md).

No fractional-sample fitting, automatic EQ, live capture, clock-drift correction,
calibration file processing, general reflected-room inversion or acoustic confidence
claim is made. A strong coherent reflected path can still look like a direct path.
Single-position proposals are provisional. Actual combined-response verification,
several physical positions, protection qualification and separately authorized
capture remain required before acoustic acceptance.
