//! Additive configurable ABI. See `src/shr_pa_v2.h` for exclusive ownership,
//! valid allocation, lifetime, alignment and non-aliasing requirements.
#![allow(clippy::missing_safety_doc)]
use crate::graph::{Graph, GraphConfig, MAX_DELAY_SAMPLES, MAX_OPERATIONS, MAX_PORTS};
use std::{mem, ptr, slice};
const MAX_JSON: usize = 1024 * 1024;
const OK: i32 = 0;
const INVALID: i32 = -1;
const FAULT: i32 = -2;
const BUSY: i32 = -3;
const STALE: i32 = -4;
pub struct ShrPaV2 {
    pub(crate) graph: Graph,
    pub(crate) phase: u32,
    pub(crate) epoch: u64,
    pub(crate) next_frame: u64,
    pub(crate) generation: u64,
    applied_frame: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShrPaStatusV2 {
    pub version: u32,
    pub size: u32,
    pub sample_rate: u32,
    pub max_block: u32,
    pub input_channels: u32,
    pub output_channels: u32,
    pub muted: u32,
    pub quiesced: u32,
    pub fault_latched: u32,
    pub committed: u32,
    pub reserved0: u32,
    pub reserved1: u32,
    pub epoch: u64,
    pub next_frame: u64,
    pub generation: u64,
    pub applied_frame: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShrPaCapabilitiesV2 {
    pub version: u32,
    pub size: u32,
    pub min_rate: u32,
    pub max_rate: u32,
    pub min_block: u32,
    pub max_block: u32,
    pub max_ports: u32,
    pub max_nodes: u32,
    pub max_operations: u32,
    pub max_delay_samples: u32,
    pub max_json_bytes: u32,
    pub sample_format: u32,
    pub measurement_available: u32,
    pub true_peak_available: u32,
    pub acoustic_protection_available: u32,
    pub physical_io_owned: u32,
}
const _: () = assert!(mem::size_of::<ShrPaStatusV2>() == 80);
const _: () = assert!(mem::size_of::<ShrPaCapabilitiesV2>() == 64);
pub(crate) fn span<T>(p: *const T, count: usize) -> Option<(usize, usize)> {
    let a = p as usize;
    if p.is_null() || !a.is_multiple_of(mem::align_of::<T>()) || count == 0 {
        return None;
    }
    let bytes = count.checked_mul(mem::size_of::<T>())?;
    if bytes > isize::MAX as usize {
        return None;
    }
    Some((a, a.checked_add(bytes)?))
}
pub(crate) fn overlaps(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.1 && b.0 < a.1
}
impl ShrPaV2 {
    pub(crate) fn overlaps(&self, range: (usize, usize)) -> bool {
        overlaps(span(self, 1).unwrap(), range) || self.graph.overlaps(range)
    }
}
unsafe fn parse(json: *const u8, bytes: u32) -> Result<GraphConfig, String> {
    if bytes as usize > MAX_JSON || span(json, bytes as usize).is_none() {
        return Err("JSON pointer/length: require 1..1048576 bytes".into());
    }
    let config: GraphConfig =
        serde_json::from_slice(unsafe { slice::from_raw_parts(json, bytes as usize) })
            .map_err(|e| format!("graph JSON: {e}"))?;
    config.validate().map_err(str::to_owned)?;
    Ok(config)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_validate(
    json: *const u8,
    bytes: u32,
    error: *mut u8,
    error_capacity: u32,
) -> i32 {
    let Some(e) = span(error, error_capacity as usize) else {
        return INVALID;
    };
    let Some(j) = span(json, bytes as usize) else {
        return INVALID;
    };
    if overlaps(e, j) {
        return INVALID;
    }
    match unsafe { parse(json, bytes) } {
        Ok(_) => {
            unsafe { error.write(0) };
            OK
        }
        Err(message) => {
            let n = message.len().min(error_capacity as usize - 1);
            unsafe {
                ptr::copy_nonoverlapping(message.as_ptr(), error, n);
                error.add(n).write(0)
            };
            INVALID
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_prepare(
    json: *const u8,
    bytes: u32,
    version: u32,
) -> *mut ShrPaV2 {
    if version != 2 {
        return ptr::null_mut();
    }
    let Ok(config) = (unsafe { parse(json, bytes) }) else {
        return ptr::null_mut();
    };
    let Ok(graph) = Graph::prepare(config) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(ShrPaV2 {
        graph,
        phase: 0,
        epoch: 0,
        next_frame: 0,
        generation: 0,
        applied_frame: 0,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_apply(
    active: *mut *mut ShrPaV2,
    prepared: *mut ShrPaV2,
    retired: *mut *mut ShrPaV2,
    epoch: u64,
    frame: u64,
) -> i32 {
    let (Some(a), Some(p), Some(r)) = (span(active, 1), span(prepared, 1), span(retired, 1)) else {
        return INVALID;
    };
    if overlaps(a, r) || overlaps(a, p) || overlaps(r, p) || epoch == 0 {
        return INVALID;
    }
    let old = unsafe { active.read() };
    if !unsafe { retired.read() }.is_null() {
        return BUSY;
    }
    if old == prepared {
        return INVALID;
    }
    let prepared = unsafe { &mut *prepared };
    if prepared.overlaps(a) || prepared.overlaps(r) || prepared.phase != 0 {
        return INVALID;
    }
    let generation = if old.is_null() {
        1
    } else {
        let Some(o) = span(old, 1) else {
            return INVALID;
        };
        if overlaps(o, p) || overlaps(o, a) || overlaps(o, r) || prepared.overlaps(o) {
            return INVALID;
        }
        let previous = unsafe { &mut *old };
        if previous.overlaps(a)
            || previous.overlaps(r)
            || previous.overlaps(p)
            || previous.phase != 1
        {
            return INVALID;
        }
        if !previous.graph.quiesced() {
            return BUSY;
        }
        if epoch < previous.epoch || (epoch == previous.epoch && frame != previous.next_frame) {
            return STALE;
        }
        let Some(g) = previous.generation.checked_add(1) else {
            return STALE;
        };
        g
    };
    prepared.phase = 1;
    prepared.epoch = epoch;
    prepared.next_frame = frame;
    prepared.applied_frame = frame;
    prepared.generation = generation;
    if !old.is_null() {
        unsafe { (*old).phase = 2 };
    }
    unsafe {
        retired.write(old);
        active.write(prepared)
    };
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_mute(handle: *mut ShrPaV2) -> i32 {
    if span(handle, 1).is_none() {
        return INVALID;
    }
    let h = unsafe { &mut *handle };
    if h.phase != 1 {
        return INVALID;
    }
    h.graph.mute();
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_rearm(handle: *mut ShrPaV2, epoch: u64, frame: u64) -> i32 {
    if span(handle, 1).is_none() {
        return INVALID;
    }
    let h = unsafe { &mut *handle };
    if h.phase != 1 {
        return INVALID;
    }
    if h.epoch != epoch || h.next_frame != frame {
        return STALE;
    }
    if h.graph.fault {
        return FAULT;
    }
    if h.graph.rearm().is_err() {
        return BUSY;
    }
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_process(
    handle: *mut ShrPaV2,
    input: *const f64,
    output: *mut f64,
    input_channels: u32,
    output_channels: u32,
    frames: u32,
    epoch: u64,
    first_frame: u64,
) -> i32 {
    let (Some(hs), Some(i), Some(o)) = (
        span(handle, 1),
        span(
            input,
            (input_channels as usize).saturating_mul(frames as usize),
        ),
        span(
            output,
            (output_channels as usize).saturating_mul(frames as usize),
        ),
    ) else {
        return INVALID;
    };
    if overlaps(i, o) || overlaps(hs, i) || overlaps(hs, o) {
        return INVALID;
    }
    let h = unsafe { &mut *handle };
    if h.phase != 1
        || input_channels as usize != h.graph.config.inputs.len()
        || output_channels as usize != h.graph.config.outputs.len()
        || frames as usize > h.graph.config.max_block
        || h.overlaps(i)
        || h.overlaps(o)
    {
        return INVALID;
    }
    let out =
        unsafe { slice::from_raw_parts_mut(output, frames as usize * output_channels as usize) };
    if epoch != h.epoch
        || first_frame != h.next_frame
        || first_frame.checked_add(frames as u64).is_none()
    {
        h.graph.fault = true;
        h.graph.mute();
        h.graph.ramp = 0.;
        out.fill(0.);
        return STALE;
    }
    let result = h.graph.process(
        unsafe { slice::from_raw_parts(input, frames as usize * input_channels as usize) },
        out,
    );
    if result.is_err() {
        return FAULT;
    }
    h.next_frame += frames as u64;
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_status(
    handle: *const ShrPaV2,
    output: *mut ShrPaStatusV2,
    version: u32,
    size: u32,
) -> i32 {
    let (Some(_), Some(o)) = (span(handle, 1), span(output, 1)) else {
        return INVALID;
    };
    if version != 2 || size != 80 {
        return INVALID;
    }
    let h = unsafe { &*handle };
    if h.overlaps(o) {
        return INVALID;
    }
    unsafe {
        output.write(ShrPaStatusV2 {
            version: 2,
            size: 80,
            sample_rate: h.graph.config.sample_rate,
            max_block: h.graph.config.max_block as u32,
            input_channels: h.graph.config.inputs.len() as u32,
            output_channels: h.graph.config.outputs.len() as u32,
            muted: h.graph.muted as u32,
            quiesced: h.graph.quiesced() as u32,
            fault_latched: h.graph.fault as u32,
            committed: (h.phase == 1) as u32,
            reserved0: 0,
            reserved1: 0,
            epoch: h.epoch,
            next_frame: h.next_frame,
            generation: h.generation,
            applied_frame: h.applied_frame,
        })
    };
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_capabilities(
    output: *mut ShrPaCapabilitiesV2,
    version: u32,
    size: u32,
) -> i32 {
    if span(output, 1).is_none() || version != 2 || size != 64 {
        return INVALID;
    }
    unsafe {
        output.write(ShrPaCapabilitiesV2 {
            version: 2,
            size: 64,
            min_rate: 8000,
            max_rate: 192000,
            min_block: 1,
            max_block: 8192,
            max_ports: MAX_PORTS as u32,
            max_nodes: MAX_PORTS as u32,
            max_operations: MAX_OPERATIONS as u32,
            max_delay_samples: MAX_DELAY_SAMPLES as u32,
            max_json_bytes: MAX_JSON as u32,
            sample_format: 1,
            ..ShrPaCapabilitiesV2::default()
        })
    };
    OK
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v2_destroy(handle: *mut ShrPaV2) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}
