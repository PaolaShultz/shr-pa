//! Controller-only preset library and recoverable working state. Never called by render.
use crate::config::{Config, EqBand, GEQ_HZ, Layout};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const USER_SLOTS: usize = 75;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeqMode {
    Manual,
    Flat,
    Speech,
    Warm,
    Gentle,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EqState {
    pub mode: GeqMode,
    pub manual: [[f64; 31]; 2],
    pub input: [Option<[EqBand; 8]>; 2],
    pub output: [Option<[EqBand; 8]>; 3],
}
impl EqState {
    pub fn new(c: &Config) -> Self {
        Self {
            mode: GeqMode::Manual,
            manual: c.geq.db,
            input: [None; 2],
            output: [None; 3],
        }
    }
    pub fn geq(&mut self, c: &mut Config, mode: GeqMode) {
        if self.mode == GeqMode::Manual {
            self.manual = c.geq.db;
        }
        c.geq.db = if mode == GeqMode::Manual {
            self.manual
        } else {
            let curve = GEQ_HZ.map(|hz| match mode {
                GeqMode::Speech => {
                    if hz < 125. {
                        -3.
                    } else if (1000. ..=4000.).contains(&hz) {
                        1.5
                    } else {
                        0.
                    }
                }
                GeqMode::Warm => {
                    if hz <= 160. {
                        1.5
                    } else if hz >= 4000. {
                        -1.5
                    } else {
                        0.
                    }
                }
                GeqMode::Gentle => (-0.5 * (hz / 1000.).log2()).clamp(-2., 2.),
                _ => 0.,
            });
            [curve; 2]
        };
        self.mode = mode;
    }
    pub fn peq(
        &mut self,
        c: &mut Config,
        input: bool,
        index: usize,
        restore: bool,
    ) -> io::Result<()> {
        let (history, eq) = if input {
            (self.input.get_mut(index), c.input_eq.get_mut(index))
        } else {
            (
                self.output.get_mut(index),
                c.bands.get_mut(index).map(|b| &mut b.eq),
            )
        };
        let (Some(history), Some(eq)) = (history, eq) else {
            return Err(io::Error::other("invalid EQ scope"));
        };
        if restore {
            *eq = history
                .take()
                .ok_or_else(|| io::Error::other("no EQ restore point"))?;
        } else {
            if history.is_none() {
                *history = Some(*eq);
            }
            for e in eq {
                e.db = 0.;
            }
        }
        Ok(())
    }
    pub fn validate(&self, c: Config) -> io::Result<()> {
        let mut effective = c;
        let mut history = self.clone();
        // The mode and retained manual settings must agree with the audible snapshot.
        history.mode = GeqMode::Flat;
        history.geq(&mut effective, self.mode);
        if effective.geq.db != c.geq.db {
            return Err(io::Error::other(
                "EQ mode disagrees with processing snapshot",
            ));
        }
        let mut check = c;
        check.geq.db = self.manual;
        for (i, e) in self.input.iter().enumerate() {
            if let Some(e) = e {
                check.input_eq[i] = *e;
            }
        }
        for (i, e) in self.output.iter().enumerate() {
            if let Some(e) = e {
                check.bands[i].eq = *e;
            }
        }
        check.validate().map_err(io::Error::other)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub version: u32,
    pub name: String,
    pub config: Config,
    pub eq: EqState,
}
impl Preset {
    pub fn new(name: &str, config: Config, eq: EqState) -> Self {
        Self {
            version: 2,
            name: name.into(),
            config,
            eq,
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.version != 2 {
            return Err(io::Error::other("unsupported library preset version"));
        }
        if self.name.trim().is_empty()
            || !self.name.is_ascii()
            || self.name.len() > 24
            || self.name.chars().any(char::is_control)
        {
            return Err(io::Error::other("name needs 1..24 printable ASCII chars"));
        }
        self.config.validate().map_err(io::Error::other)?;
        self.eq.validate(self.config)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Working {
    pub version: u32,
    pub selected: String,
    pub active: String,
    pub saved: Config,
    pub saved_eq: EqState,
    pub preset: Preset,
}
impl Working {
    pub(crate) fn validate(&self) -> io::Result<()> {
        if self.version != 2 || self.active.len() > 80 || self.active.chars().any(char::is_control)
        {
            return Err(io::Error::other("unsupported working state"));
        }
        slot(&self.selected)?;
        if self.active != "standalone" {
            slot(&self.active)?;
        }
        if self.saved.sample_rate != self.preset.config.sample_rate
            || self.saved.max_block != self.preset.config.max_block
        {
            return Err(io::Error::other("working baseline rate/block differs"));
        }
        self.saved.validate().map_err(io::Error::other)?;
        self.saved_eq.validate(self.saved)?;
        self.preset.validate()
    }
}
pub fn slot(id: &str) -> io::Result<(bool, usize)> {
    let n = id
        .get(1..)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let template = id.starts_with('T');
    if !(template || id.starts_with('U')) || n == 0 || n > if template { 6 } else { USER_SLOTS } {
        return Err(io::Error::other("select U1..U75 or T1..T6"));
    }
    if id != format!("{}{n}", if template { 'T' } else { 'U' }) {
        return Err(io::Error::other("use canonical slot IDs, e.g. U1"));
    }
    Ok((template, n))
}
pub struct Library {
    root: PathBuf,
    _lock: fs::File,
    pub recovery_blocked: bool,
}
impl Library {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        fs::create_dir_all(&root)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.as_ref().join("lock"))?;
        lock.try_lock()
            .map_err(|_| io::Error::other("library already open in another editor"))?;
        Ok(Self {
            root: root.as_ref().into(),
            _lock: lock,
            recovery_blocked: false,
        })
    }
    pub fn load(&self, id: &str) -> io::Result<Preset> {
        let (template, n) = slot(id)?;
        if template {
            let c = Config {
                layout: [
                    Layout::FullRange,
                    Layout::External,
                    Layout::TwoWay,
                    Layout::ThreeWay,
                    Layout::SixFullRange,
                    Layout::FourPlusSubs,
                ][n - 1],
                ..Config::default()
            };
            return Ok(Preset::new(&format!("{:?}", c.layout), c, EqState::new(&c)));
        }
        let p: Preset = read(&self.root.join(format!("U{n}.json")))?;
        p.validate()?;
        Ok(p)
    }
    pub fn save(&self, id: &str, p: &Preset, overwrite: bool) -> io::Result<()> {
        let (template, n) = slot(id)?;
        if template {
            return Err(io::Error::other(
                "templates are immutable; select a user slot",
            ));
        }
        p.validate()?;
        let path = self.root.join(format!("U{n}.json"));
        if path.exists() && !overwrite {
            return Err(io::Error::other("slot occupied; use save! or copy!"));
        }
        atomic(&path, p)
    }
    pub fn recover(&mut self, base: &Config) -> io::Result<Option<Working>> {
        let path = self.root.join("working.json");
        let result = (|| {
            if !path.exists() {
                return Ok(None);
            }
            let w: Working = read(&path)?;
            w.validate()?;
            if w.preset.config.sample_rate != base.sample_rate
                || w.preset.config.max_block != base.max_block
            {
                return Err(io::Error::other("recovery rate/block incompatible"));
            }
            Ok(Some(w))
        })();
        self.recovery_blocked = result.is_err();
        result
    }
    pub fn checkpoint(&self, w: &Working) -> io::Result<()> {
        if self.recovery_blocked {
            return Err(io::Error::other(
                "recovery blocked; :recover-reset archives state",
            ));
        }
        w.validate()?;
        atomic(&self.root.join("working.json"), w)
    }
    pub fn reset_recovery(&mut self) -> io::Result<()> {
        let path = self.root.join("working.json");
        if path.exists() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos();
            fs::rename(
                path,
                self.root.join(format!("working-rejected-{stamp}.json")),
            )?;
            fs::File::open(&self.root)?.sync_all()?;
        }
        self.recovery_blocked = false;
        Ok(())
    }
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > 262144 {
        return Err(io::Error::other("state exceeds 256 KiB"));
    }
    let v: serde_json::Value = serde_json::from_reader(file)?;
    if v["version"].as_u64() != Some(2) {
        return Err(io::Error::other(
            "unsupported state version; v1 requires migrate OLD NEW",
        ));
    }
    Ok(serde_json::from_value(v)?)
}
pub fn atomic(path: &Path, value: &impl Serialize) -> io::Result<()> {
    atomic_write(path, value, false)
}
pub(crate) fn atomic_new(path: &Path, value: &impl Serialize) -> io::Result<()> {
    atomic_write(path, value, true)
}
fn atomic_write(path: &Path, value: &impl Serialize, create_new: bool) -> io::Result<()> {
    use io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = parent.join(format!(
        ".shr-pa-state-{}-{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        f.write_all(&serde_json::to_vec_pretty(value)?)?;
        f.sync_all()?;
        if create_new {
            // Atomic no-clobber publication, including dangling symlinks or a
            // destination created since migration validation began.
            fs::hard_link(&temp, path)?;
            fs::remove_file(&temp)?;
        } else {
            fs::rename(&temp, path)?;
        }
        fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

/// Migrate a strict v1 slot or working envelope, including both processing
/// baselines. EQ history is carried verbatim and validated against its snapshot.
pub(crate) fn migrate_value(mut v: serde_json::Value) -> io::Result<serde_json::Value> {
    if v["version"].as_u64() != Some(1) {
        return Err(io::Error::other(
            "migration requires v1 library/working envelope",
        ));
    }
    if (v.get("preset").is_some() && v["saved"]["version"].as_u64() != Some(2))
        || (v.get("preset").is_none() && v["config"]["version"].as_u64() != Some(2))
    {
        return Err(io::Error::other(
            "v1 library/working envelopes require v2 processing",
        ));
    }
    v["version"] = 2.into();
    if v.get("preset").is_some() {
        v["saved"] = serde_json::to_value(crate::config::migrate_value(v["saved"].take())?)?;
        v["preset"] = migrate_value(v["preset"].take())?;
        let w: Working = serde_json::from_value(v)?;
        w.validate()?;
        Ok(serde_json::to_value(w)?)
    } else {
        v["config"] = serde_json::to_value(crate::config::migrate_value(v["config"].take())?)?;
        let p: Preset = serde_json::from_value(v)?;
        p.validate()?;
        Ok(serde_json::to_value(p)?)
    }
}
