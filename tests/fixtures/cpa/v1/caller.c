#include "shr_pa.h"
#include <assert.h>
#include <math.h>
#include <stddef.h>
#include <string.h>
_Static_assert(sizeof(ShrPaDescriptorV1) == 80, "descriptor layout");
_Static_assert(sizeof(ShrPaStatusV1) == 24, "status layout");
_Static_assert(offsetof(ShrPaDescriptorV1, limiter_threshold_millidbfs) == 40, "threshold offset");
_Static_assert(offsetof(ShrPaDescriptorV1, unavailable_capabilities) == 76, "last offset");
int main(void) {
    ShrPaDescriptorV1 d, saved;
    memset(&d, 0x5a, sizeof d); saved = d;
    assert(shr_pa_v1_descriptor(&d, 2, sizeof d) == -1);
    assert(memcmp(&d, &saved, sizeof d) == 0);
    assert(shr_pa_v1_descriptor(&d, 1, sizeof d - 1) == -1);
    assert(memcmp(&d, &saved, sizeof d) == 0);
    assert(shr_pa_v1_descriptor(NULL, 1, sizeof d) == -1);
    assert(shr_pa_v1_descriptor((ShrPaDescriptorV1 *)((char *)&d + 1), 1, sizeof d) == -1);
    assert(memcmp(&d, &saved, sizeof d) == 0);
    assert(shr_pa_v1_descriptor(&d, 1, sizeof d) == 0);
    const ShrPaDescriptorV1 expected = {1,80,2,6,3,60,0,1,1,1,-1000,1,100,5,0,8000,192000,1,8192,15};
    assert(memcmp(&d, &expected, sizeof d) == 0);
    ShrPaV1 *h = shr_pa_v1_create(48000, 256);
    assert(h != NULL);
    ShrPaStatusV1 s, old;
    memset(&s, 0x5a, sizeof s); old = s;
    assert(shr_pa_v1_status(NULL, &s, 1, sizeof s) == -1);
    assert(shr_pa_v1_status(h, &s, 1, sizeof s + 1) == -1);
    assert(shr_pa_v1_status(h, &s, 2, sizeof s) == -1);
    assert(memcmp(&s, &old, sizeof s) == 0);
    assert(shr_pa_v1_status(h, (ShrPaStatusV1 *)h, 1, sizeof s) == -1);
    assert(shr_pa_v1_status(h, &s, 1, sizeof s) == 0);
    assert(s.version == 1 && s.size == 24 && s.sample_rate == 48000 && s.max_block == 256);
    assert(s.fault_latched == 0 && s.recreate_required == 0);
    double input[512], output[1536];
    for (size_t i=0;i<256;i++) { input[i*2]=0.25; input[i*2+1]=-0.125; }
    for (size_t i=0;i<1536;i++) output[i]=42;
    assert(shr_pa_v1_process(h, input, output, 257) == -1);
    for (size_t i=0;i<1536;i++) assert(output[i]==42);
    assert(shr_pa_v1_process(h, input, input, 256) == -1);
    assert(shr_pa_v1_process(h, input, output, 256) == 0);
    assert(fabs(output[0] - 0.25/240) < 1e-16);
    assert(output[255*6] == 0.25 && output[255*6+1] == -0.125);
    for (size_t i=0;i<256;i++) for(size_t c=2;c<6;c++) assert(output[i*6+c]==0);
    for (size_t i=0;i<256;i++) { input[i*2]=4; input[i*2+1]=1; }
    assert(shr_pa_v1_process(h, input, output, 256) == 0);
    assert(fabs(output[0]-pow(10,-1.0/20))<1e-15 && output[1]==output[0]/4);
    input[300]=NAN;
    assert(shr_pa_v1_process(h, input, output, 256) == -2);
    for (size_t i=0;i<1536;i++) assert(output[i]==0);
    assert(shr_pa_v1_status(h, &s, 1, sizeof s) == 0);
    assert(s.fault_latched==1 && s.recreate_required==1);
    input[300]=0;
    assert(shr_pa_v1_process(h, input, output, 256) == -2);
    shr_pa_v1_destroy(h);
    h=shr_pa_v1_create(8000,1);
    assert(shr_pa_v1_status(h, &s, 1, sizeof s)==0 && s.sample_rate==8000 && s.max_block==1 && s.fault_latched==0);
    shr_pa_v1_destroy(h);
    h=shr_pa_v1_create(192000,8192); assert(h);
    assert(shr_pa_v1_status(h, &s, 1, sizeof s)==0 && s.sample_rate==192000 && s.max_block==8192);
    shr_pa_v1_destroy(h);
    assert(shr_pa_v1_create(7999,1)==NULL && shr_pa_v1_create(192001,1)==NULL);
    assert(shr_pa_v1_create(48000,0)==NULL && shr_pa_v1_create(48000,8193)==NULL);
    return 0;
}
