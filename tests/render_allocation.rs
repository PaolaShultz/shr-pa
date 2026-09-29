use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct Counting;
thread_local! { static COUNT: Cell<Option<usize>> = const { Cell::new(None) }; }
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;
#[test]
fn render_and_mute_commands_never_allocate_or_free() {
    let mut c = shr_pa::config::Config {
        input_delay_ms: 100.,
        ..Default::default()
    };
    for b in &mut c.bands {
        b.delay_ms = 10.;
        for e in &mut b.eq {
            e.db = 6.;
        }
    }
    let mut e = shr_pa::dsp::Engine::new(c).unwrap();
    let input = [[0.1; 2]; 128];
    let mut output = [[0.; 6]; 128];
    COUNT.with(|c| c.set(Some(0)));
    for _ in 0..100 {
        e.set_mutes([false; 6]);
        e.render(&input, &mut output).unwrap();
    }
    let mut bad = input;
    bad[0][0] = f32::INFINITY;
    e.render(&bad, &mut output).unwrap();
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
}

#[test]
fn prepared_handoff_full_processing_transitions_recall_and_fault_never_allocate() {
    use shr_pa::{
        config::{Config, EqKind},
        control::Handoff,
        dsp::{Engine, Prepared},
    };
    let mut c = Config::default();
    c.geq.enabled = true;
    c.geq.db = [[1.; 31]; 2];
    c.compressor.enabled = true;
    for es in &mut c.input_eq {
        for e in es {
            e.db = -1.;
        }
    }
    for b in &mut c.bands {
        for e in &mut b.eq {
            e.kind = EqKind::LowShelf;
            e.db = 1.;
        }
    }
    let mut e = Engine::new(c).unwrap();
    let mut next = c;
    next.input_delay_ms = 100.;
    next.geq.db = [[-1.; 31]; 2];
    for b in &mut next.bands {
        b.delay_ms = 10.;
        b.inverted = true;
        b.limiter_db = -20.;
        for e in &mut b.eq {
            e.kind = EqKind::HighShelf;
        }
    }
    let edits = [
        Prepared::new(next, false).unwrap(),
        Prepared::new(c, true).unwrap(),
        Prepared::new(next, false).unwrap(),
    ];
    let slot = Handoff::default();
    let input = [[0.1; 2]; 128];
    let mut out = [[0.; 6]; 128];
    COUNT.with(|c| c.set(Some(0)));
    for p in edits {
        slot.publish(p).unwrap();
        for _ in 0..60 {
            slot.service(&mut e);
            e.set_mutes([false; 6]);
            e.render(&input, &mut out).unwrap();
        }
    }
    let bad = [[f32::NAN; 2]; 128];
    e.render(&bad, &mut out).unwrap();
    slot.publish(edits[0]).unwrap();
    slot.service(&mut e);
    e.render(&input, &mut out).unwrap();
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    assert_eq!(out, [[0.; 6]; 128]);
}
