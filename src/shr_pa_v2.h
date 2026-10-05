#ifndef SHR_PA_V2_H
#define SHR_PA_V2_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct ShrPaV2 ShrPaV2;
enum { SHR_PA_V2_VERSION=2, SHR_PA_V2_OK=0, SHR_PA_V2_INVALID=-1,
       SHR_PA_V2_FAULT=-2, SHR_PA_V2_BUSY=-3, SHR_PA_V2_STALE=-4,
       SHR_PA_V2_STATUS_SIZE=80, SHR_PA_V2_CAPABILITIES_SIZE=64 };
typedef struct ShrPaStatusV2 {
    uint32_t version, size, sample_rate, max_block;
    uint32_t input_channels, output_channels, muted, quiesced;
    uint32_t fault_latched, committed, reserved0, reserved1;
    uint64_t epoch, next_frame, generation, applied_frame;
} ShrPaStatusV2;
typedef struct ShrPaCapabilitiesV2 {
    uint32_t version, size, min_rate, max_rate;
    uint32_t min_block, max_block, max_ports, max_nodes;
    uint32_t max_operations, max_delay_samples, max_json_bytes, sample_format;
    uint32_t measurement_available, true_peak_available, acoustic_protection_available, physical_io_owned;
} ShrPaCapabilitiesV2;
/* Controller only; strict UTF-8 graph schema v2, <=1 MiB. Returns fresh muted
 * prepared ownership or NULL. validate writes a NUL-terminated reason if capacity
 * permits; prepare copies all JSON data. Library remains loaded until all handles
 * are destroyed. All calls require exclusive single-owner access, no races. */
int32_t shr_pa_v2_validate(const uint8_t *json, uint32_t bytes,
                           uint8_t *error, uint32_t error_capacity);
ShrPaV2 *shr_pa_v2_prepare(const uint8_t *json, uint32_t bytes, uint32_t version);
/* RT boundary: *active may be NULL; *retired MUST be NULL (one reservation).
 * prepared must be fresh, distinct and uncommitted. Existing active must be
 * quiesced. Successful swap transfers prepared ownership; returns old active in
 * retired without freeing. Generation increases; epoch nonzero, frame explicit.
 * A replacement in the same epoch must use exactly active.next_frame; new epochs
 * must increase. Failure leaves all three ownerships and graph unchanged/muted.
 * Destroy retired offRT before reusing that slot; no queued changes. */
int32_t shr_pa_v2_apply(ShrPaV2 **active, ShrPaV2 *prepared,
                       ShrPaV2 **retired, uint64_t epoch, uint64_t frame);
/* Requests a 5ms ramp to persistent mute. Render until status.quiesced. */
int32_t shr_pa_v2_mute(ShrPaV2 *handle);
/* Explicit rearm only when committed, healthy/quiesced and timeline matches. */
int32_t shr_pa_v2_rearm(ShrPaV2 *handle, uint64_t epoch, uint64_t frame);
/* Interleaved f64 arrays with exact dimensions. Frames 1..max_block.
 * Buffers, handle and all owned allocations must be live/aligned/disjoint.
 * No allocations, frees, locks, coefficient design or I/O in render/apply/mute/
 * rearm/status. Bad shape -> unchanged output. Timeline mismatch -> whole block
 * silence and latched fault; fresh prepared commit required. Numeric fault also
 * silences whole block. Source cursor advances only successful matching blocks.
 * The API cannot validate dangling pointers or prove backing allocation length. */
int32_t shr_pa_v2_process(ShrPaV2 *handle, const double *input, double *output,
                         uint32_t input_channels, uint32_t output_channels,
                         uint32_t frames, uint64_t epoch, uint64_t first_frame);
int32_t shr_pa_v2_status(const ShrPaV2 *handle, ShrPaStatusV2 *output,
                        uint32_t version, uint32_t size);
int32_t shr_pa_v2_capabilities(ShrPaCapabilitiesV2 *output,
                              uint32_t version, uint32_t size);
/* Controller only, once after worker stops. NULL safe; never destroy an active
 * handle still referenced by worker or twice through an alias. */
void shr_pa_v2_destroy(ShrPaV2 *handle);
#ifdef __cplusplus
}
#endif
#endif
