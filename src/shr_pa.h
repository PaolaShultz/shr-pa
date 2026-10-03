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

#ifdef __cplusplus
}
#endif
#endif
