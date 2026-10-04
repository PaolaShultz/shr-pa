#ifndef SHR_PA_V1_H
#define SHR_PA_V1_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ShrPaV1 ShrPaV1;
enum { SHR_PA_OK = 0, SHR_PA_INVALID_ARGUMENT = -1, SHR_PA_FAULT = -2 };

/* Controller only. Rate 8000..192000 Hz; block 1..8192 frames; invalid -> NULL.
 * Unity gain, full-range L/R on logical outputs 0/1, silence on 2..5.
 * Existing -1 dBFS linked sample limiter, 100 ms release, 5 ms startup ramp.
 * This sample ceiling is not calibrated amplifier/loudspeaker protection. */
ShrPaV1 *shr_pa_v1_create(uint32_t sample_rate, uint32_t max_block);

/* Exclusive audio-worker call. Input: frames*2 doubles; output: frames*6 doubles.
 * Interleaved, native f64 throughout; frames must be 1..max_block.
 * Handle and buffers must be live, correctly sized, aligned and disjoint.
 * No allocation/free, locks, I/O or coefficient preparation in this call.
 * Invalid argument -> -1, output untouched. Numeric fault -> -2, whole block
 * and all following valid blocks silent; destroy/create explicitly restarts.
 * Pointer checks cannot detect invalid allocations, races or stale handles. */
int32_t shr_pa_v1_process(ShrPaV1 *handle, const double *input,
                         double *output, uint32_t frames);

/* Fixed preset: zero algorithmic frames; host/device buffers are separate. */
uint32_t shr_pa_v1_delay_frames(void);

/* Controller only after worker has stopped. NULL is safe; destroy once. */
void shr_pa_v1_destroy(ShrPaV1 *handle);

/* Read-only C-PA:1 descriptor/status extension. Fixed widths, no pointers.
 * Unsupported versions or sizes are rejected before any output write. */
enum {
    SHR_PA_QUERY_VERSION = 1, SHR_PA_DESCRIPTOR_V1_SIZE = 80,
    SHR_PA_STATUS_V1_SIZE = 24, SHR_PA_SAMPLE_F64 = 1,
    SHR_PA_PRESET_FULL_RANGE = 1, SHR_PA_LIMITER_SAMPLE = 1,
    SHR_PA_UNAVAILABLE_CONTROLS = 1, SHR_PA_UNAVAILABLE_MEASUREMENT = 2,
    SHR_PA_UNAVAILABLE_ACOUSTIC_PROTECTION = 4,
    SHR_PA_UNAVAILABLE_TRUE_PEAK = 8
};
typedef struct ShrPaDescriptorV1 {
    uint32_t version;
    uint32_t size;
    uint32_t input_channels;
    uint32_t logical_outputs;
    uint32_t active_output_mask;
    uint32_t silent_output_mask;
    uint32_t physical_io_owned;
    uint32_t sample_format;
    uint32_t fixed_preset;
    uint32_t limiter_kind;
    int32_t limiter_threshold_millidbfs;
    uint32_t limiter_linked;
    uint32_t limiter_release_ms;
    uint32_t startup_ramp_ms;
    uint32_t fixed_delay_frames;
    uint32_t min_rate;
    uint32_t max_rate;
    uint32_t min_block;
    uint32_t max_block;
    uint32_t unavailable_capabilities;
} ShrPaDescriptorV1;
typedef struct ShrPaStatusV1 {
    uint32_t version, size, sample_rate, max_block;
    uint32_t fault_latched, recreate_required;
} ShrPaStatusV1;

/* Caller owns live, aligned, unaliased writable output of EXACT size.
 * No output initialization needed. NULL/alignment/span/version/size errors -> -1
 * with output unchanged. Cannot prove allocation validity or detect races.
 * Descriptor: masks bits0..5 identify logical outputs, not physical channels;
 * physical_io_owned=0 means host owns I/O. Unavailable bits above are set.
 * Threshold is signed milli-dBFS; ramp/release units are milliseconds. */
int32_t shr_pa_v1_descriptor(ShrPaDescriptorV1 *output,
                             uint32_t version, uint32_t size);

/* Single-owner/quiesced against process, queries and destroy. Output must also
 * be disjoint from ALL handle-owned storage; overlap -> -1, unchanged.
 * Returns 0 for a valid query even when fault_latched/recreate_required are 1.
 * No fault reset; stop/destroy/create is required. No allocation, locks or I/O. */
int32_t shr_pa_v1_status(const ShrPaV1 *handle, ShrPaStatusV1 *output,
                        uint32_t version, uint32_t size);

#ifdef __cplusplus
}
#endif
#endif
