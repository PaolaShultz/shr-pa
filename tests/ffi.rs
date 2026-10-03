use shr_pa::{
    config::{Config, Layout},
    dsp::Engine,
    ffi::*,
};
use std::ptr;

struct Handle(*mut ShrPaV1);
impl Handle {
    fn new(block: u32) -> Self {
        let handle = shr_pa_v1_create(48000, block);
        assert!(!handle.is_null());
        Self(handle)
    }
    fn process(&mut self, input: &[[f64; 2]], output: &mut [[f64; 6]]) -> i32 {
        assert_eq!(input.len(), output.len());
        unsafe {
            shr_pa_v1_process(
                self.0,
                input.as_ptr().cast(),
                output.as_mut_ptr().cast(),
                input.len() as u32,
            )
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { shr_pa_v1_destroy(self.0) };
    }
}

#[test]
fn creation_validates_rate_block_and_reports_no_algorithmic_delay() {
    for (rate, block) in [(0, 1), (7999, 1), (192001, 1), (48000, 0), (48000, 8193)] {
        assert!(shr_pa_v1_create(rate, block).is_null());
    }
    for (rate, block) in [(8000, 1), (192000, 8192), (48000, 128)] {
        let handle = shr_pa_v1_create(rate, block);
        assert!(!handle.is_null());
        unsafe { shr_pa_v1_destroy(handle) };
    }
    unsafe { shr_pa_v1_destroy(ptr::null_mut()) };
    assert_eq!(shr_pa_v1_delay_frames(), 0);
}

#[test]
fn ffi_preserves_native_f64_stereo_gain_and_startup_ramp() {
    let mut handle = Handle::new(256);
    let sample = 0.25 + 2_f64.powi(-40);
    assert_ne!(sample, f64::from(sample as f32));
    let input = [[sample, -sample * 0.5]; 256];
    let mut output = [[0.; 6]; 256];
    assert_eq!(handle.process(&input, &mut output), SHR_PA_OK);
    assert!((output[0][0] - sample / 240.).abs() < 1e-16);
    assert_eq!(output[255][0], sample);
    assert_eq!(output[255][1], -sample * 0.5);
    assert!(output.iter().all(|frame| frame[2..] == [0.; 4]));
    assert!(output.windows(2).all(|frames| frames[1][0] >= frames[0][0]));
}

#[test]
fn ffi_keeps_linked_sample_protection_active() {
    let mut handle = Handle::new(256);
    let mut output = [[0.; 6]; 256];
    assert_eq!(handle.process(&[[0.; 2]; 256], &mut output), SHR_PA_OK);
    assert_eq!(handle.process(&[[4., 1.]; 256], &mut output), SHR_PA_OK);
    let ceiling = 10_f64.powf(-1. / 20.);
    for frame in output {
        assert!((frame[0] - ceiling).abs() < 1e-15);
        assert_eq!(frame[1], frame[0] / 4.);
        assert_eq!(frame[2..], [0.; 4]);
    }
}

#[test]
fn numeric_fault_clears_whole_block_and_requires_new_handle() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e100] {
        let mut handle = Handle::new(64);
        let mut input = [[0.1, -0.2]; 64];
        input[32][1] = bad;
        let mut output = [[42.; 6]; 64];
        assert_eq!(handle.process(&input, &mut output), SHR_PA_FAULT);
        assert_eq!(output, [[0.; 6]; 64]);
        input[32][1] = 0.2;
        output.fill([42.; 6]);
        assert_eq!(handle.process(&input, &mut output), SHR_PA_FAULT);
        assert_eq!(output, [[0.; 6]; 64]);
        let mut fresh = Handle::new(64);
        assert_eq!(fresh.process(&input, &mut output), SHR_PA_OK);
        assert!(output[0][0] > 0.);
    }
}

#[test]
fn bad_arguments_are_rejected_before_output_or_state_changes() {
    let mut handle = Handle::new(64);
    let input = [[0.1; 2]; 65];
    let mut output = [[42.; 6]; 65];
    let i = input.as_ptr().cast::<f64>();
    let o = output.as_mut_ptr().cast::<f64>();
    unsafe {
        for (h, i, o, n) in [
            (ptr::null_mut(), i, o, 64),
            (handle.0, ptr::null(), o, 64),
            (handle.0, i, ptr::null_mut(), 64),
            (handle.0, i, o, 0),
            (handle.0, i, o, 65),
            (handle.0, i, o, 8193),
            (handle.0, i.cast::<u8>().add(1).cast(), o, 64),
            (handle.0, i, o.cast::<u8>().add(1).cast(), 64),
            (handle.0, o.cast_const(), o, 64),
            (handle.0, o.add(2).cast_const(), o, 64),
        ] {
            assert_eq!(shr_pa_v1_process(h, i, o, n), SHR_PA_INVALID_ARGUMENT);
        }
    }
    assert_eq!(output, [[42.; 6]; 65]);
    assert_eq!(handle.process(&input[..64], &mut output[..64]), SHR_PA_OK);
    assert!((output[0][0] - 0.1 / 240.).abs() < 1e-16);
}

#[test]
fn native_f64_and_legacy_f32_share_all_layout_processing() {
    for layout in [
        Layout::FullRange,
        Layout::External,
        Layout::TwoWay,
        Layout::ThreeWay,
        Layout::SixFullRange,
        Layout::FourPlusSubs,
    ] {
        let mut config = Config {
            max_block: 37,
            layout,
            input_delay_ms: 0.2,
            ..Config::default()
        };
        config.geq.enabled = true;
        config.geq.db = [[1.; 31]; 2];
        config.compressor.enabled = true;
        for band in &mut config.bands {
            band.delay_ms = 0.1;
            band.eq[0].db = 1.5;
        }
        let mut narrow = Engine::new(config).unwrap();
        let mut wide = Engine::new(config).unwrap();
        narrow.set_mutes([false; 6]);
        wide.set_mutes([false; 6]);
        for block in 0..20 {
            let input: [[f32; 2]; 37] = std::array::from_fn(|n| {
                let t = (block * 37 + n) as f32;
                [(t * 0.013).sin() * 0.9, (t * 0.027).cos() * 0.3]
            });
            let mut f32_output = [[0.; 6]; 37];
            let mut f64_output = [[0.; 6]; 37];
            narrow.render(&input, &mut f32_output).unwrap();
            wide.render_f64(&input.map(|f| f.map(f64::from)), &mut f64_output)
                .unwrap();
            assert_eq!(f32_output, f64_output.map(|f| f.map(|x| x as f32)));
        }
    }
}
