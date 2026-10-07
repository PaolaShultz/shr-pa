#ifndef SHR_PA_EQ_V1_H
#define SHR_PA_EQ_V1_H
#include "shr_pa_v2.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Optional independent extension. Load ALL symbols and validate capabilities.
 * All handle/token access is exclusive; keep library loaded until destruction.
 * Prepare/readback/destroy are offRT. Apply/status/retire do not allocate/free.
 * Apply consumes *prepared only on success. Retire transfers finished/faulted
 * storage to initially NULL *retired, for offRT destroy. Full graph replacement
 * transfers extension resources with the retired graph; destroy it offRT.
 * EQ never rearms/unmutes. Prepare pins lifetime, both generations, epoch/frame.
 * Same-settings success increments EQ generation but preserves all histories.
 * JSON: {version:1,inputs:[exactly two distinct EQ-only settings objects]}.
 * Fade uses ceil(rate/200) frames, weights 0..1 inclusive, before one compressor.
 * No retarget while transition/retirement storage is occupied.
 */
typedef struct ShrPaEqPreparedV1 ShrPaEqPreparedV1;
typedef struct {
 uint32_t version,size,eligible,retirement_occupied;
 uint64_t graph_generation,eq_generation,epoch,next_frame,remaining,instance;
} ShrPaEqStatusV1;
typedef struct {
 uint32_t version,size,status_size,max_json_bytes,sections_per_input,selected_inputs,extra_work_units,reserved;
} ShrPaEqCapabilitiesV1;
int32_t shr_pa_eq_v1_capabilities(ShrPaEqCapabilitiesV1*,uint32_t,uint32_t);
int32_t shr_pa_eq_v1_status(const ShrPaV2*,ShrPaEqStatusV1*,uint32_t,uint32_t);
ShrPaEqPreparedV1 *shr_pa_eq_v1_prepare(const ShrPaV2*,const uint8_t*,uint32_t,uint32_t,uint64_t,uint64_t,uint64_t,uint64_t);
int32_t shr_pa_eq_v1_apply(ShrPaV2*,ShrPaEqPreparedV1**,uint64_t,uint64_t);
int32_t shr_pa_eq_v1_retire(ShrPaV2*,ShrPaEqPreparedV1**);
void shr_pa_eq_v1_destroy(ShrPaEqPreparedV1*);
/* OffRT; positive byte count excluding NUL on success; output untouched on error. */
int32_t shr_pa_eq_v1_readback(const ShrPaV2*,uint32_t,uint32_t,uint8_t*,uint32_t);
#ifdef __cplusplus
}
#endif
#endif
