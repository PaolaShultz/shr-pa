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
fn native_f64_ffi_processing_faults_and_argument_errors_never_allocate_or_free() {
    use shr_pa::ffi::*;
    let handle = shr_pa_v1_create(48000, 128);
    assert!(!handle.is_null());
    let mut input = [[0.1_f64, -0.2]; 128];
    let mut output = [[0_f64; 6]; 128];
    COUNT.with(|c| c.set(Some(0)));
    for _ in 0..100 {
        assert_eq!(
            unsafe {
                shr_pa_v1_process(
                    handle,
                    input.as_ptr().cast(),
                    output.as_mut_ptr().cast(),
                    128,
                )
            },
            SHR_PA_OK
        );
    }
    input[64][0] = f64::NAN;
    assert_eq!(
        unsafe {
            shr_pa_v1_process(
                handle,
                input.as_ptr().cast(),
                output.as_mut_ptr().cast(),
                128,
            )
        },
        SHR_PA_FAULT
    );
    assert_eq!(
        unsafe { shr_pa_v1_process(handle, input.as_ptr().cast(), output.as_mut_ptr().cast(), 0) },
        SHR_PA_INVALID_ARGUMENT
    );
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    unsafe { shr_pa_v1_destroy(handle) };
}

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
    let mut eq = shr_pa::library::EqState::new(&c);
    let mut curve = c;
    eq.geq(&mut curve, shr_pa::library::GeqMode::Speech);
    let curve_edit = Prepared::new(curve, false).unwrap();
    eq.geq(&mut curve, shr_pa::library::GeqMode::Manual);
    eq.peq(&mut curve, true, 0, false).unwrap();
    eq.peq(&mut curve, false, 2, false).unwrap();
    let flat_edit = Prepared::new(curve, false).unwrap();
    eq.peq(&mut curve, true, 0, true).unwrap();
    eq.peq(&mut curve, false, 2, true).unwrap();
    let restore_edit = Prepared::new(curve, false).unwrap();
    let mut crossover = next;
    let mut pairs = crossover.layout_edges();
    for pair in &mut pairs {
        pair.hp.bypass = false;
        pair.hp.slope = 48;
        pair.lp.bypass = false;
        pair.lp.slope = 48;
        pair.lp.family = shr_pa::config::Family::Butterworth;
    }
    crossover.crossover = shr_pa::config::Crossover::Independent(pairs);
    let edge_edit = Prepared::new(crossover, false).unwrap();
    pairs[0].hp.bypass = true;
    pairs[1].lp.slope = 42;
    crossover.crossover = shr_pa::config::Crossover::Independent(pairs);
    let bypass_edit = Prepared::new(crossover, false).unwrap();
    let edits = [
        edge_edit,
        bypass_edit,
        curve_edit,
        flat_edit,
        restore_edit,
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

#[test]
fn pink_generation_and_six_output_processing_never_allocate_or_free() {
    use shr_pa::{
        config::Config,
        dsp::Engine,
        offline::{Generator, GeneratorLevel, LiveGenerator, Signal},
    };
    let g = Generator::with_level(
        Signal::Pink,
        48000,
        131072,
        GeneratorLevel::new(-7.5).unwrap(),
    )
    .unwrap();
    let mut g = LiveGenerator::new(g);
    let mut engine = Engine::new(Config::default()).unwrap();
    let mut input = [[0.; 2]; 128];
    let mut output = [[0.; 6]; 128];
    let control = shr_pa::control::GeneratorControl::default();
    let levels = [-60., 0., -20.].map(|db| GeneratorLevel::new(db).unwrap());
    COUNT.with(|c| c.set(Some(0)));
    engine.set_mutes([false; 6]);
    for i in 0..1024 {
        control.request(levels[i % levels.len()]);
        control.request_enabled(i % 7 < 3);
        control.service_live(&mut g);
        input.fill([0.2, -0.3]);
        g.mix_capture(&mut input);
        engine.render(&input, &mut output).unwrap();
    }
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    assert!(!engine.faulted());
    assert!(output.iter().any(|frame| frame[4] != 0.));
}

#[test]
fn ffi_readonly_queries_never_allocate_or_free() {
    use shr_pa::ffi::*;
    let handle = shr_pa_v1_create(48000, 128);
    assert!(!handle.is_null());
    let mut descriptor = ShrPaDescriptorV1::default();
    let mut status = ShrPaStatusV1::default();
    COUNT.with(|c| c.set(Some(0)));
    for _ in 0..100 {
        unsafe {
            assert_eq!(shr_pa_v1_descriptor(&mut descriptor, 1, 80), SHR_PA_OK);
            assert_eq!(shr_pa_v1_status(handle, &mut status, 1, 24), SHR_PA_OK);
            assert_eq!(
                shr_pa_v1_status(handle, &mut status, 2, 24),
                SHR_PA_INVALID_ARGUMENT
            );
            assert_eq!(
                shr_pa_v1_status(handle, handle.cast(), 1, 24),
                SHR_PA_INVALID_ARGUMENT
            );
        }
    }
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    unsafe { shr_pa_v1_destroy(handle) };
}

#[test]
fn configurable_graph_render_commit_retirement_and_queries_never_allocate_or_free() {
    use shr_pa::{
        ffi_v2::*,
        graph::{GraphConfig, Input, Source},
    };
    use std::ptr;
    for channels in [16, 32, 48] {
        let mut config = GraphConfig::stereo(vec![100., 600., 3000.]);
        config.inputs = vec![Input::default(); channels];
        config.outputs[0].source = Some(Source::Input(channels - 1));
        let json = serde_json::to_vec(&config).unwrap();
        let first = unsafe { shr_pa_v2_prepare(json.as_ptr(), json.len() as u32, 2) };
        let second = unsafe { shr_pa_v2_prepare(json.as_ptr(), json.len() as u32, 2) };
        assert!(!first.is_null() && !second.is_null());
        let mut active = ptr::null_mut();
        let mut retired = ptr::null_mut();
        let input = vec![0.1; channels * 256];
        let mut output = vec![0.; 8 * 256];
        let mut status = ShrPaStatusV2::default();
        COUNT.with(|c| c.set(Some(0)));
        unsafe {
            assert_eq!(shr_pa_v2_apply(&mut active, first, &mut retired, 1, 0), 0);
            assert_eq!(shr_pa_v2_rearm(active, 1, 0), 0);
            assert_eq!(
                shr_pa_v2_process(
                    active,
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    channels as u32,
                    8,
                    256,
                    1,
                    0
                ),
                0
            );
            assert_eq!(shr_pa_v2_mute(active), 0);
            assert_eq!(
                shr_pa_v2_process(
                    active,
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    channels as u32,
                    8,
                    256,
                    1,
                    256
                ),
                0
            );
            assert_eq!(shr_pa_v2_status(active, &mut status, 2, 80), 0);
            assert_eq!(status.quiesced, 1);
            assert_eq!(
                shr_pa_v2_apply(&mut active, second, &mut retired, 1, 512),
                0
            );
            assert_eq!(
                shr_pa_v2_process(
                    active,
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    channels as u32,
                    8,
                    256,
                    1,
                    1
                ),
                -4
            );
            assert_eq!(
                shr_pa_v2_process(
                    active,
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    channels as u32,
                    8,
                    256,
                    1,
                    512
                ),
                -2
            );
            assert_eq!(
                shr_pa_v2_process(
                    active,
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    1,
                    8,
                    256,
                    1,
                    512
                ),
                -1
            );
        }
        let count = COUNT.with(|c| c.replace(None).unwrap());
        assert_eq!(count, 0);
        unsafe {
            shr_pa_v2_destroy(retired);
            shr_pa_v2_destroy(active);
        }
    }
}

#[test]
fn eq_extension_apply_endpoint_busy_stale_fault_and_retirement_never_allocate_or_free() {
    use shr_pa::{
        ffi_eq::*,
        ffi_v2::*,
        graph::GraphConfig,
        live_eq::{EqPatch, EqSettings},
    };
    unsafe {
        let c = GraphConfig::stereo(vec![]);
        let full = serde_json::to_vec(&c).unwrap();
        let prepared = shr_pa_v2_prepare(full.as_ptr(), full.len() as u32, 2);
        let mut h = std::ptr::null_mut();
        let mut old = std::ptr::null_mut();
        assert_eq!(shr_pa_v2_apply(&mut h, prepared, &mut old, 1, 0), 0);
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
        let owned_span = p;
        let mut conflict = shr_pa_eq_v1_prepare(h, json.as_ptr(), json.len() as u32, 1, 1, 0, 1, 0);
        let input = [0.1; 96];
        let mut out = [0.; 96];
        let mut retired = std::ptr::null_mut();
        COUNT.with(|c| c.set(Some(0)));
        assert_eq!(shr_pa_eq_v1_apply(h, &mut p, 1, 1), -4);
        assert_eq!(shr_pa_eq_v1_apply(h, &mut p, 1, 0), 0);
        assert_eq!(shr_pa_eq_v1_apply(h, &mut conflict, 1, 0), -3);
        for phase in 0..2 {
            assert_eq!(shr_pa_eq_v1_status(h, owned_span.cast(), 1, 64), -1);
            assert_eq!(shr_pa_eq_v1_readback(h, 0, 1, owned_span.cast(), 64), -1);
            assert_eq!(shr_pa_eq_v1_retire(h, owned_span.cast()), -1);
            assert_eq!(shr_pa_eq_v1_apply(h, owned_span.cast(), 1, 0), -1);
            assert_eq!(
                shr_pa_v2_process(h, input.as_ptr(), owned_span.cast(), 2, 2, 1, 1, 0),
                -1
            );
            assert_eq!(
                shr_pa_v2_process(h, owned_span.cast(), out.as_mut_ptr(), 2, 2, 1, 1, 0),
                -1
            );
            if phase == 0 {
                for block in 0..5 {
                    assert_eq!(
                        shr_pa_v2_process(
                            h,
                            input.as_ptr(),
                            out.as_mut_ptr(),
                            2,
                            2,
                            48,
                            1,
                            block * 48
                        ),
                        0
                    );
                }
            }
        }
        assert_eq!(shr_pa_eq_v1_retire(h, &mut retired), 0);
        assert_eq!(shr_pa_eq_v1_apply(h, &mut conflict, 1, 240), -4);
        let invalid = [f64::NAN; 96];
        assert_eq!(
            shr_pa_v2_process(h, invalid.as_ptr(), out.as_mut_ptr(), 2, 2, 48, 1, 240),
            -2
        );
        assert_eq!(shr_pa_eq_v1_apply(h, &mut conflict, 1, 240), -2);
        let n = COUNT.with(|c| c.replace(None).unwrap());
        assert_eq!(n, 0);
        shr_pa_eq_v1_destroy(retired);
        shr_pa_eq_v1_destroy(conflict);
        shr_pa_v2_destroy(h);
    }
}
