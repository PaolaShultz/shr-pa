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
