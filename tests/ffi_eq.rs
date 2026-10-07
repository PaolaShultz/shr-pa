use shr_pa::{
    ffi_eq::*,
    ffi_v2::*,
    graph::GraphConfig,
    live_eq::{EqPatch, EqSettings},
};
use std::ptr;
unsafe fn active(c: &GraphConfig, frame: u64) -> *mut ShrPaV2 {
    let json = serde_json::to_vec(c).unwrap();
    let p = unsafe { shr_pa_v2_prepare(json.as_ptr(), json.len() as u32, 2) };
    let mut h = ptr::null_mut();
    let mut retired = ptr::null_mut();
    assert_eq!(
        unsafe { shr_pa_v2_apply(&mut h, p, &mut retired, 1, frame) },
        0
    );
    h
}
unsafe fn status(h: *mut ShrPaV2) -> ShrPaEqStatusV1 {
    let mut s = ShrPaEqStatusV1::default();
    assert_eq!(unsafe { shr_pa_eq_v1_status(h, &mut s, 1, 64) }, 0);
    s
}
#[test]
fn abi_lifetime_frame_refusal_and_muted_full_replacement() {
    unsafe {
        let c = GraphConfig::stereo(vec![]);
        let mut a = active(&c, 0);
        let b = active(&c, 0);
        let mut patch = EqPatch {
            version: 1,
            inputs: [
                EqSettings::from_input(1, &c.inputs[1]),
                EqSettings::from_input(0, &c.inputs[0]),
            ],
        };
        patch.inputs[0].eq[0].db = 6.;
        let json = serde_json::to_vec(&patch).unwrap();
        let mut p = shr_pa_eq_v1_prepare(a, json.as_ptr(), json.len() as u32, 1, 1, 0, 1, 0);
        assert!(!p.is_null());
        let saved = p;
        assert_eq!(shr_pa_eq_v1_apply(b, &mut p, 1, 0), -4);
        assert_eq!(p, saved);
        assert_eq!(shr_pa_eq_v1_apply(a, &mut p, 1, 1), -4);
        assert_eq!(p, saved);
        assert_eq!(shr_pa_eq_v1_apply(a, &mut p, 1, 0), 0);
        assert!(p.is_null());
        assert_eq!(status(a).remaining, 240);
        assert_eq!(status(a).eq_generation, 1);
        // Already-muted full graph replacement retains extension in retired graph.
        let full = serde_json::to_vec(&c).unwrap();
        let fresh = shr_pa_v2_prepare(full.as_ptr(), full.len() as u32, 2);
        let mut retired = ptr::null_mut();
        assert_eq!(shr_pa_v2_apply(&mut a, fresh, &mut retired, 1, 0), 0);
        assert_eq!(status(a).eq_generation, 0);
        assert_eq!(status(retired).remaining, 240);
        shr_pa_v2_destroy(retired);
        shr_pa_v2_destroy(a);
        shr_pa_v2_destroy(b);
        let overflow = active(&c, u64::MAX - 1);
        assert!(
            shr_pa_eq_v1_prepare(
                overflow,
                json.as_ptr(),
                json.len() as u32,
                1,
                1,
                0,
                1,
                u64::MAX - 1
            )
            .is_null()
        );
        assert_eq!(status(overflow).eq_generation, 0);
        shr_pa_v2_destroy(overflow);
    }
}
#[test]
fn midfade_fault_retirement_readback_and_disarmed_target_recovery() {
    unsafe {
        let c = GraphConfig::stereo(vec![]);
        let h = active(&c, 0);
        let mut patch = EqPatch {
            version: 1,
            inputs: [
                EqSettings::from_input(0, &c.inputs[0]),
                EqSettings::from_input(1, &c.inputs[1]),
            ],
        };
        patch.inputs[0].eq[0].db = 6.;
        let json = serde_json::to_vec(&patch).unwrap();
        let mut p = shr_pa_eq_v1_prepare(h, json.as_ptr(), json.len() as u32, 1, 1, 0, 1, 0);
        assert_eq!(shr_pa_eq_v1_apply(h, &mut p, 1, 0), 0);
        let input = [f64::NAN, 0.];
        let mut output = [1.; 2];
        assert_eq!(
            shr_pa_v2_process(h, input.as_ptr(), output.as_mut_ptr(), 2, 2, 1, 1, 0),
            -2
        );
        assert_eq!(output, [0., 0.]);
        let mut retired = ptr::null_mut();
        assert_eq!(shr_pa_eq_v1_retire(h, &mut retired), 0);
        let mut bytes = vec![0; 65536];
        let n = shr_pa_eq_v1_readback(h, 0, 1, bytes.as_mut_ptr(), bytes.len() as u32);
        assert!(n > 0);
        let value: serde_json::Value = serde_json::from_slice(&bytes[..n as usize]).unwrap();
        assert_eq!(value["fault_latched"], true);
        assert_eq!(value["settled"], false);
        assert_eq!(value["current"][0]["eq"][0]["db"], 0.);
        assert_eq!(value["target"][0]["eq"][0]["db"], 6.);
        shr_pa_eq_v1_destroy(retired);
        shr_pa_v2_destroy(h);
    }
}

#[test]
fn optional_abi_null_alignment_size_and_managed_span_refusals_preserve_ownership() {
    unsafe {
        let c = GraphConfig::stereo(vec![]);
        let h = active(&c, 0);
        let mut s = ShrPaEqStatusV1 {
            instance: u64::MAX,
            ..Default::default()
        };
        assert_eq!(shr_pa_eq_v1_status(ptr::null(), &mut s, 1, 64), -1);
        assert_eq!(shr_pa_eq_v1_status(h, &mut s, 2, 64), -1);
        assert_eq!(shr_pa_eq_v1_status(h, &mut s, 1, 63), -1);
        assert_eq!(s.instance, u64::MAX);
        let mut storage = [0u64; 16];
        let misaligned = storage.as_mut_ptr().cast::<u8>().add(1);
        assert_eq!(shr_pa_eq_v1_status(h, misaligned.cast(), 1, 64), -1);
        assert_eq!(
            shr_pa_eq_v1_status(h.cast::<u8>().add(1).cast(), &mut s, 1, 64),
            -1
        );
        assert_eq!(shr_pa_eq_v1_apply(h, misaligned.cast(), 1, 0), -1);
        assert_eq!(shr_pa_eq_v1_retire(h, misaligned.cast()), -1);
        let patch = EqPatch {
            version: 1,
            inputs: [
                EqSettings::from_input(0, &c.inputs[0]),
                EqSettings::from_input(1, &c.inputs[1]),
            ],
        };
        let raw = serde_json::to_vec(&patch).unwrap();
        let mut p = shr_pa_eq_v1_prepare(h, raw.as_ptr(), raw.len() as u32, 1, 1, 0, 1, 0);
        assert!(!p.is_null());
        let managed = p;
        assert_eq!(shr_pa_eq_v1_apply(h, &mut p, 1, 0), 0);
        assert_eq!(shr_pa_eq_v1_status(h, managed.cast(), 1, 64), -1);
        assert_eq!(shr_pa_eq_v1_readback(h, 0, 1, managed.cast(), 128), -1);
        assert_eq!(shr_pa_eq_v1_retire(h, managed.cast()), -1);
        assert_eq!(status(h).eq_generation, 1);
        let mut retired = ptr::null_mut();
        assert_eq!(shr_pa_eq_v1_retire(h, &mut retired), 0);
        assert_eq!(retired, managed);
        shr_pa_eq_v1_destroy(retired);
        shr_pa_v2_destroy(h);
    }
}
