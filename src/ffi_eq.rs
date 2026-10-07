//! Independent optional EQ v1 ABI. Exclusive owner access is required.
#![allow(clippy::missing_safety_doc)]
use crate::{
    ffi_v2::{ShrPaV2, overlaps, span},
    live_eq::{EqPatch, PreparedEq},
};
use std::{mem, ptr, slice};
const INVALID: i32 = -1;
const FAULT: i32 = -2;
const BUSY: i32 = -3;
const STALE: i32 = -4;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct ShrPaEqStatusV1 {
    pub version: u32,
    pub size: u32,
    pub eligible: u32,
    pub retirement_occupied: u32,
    pub graph_generation: u64,
    pub eq_generation: u64,
    pub epoch: u64,
    pub next_frame: u64,
    pub remaining: u64,
    pub instance: u64,
}
const _: () = assert!(mem::size_of::<ShrPaEqStatusV1>() == 64);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_status(
    h: *const ShrPaV2,
    out: *mut ShrPaEqStatusV1,
    version: u32,
    size: u32,
) -> i32 {
    let (Some(_), Some(o)) = (span(h, 1), span(out, 1)) else {
        return INVALID;
    };
    if version != 1 || size != 64 {
        return INVALID;
    }
    let h = unsafe { &*h };
    if h.overlaps(o) {
        return INVALID;
    }
    let s = h.graph.eq_progress();
    unsafe {
        out.write(ShrPaEqStatusV1 {
            version: 1,
            size: 64,
            eligible: s.eligible as u32,
            retirement_occupied: s.retirement_occupied as u32,
            graph_generation: h.generation,
            eq_generation: s.generation,
            epoch: h.epoch,
            next_frame: h.next_frame,
            remaining: s.remaining,
            instance: h.graph.instance,
        });
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_prepare(
    h: *const ShrPaV2,
    json: *const u8,
    bytes: u32,
    version: u32,
    graph_generation: u64,
    eq_generation: u64,
    epoch: u64,
    frame: u64,
) -> *mut PreparedEq {
    let (Some(_), Some(j)) = (span(h, 1), span(json, bytes as usize)) else {
        return ptr::null_mut();
    };
    if version != 1 || bytes > 32768 {
        return ptr::null_mut();
    }
    let h = unsafe { &*h };
    if h.overlaps(j)
        || h.phase != 1
        || h.generation != graph_generation
        || h.graph.eq_generation != eq_generation
        || h.epoch != epoch
        || frame < h.next_frame
    {
        return ptr::null_mut();
    }
    let Ok(patch) = EqPatch::parse(
        unsafe { slice::from_raw_parts(json, bytes as usize) },
        h.graph.config().sample_rate,
        h.graph.config().inputs.len(),
    ) else {
        return ptr::null_mut();
    };
    let Ok(mut p) = h.graph.prepare_eq(patch) else {
        return ptr::null_mut();
    };
    if !p.noop && frame.checked_add(p.duration).is_none() {
        return ptr::null_mut();
    }
    p.graph_generation = graph_generation;
    p.epoch = epoch;
    p.frame = frame;
    Box::into_raw(p)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_apply(
    h: *mut ShrPaV2,
    prepared: *mut *mut PreparedEq,
    epoch: u64,
    frame: u64,
) -> i32 {
    let (Some(hs), Some(ps)) = (span(h, 1), span(prepared, 1)) else {
        return INVALID;
    };
    if overlaps(hs, ps) {
        return INVALID;
    }
    // Inspect all spans before creating a mutable handle reference.
    let hh = unsafe { &*h };
    if hh.overlaps(ps) {
        return INVALID;
    }
    let raw = unsafe { prepared.read() };
    let Some(p) = span(raw, 1) else {
        return INVALID;
    };
    if hh.overlaps(p) || overlaps(p, ps) {
        return INVALID;
    }
    let pp = unsafe { &*raw };
    if pp.overlaps(hs) || pp.overlaps(ps) {
        return INVALID;
    }
    if hh.phase != 1 {
        return INVALID;
    }
    if hh.graph.fault {
        return FAULT;
    }
    if hh.graph.live_eq.is_some() {
        return BUSY;
    }
    if pp.instance != hh.graph.instance
        || pp.graph_generation != hh.generation
        || pp.base_generation != hh.graph.eq_generation
        || pp.epoch != epoch
        || pp.frame != frame
        || hh.epoch != epoch
        || hh.next_frame != frame
    {
        return STALE;
    }
    if !pp.noop && frame.checked_add(pp.duration).is_none() {
        return STALE;
    }
    let h = unsafe { &mut *h };
    let mut owned = Some(unsafe { Box::from_raw(raw) });
    match h.graph.apply_eq(&mut owned) {
        Ok(()) => {
            unsafe {
                prepared.write(ptr::null_mut());
            }
            0
        }
        Err(_) => {
            let _ = Box::into_raw(owned.unwrap());
            STALE
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_retire(
    h: *mut ShrPaV2,
    retired: *mut *mut PreparedEq,
) -> i32 {
    let (Some(hs), Some(r)) = (span(h, 1), span(retired, 1)) else {
        return INVALID;
    };
    if overlaps(hs, r) {
        return INVALID;
    }
    let h = unsafe { &mut *h };
    if h.overlaps(r) || h.phase != 1 {
        return INVALID;
    }
    if !unsafe { retired.read() }.is_null() {
        return BUSY;
    }
    let Some(p) = h.graph.retire_eq() else {
        return BUSY;
    };
    unsafe {
        retired.write(Box::into_raw(p));
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_destroy(p: *mut PreparedEq) {
    if !p.is_null() {
        drop(unsafe { Box::from_raw(p) });
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct ShrPaEqCapabilitiesV1 {
    pub version: u32,
    pub size: u32,
    pub status_size: u32,
    pub max_json_bytes: u32,
    pub sections_per_input: u32,
    pub selected_inputs: u32,
    pub extra_work_units: u32,
    pub reserved: u32,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_capabilities(
    out: *mut ShrPaEqCapabilitiesV1,
    version: u32,
    size: u32,
) -> i32 {
    if span(out, 1).is_none() || version != 1 || size != 32 {
        return INVALID;
    }
    unsafe {
        out.write(ShrPaEqCapabilitiesV1 {
            version: 1,
            size: 32,
            status_size: 64,
            max_json_bytes: 32768,
            sections_per_input: 39,
            selected_inputs: 2,
            extra_work_units: 80,
            reserved: 0,
        });
    }
    0
}
/// Controller-only JSON readback; allocation/design is permitted here. Settings
/// and normalized coefficients are opaque strings at the integer-only host wire.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_eq_v1_readback(
    h: *const ShrPaV2,
    first: u32,
    second: u32,
    out: *mut u8,
    capacity: u32,
) -> i32 {
    let (Some(_), Some(o)) = (span(h, 1), span(out, capacity as usize)) else {
        return INVALID;
    };
    let h = unsafe { &*h };
    if h.overlaps(o)
        || h.phase != 1
        || first == second
        || first as usize >= h.graph.config().inputs.len()
        || second as usize >= h.graph.config().inputs.len()
    {
        return INVALID;
    }
    let indices = [first as usize, second as usize];
    let target =
        indices.map(|i| crate::live_eq::EqSettings::from_input(i, &h.graph.config().inputs[i]));
    let current = indices.map(|i| {
        h.graph
            .live_eq
            .as_ref()
            .filter(|p| !p.finished)
            .and_then(|p| p.previous.iter().find(|e| e.input_index == i))
            .cloned()
            .or_else(|| {
                h.graph
                    .eq_aborted
                    .as_ref()
                    .and_then(|e| e.iter().find(|e| e.input_index == i))
                    .cloned()
            })
            .unwrap_or_else(|| {
                crate::live_eq::EqSettings::from_input(i, &h.graph.config().inputs[i])
            })
    });
    let rate = h.graph.config().sample_rate;
    let Ok(current_banks) = current
        .iter()
        .map(|s| s.coefficients(rate))
        .collect::<Result<Vec<_>, _>>()
    else {
        return FAULT;
    };
    let Ok(target_banks) = target
        .iter()
        .map(|s| s.coefficients(rate))
        .collect::<Result<Vec<_>, _>>()
    else {
        return FAULT;
    };
    fn fingerprint(settings: &[crate::live_eq::EqSettings; 2]) -> String {
        let bytes = serde_json::to_vec(settings).unwrap();
        let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        format!("fnv1a64:{hash:016x}")
    }
    let value = serde_json::json!({ "version": 1, "sample_rate": rate, "fault_latched": h.graph.fault,
        "settled": !h.graph.fault && h.graph.eq_progress().remaining == 0,
        "denominator": "1+a1*z^-1+a2*z^-2", "coefficient_order": ["b0","b1","b2","a1","a2"],
        "graph_generation": h.generation, "eq_generation": h.graph.eq_generation,
        "current_fingerprint": fingerprint(&current), "target_fingerprint": fingerprint(&target),
        "current": current, "target": target, "current_coefficients": current_banks, "target_coefficients": target_banks });
    let Ok(bytes) = serde_json::to_vec(&value) else {
        return INVALID;
    };
    if bytes.len() >= capacity as usize {
        return INVALID;
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        out.add(bytes.len()).write(0);
    }
    bytes.len() as i32
}
