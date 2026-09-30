use shr_pa::{
    config::Config,
    library::{EqState, GeqMode, Library, Preset, Working},
    ui::Editor,
};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "shr-pa-library-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn working(c: Config) -> Working {
    Working {
        version: 2,
        selected: "U75".into(),
        active: "T4".into(),
        saved: Config::default(),
        saved_eq: EqState::new(&Config::default()),
        preset: Preset::new("Working", c, EqState::new(&c)),
    }
}
#[test]
fn slots_templates_names_atomic_replace_and_single_writer() {
    let t = Temp::new();
    let lib = Library::open(&t.0).unwrap();
    assert!(Library::open(&t.0).is_err());
    let p = lib.load("T4").unwrap();
    for n in 1..=75 {
        lib.save(&format!("U{n}"), &p, false).unwrap();
        assert_eq!(lib.load(&format!("U{n}")).unwrap(), p);
    }
    for id in ["U0", "U76", "T0", "T7", "../U1"] {
        assert!(lib.load(id).is_err());
    }
    assert!(lib.save("T4", &p, true).is_err());
    assert!(lib.save("U75", &p, false).is_err());
    let mut bad = p.clone();
    bad.name = "\n".into();
    assert!(lib.save("U75", &bad, true).is_err());
    bad.name = "A renamed copy".into();
    bad.config.input_gain_db = 99.;
    assert!(lib.save("U75", &bad, true).is_err());
    assert_eq!(lib.load("U75").unwrap(), p);
    bad.config.input_gain_db = -3.;
    lib.save("U75", &bad, true).unwrap();
    assert_eq!(lib.load("U75").unwrap(), bad);
    assert!(
        !fs::read_dir(&t.0).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp"))
    );
}
#[test]
fn recovery_rejects_corruption_unknown_versions_incomplete_and_incompatible_without_overwriting() {
    let t = Temp::new();
    let mut lib = Library::open(&t.0).unwrap();
    let c = Config::default();
    assert!(lib.recover(&c).unwrap().is_none());
    let w = working(Config {
        input_gain_db: -6.,
        ..c
    });
    lib.checkpoint(&w).unwrap();
    fs::write(t.0.join(".shr-pa-interrupted.tmp"), b"incomplete").unwrap();
    assert_eq!(lib.recover(&c).unwrap().unwrap().preset, w.preset);
    let mut future = serde_json::to_value(&w).unwrap();
    future["version"] = 99.into();
    let mut incompatible = w.clone();
    incompatible.preset.config.sample_rate = 44100;
    let mut unsafe_field = serde_json::to_value(&w).unwrap();
    unsafe_field["generator_active"] = true.into();
    let mut invalid_history = w.clone();
    invalid_history.preset.eq.manual[0][0] = 13.;
    for bytes in [
        b"{".to_vec(),
        b"{}".to_vec(),
        serde_json::to_vec(&future).unwrap(),
        serde_json::to_vec(&incompatible).unwrap(),
        serde_json::to_vec(&unsafe_field).unwrap(),
        serde_json::to_vec(&invalid_history).unwrap(),
    ] {
        fs::write(t.0.join("working.json"), &bytes).unwrap();
        assert!(lib.recover(&c).is_err());
        assert!(lib.checkpoint(&w).is_err());
        assert_eq!(fs::read(t.0.join("working.json")).unwrap(), bytes);
        lib.reset_recovery().unwrap();
        lib.checkpoint(&w).unwrap();
    }
    assert_eq!(
        fs::read_dir(&t.0)
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("working-rejected-"))
            .count(),
        6
    );
}
#[test]
fn eq_curves_manual_and_scoped_peq_restore_preserve_unrelated_values() {
    let mut c = Config::default();
    c.geq.db[0][2] = 4.;
    c.geq.db[1][3] = -2.;
    c.geq.linked = false;
    c.input_eq[1][3].db = 5.;
    c.bands[2].eq[4].db = -7.;
    c.bands[2].delay_ms = 3.;
    let original = c;
    let mut eq = EqState::new(&c);
    for mode in [
        GeqMode::Speech,
        GeqMode::Warm,
        GeqMode::Gentle,
        GeqMode::Flat,
    ] {
        eq.geq(&mut c, mode);
        c.validate().unwrap();
        assert_eq!(eq.manual, original.geq.db);
    }
    eq.geq(&mut c, GeqMode::Manual);
    assert_eq!(c, original);
    eq.peq(&mut c, true, 1, false).unwrap();
    eq.peq(&mut c, true, 1, false).unwrap();
    assert_eq!(c.input_eq[1][3].db, 0.);
    assert_eq!(c.input_eq[0], original.input_eq[0]);
    eq.peq(&mut c, false, 2, false).unwrap();
    assert_eq!(c.bands[2].eq[4].db, 0.);
    assert_eq!(c.bands[2].delay_ms, 3.);
    assert_eq!(c.bands[0], original.bands[0]);
    eq.peq(&mut c, true, 1, true).unwrap();
    eq.peq(&mut c, false, 2, true).unwrap();
    assert_eq!(c, original);
    assert!(eq.peq(&mut c, false, 2, true).is_err());
    assert!(eq.peq(&mut c, true, 2, false).is_err());
}
#[test]
fn editor_selection_copy_recall_recovery_and_persisted_history() {
    let t = Temp::new();
    let mut e = Editor::new().unwrap();
    e.attach_library(t.0.clone());
    e.key('g');
    let edited = e.config;
    e.command("select T1");
    assert_eq!(e.config, edited);
    e.command("copy U75 My full range");
    assert_eq!(e.config, edited);
    e.command("select U75");
    e.command("save Changed");
    assert!(e.message().contains("occupied"));
    e.command("recall");
    assert_eq!(e.config.layout, shr_pa::config::Layout::FullRange);
    e.key('e');
    let manual = e.config;
    e.command("flat H");
    e.command("geq speech");
    assert!(e.modified());
    let pending = e.config;
    drop(e);
    let mut e = Editor::new().unwrap();
    e.attach_library(t.0.clone());
    assert_eq!(e.config, pending);
    assert!(e.message().contains("Recovered"));
    assert!(e.modified());
    e.command("restore H");
    e.command("geq manual");
    assert_eq!(e.config, manual);
    e.command("save! New name");
    assert!(!e.modified());
    e.command("select T4");
    assert_eq!(e.config, manual);
    for page in [shr_pa::ui::Page::Home, shr_pa::ui::Page::Features] {
        let rows = e.screen(page, 40, 13);
        assert_eq!(rows.len(), 13);
        assert!(rows.iter().all(|l| l.chars().count() <= 40));
    }
    drop(e);
    let lib = Library::open(&t.0).unwrap();
    assert_eq!(lib.load("U75").unwrap().config, manual);
}
#[test]
fn incompatible_library_recall_is_rejected_before_any_edit_or_checkpoint() {
    let t = Temp::new();
    let lib = Library::open(&t.0).unwrap();
    let mut p = lib.load("T4").unwrap();
    p.config.max_block = 256;
    lib.save("U1", &p, false).unwrap();
    drop(lib);
    let mut e = Editor::new().unwrap();
    e.attach_library(t.0.clone());
    e.key('g');
    let before = e.config;
    let bytes = fs::read(t.0.join("working.json")).unwrap();
    e.command("recall");
    assert!(e.message().contains("requires restart"));
    assert_eq!(e.config, before);
    assert_eq!(fs::read(t.0.join("working.json")).unwrap(), bytes);
}

#[test]
fn eq_transactions_retry_after_backpressure_preserve_other_pair_state_and_faults() {
    use shr_pa::{
        config::Layout,
        control::Handoff,
        dsp::{Engine, Prepared},
    };
    let mut c = Config {
        layout: Layout::SixFullRange,
        ..Config::default()
    };
    c.bands[0].eq[0].db = 6.;
    c.bands[1].eq[0].db = -4.;
    c.bands[1].delay_ms = 2.;
    let mut eq = EqState::new(&c);
    let mut e = Engine::new(c).unwrap();
    let mut reference = Engine::new(c).unwrap();
    let input = [[0.01; 2]; 128];
    let mut out = [[0.; 6]; 128];
    let mut expected = out;
    let mutes = [false, false, false, false, true, true];
    e.set_mutes(mutes);
    reference.set_mutes(mutes);
    for _ in 0..30 {
        e.render(&input, &mut out).unwrap();
        reference.render(&input, &mut expected).unwrap();
    }
    eq.peq(&mut c, false, 0, false).unwrap();
    let flatten = Prepared::new(c, false).unwrap();
    eq.peq(&mut c, false, 0, true).unwrap();
    let restore = Prepared::new(c, false).unwrap();
    let h = Handoff::default();
    h.publish(flatten).unwrap();
    assert!(h.publish(restore).is_err());
    h.service(&mut e);
    h.publish(restore).unwrap();
    for _ in 0..50 {
        h.service(&mut e);
        e.render(&input, &mut out).unwrap();
        reference.render(&input, &mut expected).unwrap();
        for (a, b) in out.iter().zip(expected) {
            assert_eq!(&a[2..], &b[2..]);
        }
    }
    assert_eq!(h.accepted.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert!(!e.busy());
    assert_eq!(out, expected);
    e.render(&[[f32::NAN; 2]; 128], &mut out).unwrap();
    h.publish(Prepared::new(c, true).unwrap()).unwrap();
    h.service(&mut e);
    e.set_mutes([false; 6]);
    e.render(&input, &mut out).unwrap();
    assert!(e.faulted());
    assert_eq!(out, [[0.; 6]; 128]);
}

#[test]
fn explicit_v2_and_envelope_migration_preserves_sources_baselines_and_eq_histories() {
    fn legacy(c: Config) -> serde_json::Value {
        let mut v = serde_json::to_value(c).unwrap();
        v["version"] = 2.into();
        v.as_object_mut().unwrap().remove("crossover");
        v
    }
    let t = Temp::new();
    let mut c = Config::default();
    c.input_eq[1][3].db = 7.;
    c.bands[2].inverted = true;
    c.bands[0].delay_ms = 2.;
    let mut eq = EqState::new(&c);
    eq.peq(&mut c, true, 1, false).unwrap();
    eq.geq(&mut c, GeqMode::Warm);
    let w = Working {
        version: 2,
        selected: "U75".into(),
        active: "U2".into(),
        saved: Config::default(),
        saved_eq: EqState::new(&Config::default()),
        preset: Preset::new("Migrated venue", c, eq),
    };
    let mut old = serde_json::to_value(&w).unwrap();
    old["version"] = 1.into();
    old["preset"]["version"] = 1.into();
    old["saved"] = legacy(w.saved);
    old["preset"]["config"] = legacy(c);
    let source = t.0.join("old-working.json");
    let target = t.0.join("working.json");
    let bytes = serde_json::to_vec(&old).unwrap();
    fs::write(&source, &bytes).unwrap();
    // Legacy working recovery blocks writes and leaves bytes untouched.
    fs::write(&target, &bytes).unwrap();
    let mut lib = Library::open(&t.0).unwrap();
    assert!(lib.recover(&c).is_err());
    assert!(lib.checkpoint(&w).is_err());
    assert_eq!(fs::read(&target).unwrap(), bytes);
    fs::remove_file(&target).unwrap();
    shr_pa::config::migrate(&source, &target).unwrap();
    assert_eq!(fs::read(&source).unwrap(), bytes);
    let recovered = lib.recover(&c).unwrap().unwrap();
    assert_eq!(recovered.preset, w.preset);
    assert_eq!(recovered.saved, w.saved);
    assert_eq!(recovered.saved_eq, w.saved_eq);
    assert_eq!(recovered.selected, w.selected);
    assert_eq!(recovered.active, w.active);
    let mut restored = recovered.preset.config;
    let mut history = recovered.preset.eq;
    history.peq(&mut restored, true, 1, true).unwrap();
    assert_eq!(restored.input_eq[1][3].db, 7.);
    history.geq(&mut restored, GeqMode::Manual);
    assert_eq!(restored.geq.db, [[0.; 31]; 2]);
    let slot_source = t.0.join("old-slot.json");
    fs::write(&slot_source, serde_json::to_vec(&old["preset"]).unwrap()).unwrap();
    shr_pa::config::migrate(&slot_source, t.0.join("U75.json")).unwrap();
    assert_eq!(lib.load("U75").unwrap(), w.preset);
    // Every layout uses exactly the same legacy LR24 engine after migration.
    for (i, layout) in [
        shr_pa::config::Layout::FullRange,
        shr_pa::config::Layout::External,
        shr_pa::config::Layout::TwoWay,
        shr_pa::config::Layout::ThreeWay,
        shr_pa::config::Layout::SixFullRange,
        shr_pa::config::Layout::FourPlusSubs,
    ]
    .into_iter()
    .enumerate()
    {
        let c = Config { layout, ..c };
        let p = t.0.join(format!("v3-{i}.json"));
        fs::write(&source, serde_json::to_vec(&legacy(c)).unwrap()).unwrap();
        shr_pa::config::migrate(&source, &p).unwrap();
        assert_eq!(Config::load(&p).unwrap(), c);
    }
    let bad_target = t.0.join("bad.json");
    for bad in [
        {
            let mut v = old.clone();
            v["mystery"] = true.into();
            v
        },
        {
            let mut v = old.clone();
            v["saved"]["version"] = 99.into();
            v
        },
        {
            let mut v = old.clone();
            v["preset"]["eq"]["input"][1][3]["q"] = 0.into();
            v
        },
        {
            let mut v = old.clone();
            v["preset"]["config"]
                .as_object_mut()
                .unwrap()
                .remove("bands");
            v
        },
        {
            let mut v = legacy(c);
            v["crossover"] = "layout_lr24".into();
            v
        },
        {
            let mut v = legacy(c);
            v["unknown"] = true.into();
            v
        },
    ] {
        let bytes = serde_json::to_vec(&bad).unwrap();
        fs::write(&source, &bytes).unwrap();
        assert!(shr_pa::config::migrate(&source, &bad_target).is_err());
        assert!(!bad_target.exists());
        assert_eq!(fs::read(&source).unwrap(), bytes);
    }
    assert!(shr_pa::config::migrate(&source, &source).is_err());
    let kept = fs::read(&target).unwrap();
    assert!(shr_pa::config::migrate(&source, &target).is_err());
    assert_eq!(fs::read(&target).unwrap(), kept);
}
