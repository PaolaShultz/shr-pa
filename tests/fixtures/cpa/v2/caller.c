#include "shr_pa_v2.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
_Static_assert(sizeof(ShrPaStatusV2)==80,"status ABI");
_Static_assert(sizeof(ShrPaCapabilitiesV2)==64,"capability ABI");
int main(int argc,char **argv) {
 assert(argc==2);
 FILE *f=fopen(argv[1],"rb");assert(f);
 assert(fseek(f,0,SEEK_END)==0);long length=ftell(f);assert(length>0&&length<=1048576);rewind(f);
 uint8_t *json=malloc((size_t)length);assert(json);assert(fread(json,1,(size_t)length,f)==(size_t)length);fclose(f);
 uint8_t error[256];assert(shr_pa_v2_validate(json,(uint32_t)length,error,sizeof error)==0);
 ShrPaV2 *prepared=shr_pa_v2_prepare(json,(uint32_t)length,2);assert(prepared);
 ShrPaV2 *next=shr_pa_v2_prepare(json,(uint32_t)length,2);assert(next);free(json);
 ShrPaCapabilitiesV2 cap;assert(shr_pa_v2_capabilities(&cap,2,sizeof cap)==0);assert(cap.measurement_available==0&&cap.true_peak_available==0);
 ShrPaStatusV2 s;assert(shr_pa_v2_status(prepared,&s,2,sizeof s)==0);assert(s.committed==0&&s.muted&&s.quiesced);
 uint32_t ni=s.input_channels,no=s.output_channels,n=s.max_block;
 double *input=calloc((size_t)ni*n,sizeof(double)),*output=calloc((size_t)no*n,sizeof(double));assert(input&&output);
 for(uint32_t i=0;i<n;++i)for(uint32_t ch=0;ch<ni;++ch)input[i*ni+ch]=0.01*(ch+1)*sin((double)i*0.03);
 ShrPaV2 *active=NULL,*retired=NULL;
 assert(shr_pa_v2_apply(&active,prepared,&retired,7,0)==0&&retired==NULL);
 assert(shr_pa_v2_process(active,input,output,ni,no,n,7,0)==0);
 for(uint32_t i=0;i<no*n;++i)assert(output[i]==0.0);
 assert(shr_pa_v2_rearm(active,7,n)==0);
 assert(shr_pa_v2_process(active,input,output,ni,no,n,7,n)==0);
 double energy=0;for(uint32_t i=0;i<no*n;++i){assert(isfinite(output[i]));assert(fabs(output[i])<=1.0);energy+=output[i]*output[i];}assert(energy>0);
 assert(shr_pa_v2_apply(&active,next,&retired,7,2*n)==-3);
 assert(shr_pa_v2_mute(active)==0);assert(shr_pa_v2_process(active,input,output,ni,no,n,7,2*n)==0);
 assert(shr_pa_v2_apply(&active,next,&retired,7,3*n)==0&&retired==prepared);
 assert(shr_pa_v2_status(active,&s,2,sizeof s)==0&&s.muted&&s.generation==2&&s.applied_frame==3*n);
 shr_pa_v2_destroy(retired);shr_pa_v2_destroy(active);free(input);free(output);
 puts("C-PA v2 lifetime, real rendering and retirement passed");return 0;
}
