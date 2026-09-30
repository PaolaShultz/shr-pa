//! One fixed-size parameter slot, never an audio queue. Busy means retry unchanged.
use crate::dsp::{Engine, Prepared};
use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, AtomicU64, Ordering},
};

/// State 0 empty, 1 producer owns, 2 published, 3 consumer owns.
/// CAS also makes accidental multiple producers/consumers safe (one wins).
/// No destructor-bearing data crosses this boundary: Prepared is Copy.
pub struct Handoff {
    state: AtomicU8,
    slot: UnsafeCell<MaybeUninit<Prepared>>,
    pub accepted: AtomicU64,
    pub rejected: AtomicU64,
}
// SAFETY: slot access is exclusive under state CAS. Release/Acquire publishes
// initialized bytes and finishes reads before reuse. No slot references escape.
unsafe impl Sync for Handoff {}
impl Default for Handoff {
    fn default() -> Self {
        Self {
            state: AtomicU8::new(0),
            slot: UnsafeCell::new(MaybeUninit::uninit()),
            accepted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
        }
    }
}
impl Handoff {
    pub fn publish(&self, change: Prepared) -> Result<(), &'static str> {
        self.state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| "control busy; edit retained for retry")?;
        // SAFETY: state 1 is exclusively owned by this invocation.
        unsafe {
            (*self.slot.get()).write(change);
        }
        self.state.store(2, Ordering::Release);
        Ok(())
    }
    pub fn pending(&self) -> bool {
        self.state.load(Ordering::Acquire) != 0
    }
    /// At most one transaction per block; do not consume during a transition.
    pub fn service(&self, engine: &mut Engine) {
        if engine.busy() && !engine.faulted() {
            return;
        }
        if self
            .state
            .compare_exchange(2, 3, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
        // SAFETY: state 3 owns an initialized slot; Prepared has no destructor.
        let change = unsafe { (*self.slot.get()).assume_init_read() };
        if engine.apply(change).is_ok() {
            self.accepted.fetch_add(1, Ordering::Release);
        } else {
            self.rejected.fetch_add(1, Ordering::Release);
        }
        self.state.store(0, Ordering::Release);
    }
}

/// Independent latest runtime gain and source/capture targets for an explicit source.
/// Preparation (including exponentiation) stays on the controller. Each atomic
/// coalesces its own edits; the two targets are not a coherent transaction.
#[derive(Default)]
pub struct GeneratorControl {
    scale: AtomicU64,
    enabled: AtomicU8,
}
impl GeneratorControl {
    /// Only an existing explicit live source can consume this request.
    /// 0 retains startup behavior, 1 restores capture, 2 resumes the source.
    pub fn request_enabled(&self, enabled: bool) {
        self.enabled
            .store(if enabled { 2 } else { 1 }, Ordering::Release);
    }
    pub fn service_live(&self, source: &mut crate::offline::LiveGenerator) {
        self.service(&mut source.generator);
        match self.enabled.load(Ordering::Acquire) {
            1 => source.set_enabled(false),
            2 => source.set_enabled(true),
            _ => {}
        }
    }
    pub fn request(&self, level: crate::offline::GeneratorLevel) {
        self.scale.store(level.scale().to_bits(), Ordering::Release);
    }
    /// Called once at a block boundary, only for an explicitly started source.
    pub fn service(&self, generator: &mut crate::offline::Generator) {
        let bits = self.scale.load(Ordering::Acquire);
        if bits != 0 {
            generator.ramp_scale(f64::from_bits(bits));
        }
    }
}
