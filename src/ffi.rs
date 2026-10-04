//! Version 1 C boundary for the existing full-range PA engine.
//!
//! Creation/destruction belong to the host controller. One audio worker owns each
//! handle exclusively; process has no allocation, locks or I/O. See `shr_pa.h`.

use crate::{
    config::{Config, Layout},
    dsp::Engine,
};
use std::{mem, ptr, slice};

/// Opaque owning handle. Its Rust layout is private to this library version.
pub struct ShrPaV1 {
    engine: Engine,
    max_block: usize,
    sample_rate: u32,
}

/// Successful processing.
pub const SHR_PA_OK: i32 = 0;
/// Invalid pointer shape, overlapping storage, or frame count; output is untouched.
pub const SHR_PA_INVALID_ARGUMENT: i32 = -1;
/// Numerical fault latched; this block and all following valid blocks are silent.
pub const SHR_PA_FAULT: i32 = -2;

/// Prepare unity-gain stereo full-range processing with the existing -1 dBFS
/// linked sample limiter and 5 ms startup ramp. Logical channels 2..5 are silent.
/// Returns null for rates outside 8000..192000 Hz or blocks outside 1..8192 frames.
#[unsafe(no_mangle)]
pub extern "C" fn shr_pa_v1_create(sample_rate: u32, max_block: u32) -> *mut ShrPaV1 {
    let config = Config {
        sample_rate,
        max_block: max_block as usize,
        layout: Layout::FullRange,
        ..Config::default()
    };
    let Ok(mut engine) = Engine::new(config) else {
        return ptr::null_mut();
    };
    engine.set_mutes([false; 6]);
    Box::into_raw(Box::new(ShrPaV1 {
        engine,
        max_block: max_block as usize,
        sample_rate,
    }))
}

fn span<T>(pointer: *const T, count: usize) -> Option<(usize, usize)> {
    let start = pointer as usize;
    let bytes = count.checked_mul(mem::size_of::<T>())?;
    if pointer.is_null()
        || !start.is_multiple_of(mem::align_of::<T>())
        || bytes > isize::MAX as usize
    {
        return None;
    }
    Some((start, start.checked_add(bytes)?))
}

fn overlaps(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// Process 1..max_block interleaved stereo f64 input frames into six f64 outputs.
/// Argument errors leave output untouched. Non-finite samples or numerical faults
/// latch silence and return SHR_PA_FAULT until this handle is destroyed/recreated.
///
/// # Safety
/// A nonnull handle must be a live result of create and exclusively owned during
/// the call. Nonnull sample pointers must refer to initialized readable input and
/// writable output allocations of at least frames*2 and frames*6 doubles. Storage
/// must not overlap the handle or each other. Null, alignment, range-overflow and
/// overlap checks cannot establish allocation validity or detect use after free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v1_process(
    handle: *mut ShrPaV1,
    input: *const f64,
    output: *mut f64,
    frames: u32,
) -> i32 {
    let Some(handle_span) = span(handle, 1) else {
        return SHR_PA_INVALID_ARGUMENT;
    };
    if frames == 0 || frames > 8192 {
        return SHR_PA_INVALID_ARGUMENT;
    }
    let frames = frames as usize;
    let Some(input_span) = span(input, frames * 2) else {
        return SHR_PA_INVALID_ARGUMENT;
    };
    let Some(output_span) = span(output, frames * 6) else {
        return SHR_PA_INVALID_ARGUMENT;
    };
    if overlaps(input_span, output_span)
        || overlaps(input_span, handle_span)
        || overlaps(output_span, handle_span)
    {
        return SHR_PA_INVALID_ARGUMENT;
    }
    // SAFETY: caller supplies the live, exclusively owned handle; simple pointer
    // shapes and all overlaps were rejected before constructing any references.
    let state = unsafe { &mut *handle };
    if frames > state.max_block {
        return SHR_PA_INVALID_ARGUMENT;
    }
    // SAFETY: the caller guarantees allocation lengths and validity. Arrays have
    // the same contiguous layout/alignment as their elements, and spans disjoint.
    let input = unsafe { slice::from_raw_parts(input.cast::<[f64; 2]>(), frames) };
    let output = unsafe { slice::from_raw_parts_mut(output.cast::<[f64; 6]>(), frames) };
    if state.engine.render_f64(input, output).is_err() {
        return SHR_PA_INVALID_ARGUMENT;
    }
    if state.engine.faulted() {
        SHR_PA_FAULT
    } else {
        SHR_PA_OK
    }
}

/// The fixed full-range preset introduces no lookahead, delay or block buffering.
/// The startup gain ramp is not a signal delay. Device/host buffering is separate.
#[unsafe(no_mangle)]
pub extern "C" fn shr_pa_v1_delay_frames() -> u32 {
    0
}

/// Release a handle on the controller after processing has stopped. Null is safe.
///
/// # Safety
/// A nonnull handle must be a live result of create, destroyed exactly once, with
/// no concurrent calls or remaining users. No pointer-validity test can prove this.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v1_destroy(handle: *mut ShrPaV1) {
    if !handle.is_null() {
        // SAFETY: ownership returns exactly once from the caller under this contract.
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Fixed capability descriptor; all fields have exact 32-bit C representation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShrPaDescriptorV1 {
    pub version: u32,
    pub size: u32,
    pub input_channels: u32,
    pub logical_outputs: u32,
    pub active_output_mask: u32,
    pub silent_output_mask: u32,
    pub physical_io_owned: u32,
    pub sample_format: u32,
    pub fixed_preset: u32,
    pub limiter_kind: u32,
    pub limiter_threshold_millidbfs: i32,
    pub limiter_linked: u32,
    pub limiter_release_ms: u32,
    pub startup_ramp_ms: u32,
    pub fixed_delay_frames: u32,
    pub min_rate: u32,
    pub max_rate: u32,
    pub min_block: u32,
    pub max_block: u32,
    pub unavailable_capabilities: u32,
}

/// Quiesced handle health; no counters, meters or physical acceptance inferred.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShrPaStatusV1 {
    pub version: u32,
    pub size: u32,
    pub sample_rate: u32,
    pub max_block: u32,
    pub fault_latched: u32,
    pub recreate_required: u32,
}

fn query_span<T>(output: *mut T, version: u32, size: u32) -> Option<(usize, usize)> {
    if version != 1 || size as usize != mem::size_of::<T>() {
        return None;
    }
    span(output, 1)
}

/// Read fixed capabilities without an engine handle; invalid output stays unchanged.
///
/// # Safety
/// Output must be a live writable allocation of the exact requested struct size.
/// Pointer shape checks cannot prove allocation validity or absence of races.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v1_descriptor(
    output: *mut ShrPaDescriptorV1,
    version: u32,
    size: u32,
) -> i32 {
    if query_span(output, version, size).is_none() {
        return SHR_PA_INVALID_ARGUMENT;
    }
    let value = ShrPaDescriptorV1 {
        version: 1,
        size: 80,
        input_channels: 2,
        logical_outputs: 6,
        active_output_mask: 3,
        silent_output_mask: 60,
        physical_io_owned: 0,
        sample_format: 1,
        fixed_preset: 1,
        limiter_kind: 1,
        limiter_threshold_millidbfs: -1000,
        limiter_linked: 1,
        limiter_release_ms: 100,
        startup_ramp_ms: 5,
        fixed_delay_frames: 0,
        min_rate: 8000,
        max_rate: 192000,
        min_block: 1,
        max_block: 8192,
        unavailable_capabilities: 15,
    };
    // SAFETY: validated shape; caller owns the live output allocation.
    unsafe { ptr::write(output, value) };
    SHR_PA_OK
}

/// Read health while this handle is quiesced. A latched fault is data, not a query error.
///
/// # Safety
/// Handle must be live and single-owner, with no concurrent process/query/destroy.
/// Output must be live writable storage disjoint from all handle-owned storage.
/// Checks reject shape/overlap errors but cannot prove foreign allocation validity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shr_pa_v1_status(
    handle: *const ShrPaV1,
    output: *mut ShrPaStatusV1,
    version: u32,
    size: u32,
) -> i32 {
    let Some(handle_span) = span(handle, 1) else {
        return SHR_PA_INVALID_ARGUMENT;
    };
    let Some(output_span) = query_span(output, version, size) else {
        return SHR_PA_INVALID_ARGUMENT;
    };
    if overlaps(handle_span, output_span) {
        return SHR_PA_INVALID_ARGUMENT;
    }
    // SAFETY: caller guarantees live quiesced handle; inline overlap rejected.
    let state = unsafe { &*handle };
    if state
        .engine
        .owned_spans()
        .into_iter()
        .any(|owned| overlaps(owned, output_span))
    {
        return SHR_PA_INVALID_ARGUMENT;
    }
    let fault = u32::from(state.engine.faulted());
    let value = ShrPaStatusV1 {
        version: 1,
        size: 24,
        sample_rate: state.sample_rate,
        max_block: state.max_block as u32,
        fault_latched: fault,
        recreate_required: fault,
    };
    // SAFETY: validated shape and disjoint storage; caller owns output allocation.
    unsafe { ptr::write(output, value) };
    SHR_PA_OK
}

const _: () = assert!(mem::size_of::<ShrPaDescriptorV1>() == 80);
const _: () = assert!(mem::size_of::<ShrPaStatusV1>() == 24);

#[cfg(test)]
mod query_tests {
    use super::*;
    #[test]
    fn ffi_status_refuses_all_owned_heap_storage_without_writes() {
        let handle = shr_pa_v1_create(48000, 64);
        // SAFETY: this test exclusively owns a freshly created live handle.
        unsafe {
            for (start, end) in (*handle).engine.owned_spans() {
                assert!(end - start >= mem::size_of::<ShrPaStatusV1>());
                let before = slice::from_raw_parts(start as *const u8, 24).to_vec();
                assert_eq!(
                    shr_pa_v1_status(handle, start as *mut _, 1, 24),
                    SHR_PA_INVALID_ARGUMENT
                );
                assert_eq!(slice::from_raw_parts(start as *const u8, 24), before);
                let tail = (end - 4) as *mut ShrPaStatusV1;
                assert_eq!(
                    shr_pa_v1_status(handle, tail, 1, 24),
                    SHR_PA_INVALID_ARGUMENT
                );
            }
            shr_pa_v1_destroy(handle);
        }
    }
}
