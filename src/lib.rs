//! Fixed 2 x 6 PA processing, independent of physical channel availability.
pub mod commands;
pub mod config;
pub mod control;
pub mod dsp;
pub mod ffi;
pub mod library;
pub mod offline;
pub mod transport;
pub mod ui;

pub mod ffi_v2;
pub mod graph;

pub mod live_eq;

pub mod ffi_eq;

#[cfg(feature = "owner-allocation-guard")]
mod owner_allocation_guard {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };
    thread_local! { static COUNT: Cell<Option<u64>> = const { Cell::new(None) }; }
    struct Guard;
    fn event() {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n.saturating_add(1)));
            }
        });
    }
    unsafe impl GlobalAlloc for Guard {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            event();
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            event();
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            event();
            unsafe { System.realloc(p, l, n) }
        }
    }
    #[global_allocator]
    static ALLOC: Guard = Guard;
    #[unsafe(no_mangle)]
    pub extern "C" fn shr_pa_owner_allocation_guard_begin() {
        COUNT.with(|c| c.set(Some(0)));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn shr_pa_owner_allocation_guard_end() -> u64 {
        COUNT.with(|c| c.replace(None).unwrap_or(u64::MAX))
    }
}
