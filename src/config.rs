//! Validated, versioned processing presets. No hardware identity is persisted.
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    FullRange,
    External,
    TwoWay,
    ThreeWay,
    SixFullRange,
    FourPlusSubs,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Stereo,
    MonoLeft,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EqKind {
    Bell,
    LowShelf,
    HighShelf,
}
pub const GEQ_HZ: [f64; 31] = [
    20., 25., 31.5, 40., 50., 63., 80., 100., 125., 160., 200., 250., 315., 400., 500., 630., 800.,
    1000., 1250., 1600., 2000., 2500., 3150., 4000., 5000., 6300., 8000., 10000., 12500., 16000.,
    20000.,
];
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphicEq {
    pub enabled: bool,
    pub linked: bool,
    pub db: [[f64; 31]; 2],
}
impl Default for GraphicEq {
    fn default() -> Self {
        Self {
            enabled: false,
            linked: true,
            db: [[0.; 31]; 2],
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Compressor {
    pub enabled: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
}
impl Default for Compressor {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.,
            ratio: 4.,
            knee_db: 6.,
            makeup_db: 0.,
            attack_ms: 10.,
            release_ms: 100.,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EqBand {
    pub kind: EqKind,
    /// RBJ shelf slope S (not Q or dB/octave); used only for shelves.
    pub slope: f64,
    pub hz: f64,
    pub q: f64,
    pub db: f64,
}
impl Default for EqBand {
    fn default() -> Self {
        Self {
            kind: EqKind::Bell,
            slope: 1.,
            hz: 1000.,
            q: 0.707,
            db: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub eq_enabled: bool,
    pub gain_db: f64,
    pub inverted: bool,
    pub delay_ms: f64,
    pub eq: [EqBand; 8],
    pub limiter_db: f64,
    pub release_ms: f64,
}
impl Default for Band {
    fn default() -> Self {
        Self {
            eq_enabled: true,
            gain_db: 0.,
            inverted: false,
            delay_ms: 0.,
            eq: [EqBand::default(); 8],
            limiter_db: -1.,
            release_ms: 100.,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub sample_rate: u32,
    pub max_block: usize,
    pub layout: Layout,
    pub input_mode: InputMode,
    pub mono_bass: bool,
    pub low_hz: f64,
    pub high_hz: f64,
    pub input_gain_db: f64,
    pub input_delay_ms: f64,
    pub geq: GraphicEq,
    pub compressor: Compressor,
    pub input_eq_enabled: bool,
    pub input_eq: [[EqBand; 8]; 2],
    /// High, mid, low pairs. State and runtime mutes are per logical output.
    pub bands: [Band; 3],
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            geq: GraphicEq::default(),
            compressor: Compressor::default(),
            input_eq_enabled: true,
            sample_rate: 48000,
            max_block: 128,
            layout: Layout::ThreeWay,
            input_mode: InputMode::Stereo,
            mono_bass: false,
            low_hz: 120.,
            high_hz: 1800.,
            input_gain_db: 0.,
            input_delay_ms: 0.,
            input_eq: [[EqBand::default(); 8]; 2],
            bands: std::array::from_fn(|_| Band::default()),
        }
    }
}
fn range(x: f64, lo: f64, hi: f64) -> bool {
    x.is_finite() && (lo..=hi).contains(&x)
}
impl Config {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 2 {
            return Err("unsupported preset version; v1 requires explicit migrate OLD NEW");
        }
        if !(8000..=192000).contains(&self.sample_rate) || !(1..=8192).contains(&self.max_block) {
            return Err("rate must be 8000..192000 Hz; block 1..8192 frames");
        }
        let top = (self.sample_rate as f64 * 0.45).min(20000.);
        if !range(self.low_hz, 16., top)
            || !range(self.high_hz, 16., top)
            || self.low_hz >= self.high_hz
        {
            return Err("crossover requires 16 <= low < high <= min(20000, 0.45 * rate)");
        }
        if !range(self.input_gain_db, -60., 20.) || !range(self.input_delay_ms, 0., 100.) {
            return Err("invalid input gain or delay");
        }
        if self
            .geq
            .db
            .iter()
            .flatten()
            .any(|&db| !range(db, -12., 12.))
        {
            return Err("invalid GEQ gain");
        }
        let d = self.compressor;
        if !range(d.threshold_db, -60., 0.)
            || !range(d.ratio, 1., 100.)
            || !range(d.knee_db, 0., 24.)
            || !range(d.makeup_db, -20., 20.)
            || !range(d.attack_ms, 0.1, 200.)
            || !range(d.release_ms, 1., 2000.)
        {
            return Err("invalid compressor parameters");
        }
        for e in self
            .input_eq
            .iter()
            .flatten()
            .chain(self.bands.iter().flat_map(|b| b.eq.iter()))
        {
            if !range(e.slope, 0.1, 1.)
                || !range(e.hz, 20., top)
                || !range(e.q, 0.1, 15.909)
                || !range(e.db, -12., 12.)
            {
                return Err("invalid PEQ frequency, Q or gain");
            }
        }
        for b in &self.bands {
            if !range(b.gain_db, -60., 20.)
                || !range(b.delay_ms, 0., 10.)
                || !range(b.limiter_db, -60., 0.)
                || !range(b.release_ms, 1., 2000.)
            {
                return Err("invalid band gain, delay or limiter");
            }
        }
        Ok(())
    }
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > 65536 {
            return Err(io::Error::other("preset exceeds 64 KiB"));
        }
        let value: serde_json::Value = serde_json::from_reader(file)?;
        if value.get("version").and_then(|v| v.as_u64()) != Some(2) {
            return Err(io::Error::other(
                "unsupported preset version; v1 requires explicit migrate OLD NEW",
            ));
        }
        let c: Self = serde_json::from_value(value)?;
        c.validate().map_err(io::Error::other)?;
        Ok(c)
    }
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        use std::io::Write;
        self.validate().map_err(io::Error::other)?;
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp = parent.join(format!(".shr-pa-{}-{serial}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| {
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            fs::rename(&temp, path)?;
            fs::File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

/// Explicit v1 -> v2 conversion: preserve bell meanings, add bypassed new modules.
/// Unknown legacy fields are rejected before any fields are added.
pub fn migrate_v1(source: impl AsRef<Path>, target: impl AsRef<Path>) -> io::Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();
    if source == target
        || (target.exists() && fs::canonicalize(source)? == fs::canonicalize(target)?)
    {
        return Err(io::Error::other(
            "migration requires a separate destination",
        ));
    }
    let file = fs::File::open(source)?;
    if file.metadata()?.len() > 65536 {
        return Err(io::Error::other("preset exceeds 64 KiB"));
    }
    let mut v: serde_json::Value = serde_json::from_reader(file)?;
    fn keys(v: &serde_json::Value, allowed: &[&str]) -> io::Result<()> {
        let o = v
            .as_object()
            .ok_or_else(|| io::Error::other("expected legacy object"))?;
        if o.len() != allowed.len() || o.keys().any(|k| !allowed.contains(&k.as_str())) {
            return Err(io::Error::other("invalid or unknown v1 fields"));
        }
        Ok(())
    }
    keys(
        &v,
        &[
            "version",
            "sample_rate",
            "max_block",
            "layout",
            "input_mode",
            "mono_bass",
            "low_hz",
            "high_hz",
            "input_gain_db",
            "input_delay_ms",
            "input_eq",
            "bands",
        ],
    )?;
    if v["version"].as_u64() != Some(1) {
        return Err(io::Error::other("migration source must be v1"));
    }
    fn eqs(v: &mut serde_json::Value) -> io::Result<()> {
        for e in v
            .as_array_mut()
            .ok_or_else(|| io::Error::other("expected EQ array"))?
        {
            keys(e, &["hz", "q", "db"])?;
            e["kind"] = "bell".into();
            e["slope"] = 1.0.into();
        }
        Ok(())
    }
    for es in v["input_eq"]
        .as_array_mut()
        .ok_or_else(|| io::Error::other("expected input EQ"))?
    {
        eqs(es)?;
    }
    for b in v["bands"]
        .as_array_mut()
        .ok_or_else(|| io::Error::other("expected bands"))?
    {
        keys(
            b,
            &[
                "gain_db",
                "inverted",
                "delay_ms",
                "eq",
                "limiter_db",
                "release_ms",
            ],
        )?;
        eqs(&mut b["eq"])?;
        b["eq_enabled"] = true.into();
    }
    v["version"] = 2.into();
    v["input_eq_enabled"] = true.into();
    v["geq"] = serde_json::to_value(GraphicEq::default())?;
    v["compressor"] = serde_json::to_value(Compressor::default())?;
    let c: Config = serde_json::from_value(v)?;
    c.save(target)
}
