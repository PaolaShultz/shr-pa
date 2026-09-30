use shr_pa::{
    config::Config,
    transport::{Mapping, SampleFormat, TransferAction, classify},
};
#[test]
fn mappings_are_local_and_never_sum_outputs() {
    assert!(Mapping::parse(2, 2, "0,1", "0,1,-,-,-,-").is_ok());
    assert!(Mapping::parse(2, 2, "1,0", "-,-,-,-,0,1").is_ok());
    assert!(Mapping::parse(2, 2, "0,1", "0,1,2,3,4,5").is_err());
    assert!(Mapping::parse(2, 2, "0,1", "0,0,-,-,-,-").is_err());
    assert!(Mapping::parse(2, 2, "0,2", "0,1,-,-,-,-").is_err());
    assert!(Mapping::parse(2, 2, "0,1", "-,-,-,-,-,-").is_ok());
    assert!(shr_pa::dsp::Engine::new(Config::default()).is_ok());
}
#[test]
fn pcm_conversion_including_packed_negative_values_and_clipping() {
    for format in [
        SampleFormat::S32,
        SampleFormat::S16,
        SampleFormat::S24Packed,
        SampleFormat::Float,
    ] {
        for x in [-2., -1., -0.7, -0.001, 0., 0.1, 0.999, 1., 2.] {
            let mut bytes = [0; 4];
            format.encode(x, &mut bytes);
            let y = format.decode(&bytes);
            assert!((y - x.clamp(-1., 1.)).abs() < 0.00004, "{format:?} {x} {y}");
        }
        let mut bytes = [0; 4];
        format.encode(f32::NAN, &mut bytes);
        assert_eq!(format.decode(&bytes), 0.);
    }
    assert_eq!(SampleFormat::S24Packed.decode(&[0, 0, 128]), -1.);
    assert_eq!(classify(11), TransferAction::Retry);
    assert_eq!(classify(4), TransferAction::Retry);
    assert_eq!(classify(32), TransferAction::Xrun);
    assert_eq!(classify(86), TransferAction::Xrun);
    assert_eq!(classify(19), TransferAction::Disconnected);
    assert_eq!(classify(5), TransferAction::Fatal);
}
#[test]
fn presets_roundtrip_reject_unknown_fields_and_preserve_old_file() {
    let path = std::env::temp_dir().join(format!("shr-pa-test-{}.json", std::process::id()));
    let c = Config::default();
    c.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), c);
    let mut invalid = c;
    invalid.sample_rate = 0;
    assert!(invalid.save(&path).is_err());
    assert_eq!(Config::load(&path).unwrap(), c);
    let mut value = serde_json::to_value(c).unwrap();
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("crossover");
    std::fs::write(&path, serde_json::to_vec(&missing).unwrap()).unwrap();
    assert!(Config::load(&path).is_err());
    value["version"] = 99.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Config::load(&path).is_err());
    value["version"] = 3.into();
    value["generator_active"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Config::load(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(c.save(&path).is_err());
    std::fs::remove_dir(&path).unwrap();
    c.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), c);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn offline_wav_is_six_channel_deterministic_and_can_read_stereo_pcm() {
    let dir = std::env::temp_dir().join(format!("shr-pa-wav-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let c = Config::default();
    let a = dir.join("a.wav");
    let b = dir.join("b.wav");
    for source in ["noise", "pink"] {
        shr_pa::offline::render(c, source, &a, 0.03).unwrap();
        shr_pa::offline::render(c, source, &b, 0.03).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }
    let r = hound::WavReader::open(&a).unwrap();
    assert_eq!(r.spec().channels, 6);
    assert_eq!(r.duration(), 1440);
    let source = dir.join("in.wav");
    let mut w = hound::WavWriter::create(
        &source,
        hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for _ in 0..500 {
        w.write_sample(1000i16).unwrap();
        w.write_sample(-2000i16).unwrap();
    }
    w.finalize().unwrap();
    let report = shr_pa::offline::render(c, source.to_str().unwrap(), &b, 1.).unwrap();
    assert_eq!(report.frames, 500);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn editor_controls_validate_and_preview_real_dsp() {
    let mut e = shr_pa::ui::Editor::new().unwrap();
    e.key('+');
    assert_eq!(e.config.bands[0].gain_db, 1.);
    e.key('b');
    e.key('p');
    assert!(e.config.bands[1].inverted);
    e.key('u');
    e.key('r');
    assert!(
        e.screen(shr_pa::ui::Page::Home, 40, 13)
            .iter()
            .any(|s| s.contains("Rendered offline"))
    );
    assert!(e.config.validate().is_ok());
}

#[test]
fn partial_io_retries_preserve_offsets_and_faults_stop_immediately() {
    use shr_pa::transport::transfer_frames;
    let mut results = [
        Ok(2),
        Err(alsa::Error::new("read", 11)),
        Ok(0),
        Err(alsa::Error::new("read", 4)),
        Ok(1),
        Ok(4),
    ]
    .into_iter();
    let mut offsets = Vec::new();
    let mut waits = 0;
    transfer_frames(
        7,
        || false,
        |offset| {
            offsets.push(offset);
            results.next().unwrap()
        },
        || {
            waits += 1;
            Ok(true)
        },
    )
    .unwrap();
    assert_eq!(offsets, [0, 2, 2, 2, 2, 3]);
    assert_eq!(waits, 3);
    for errno in [32, 86, 19, 5] {
        let mut calls = 0;
        let error = transfer_frames(
            7,
            || false,
            |_| {
                calls += 1;
                Err(alsa::Error::new("test", errno))
            },
            || panic!("must not retry fault"),
        )
        .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(error.downcast_ref::<alsa::Error>().unwrap().errno(), errno);
    }
    assert!(transfer_frames(1, || true, |_| panic!("stopped"), || Ok(true)).is_err());
    assert!(transfer_frames(1, || false, |_| Ok(2), || Ok(true)).is_err());
}

#[test]
fn physical_playback_contains_only_explicitly_selected_outputs() {
    let m = Mapping::parse(2, 4, "0,1", "-,-,2,-,0,-").unwrap();
    let frames = [
        [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
        [-0.1, -0.2, -0.3, -0.4, -0.5, -0.6],
    ];
    let mut bytes = [255; 32];
    m.pack(&frames, SampleFormat::Float, &mut bytes).unwrap();
    let decoded: Vec<_> = bytes
        .chunks_exact(4)
        .map(|b| SampleFormat::Float.decode(b))
        .collect();
    assert_eq!(decoded, [0.5, 0., 0.3, 0., -0.5, 0., -0.3, 0.]);
    assert!(
        m.pack(&frames, SampleFormat::Float, &mut bytes[..31])
            .is_err()
    );
}

#[test]
fn software_null_pcm_cancel_before_start_does_not_wait_for_capture() {
    use shr_pa::transport::{LiveOptions, Shared};
    let shared = Shared::default();
    shared
        .stop
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let report = shr_pa::transport::run(
        Config::default(),
        LiveOptions {
            capture: "null",
            playback: "null",
            map: Mapping::parse(2, 2, "0,1", "0,1,-,-,-,-").unwrap(),
            seconds: 1.,
            signal: None,
            generator_level: None,
        },
        &shared,
    )
    .unwrap();
    assert!(report.fault.is_none(), "{report:?}");
    assert_eq!(report.frames, 0);
}
