//! Shared terminal layout for interactive display and reproducible snapshots.
pub const WIDTH: u16 = 40;
pub const HEIGHT: u16 = 13;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Features,
    Hardware,
}

impl Page {
    pub fn next(self) -> Self {
        match self {
            Self::Home => Self::Features,
            Self::Features => Self::Hardware,
            Self::Hardware => Self::Home,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Home => Self::Hardware,
            Self::Features => Self::Home,
            Self::Hardware => Self::Features,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "home" => Some(Self::Home),
            "features" => Some(Self::Features),
            "hardware" => Some(Self::Hardware),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Previous,
    Next,
    Exit,
}

/// Footer actions occupy entire labelled regions; other cells do nothing.
pub fn hit_test(column: u16, row: u16, width: u16, height: u16) -> Option<Action> {
    if width < WIDTH || height < HEIGHT || row != HEIGHT - 1 {
        return None;
    }
    match column {
        0..=12 => Some(Action::Previous),
        13..=25 => Some(Action::Next),
        26..=39 => Some(Action::Exit),
        _ => None,
    }
}

pub fn screen(page: Page, width: u16, height: u16) -> Vec<String> {
    if width < WIDTH || height < HEIGHT {
        return ["SHR PA: resize to 40x13", "q / Esc: exit"]
            .into_iter()
            .take(usize::from(height))
            .map(|s| s.chars().take(usize::from(width)).collect())
            .collect();
    }
    let body = match page {
        Page::Home => [
            "01 / ENGINE - OFFLINE EDITOR",
            "PA management for Raspberry Pi 5",
            "",
            "First target: DriveRack PA2 / 2 x 6",
            "Rust / Linux Lite / terminal UI",
            "",
            "DSP works offline; audio is not open.",
            "LR24 / PEQ / delays / limiter ready.",
            "2 program inputs / 6 logical outputs.",
        ],
        Page::Features => [
            "02 / FEATURES",
            "Working: LR24 / PEQ / delay / limiter",
            "Working: WAV / presets / ALSA / mutes",
            "Planned: RTA / AutoEQ / feedback",
            "",
            "Setup / presets / meters / controls",
            "Separate setup mic / remote plan",
            "",
            "GEQ / shelves / compressor ready.",
        ],
        Page::Hardware => [
            "03 / TARGET HARDWARE",
            "Raspberry Pi 5 / 64-bit Linux Lite",
            "Capability-based direct ALSA backend",
            "Small touchscreen / terminal",
            "Optional controllers, including MIDI",
            "",
            "UMC1820 is a future target.",
            "No measured latency claim yet.",
            "See docs/HARDWARE.md for evidence.",
        ],
    };
    let mut lines = vec![
        "SHR PA        HIGHLY EXPERIMENTAL".to_owned(),
        "----------------------------------------".to_owned(),
    ];
    lines.extend(body.map(str::to_owned));
    lines.push("----------------------------------------".to_owned());
    lines.push(format!(
        "{:<13}{:<13}{:<14}",
        "[ PREV ]", "[ NEXT ]", "[ EXIT ]"
    ));
    lines
}

/// Offline editor. Edits rebuild only while stopped; preview uses the real engine.
pub struct Editor {
    pub config: crate::config::Config,
    mutes: [bool; 6],
    selected: usize,
    message: String,
    peaks: [f32; 6],
    saved: crate::config::Config,
    path: std::path::PathBuf,
    module: usize,
    field: usize,
    index: usize,
    channel: usize,
    recall_requested: bool,
}
impl Editor {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            config: crate::config::Config::default(),
            mutes: [true; 6],
            selected: 0,
            message: "Offline: no device open".into(),
            peaks: [0.; 6],
            saved: crate::config::Config::default(),
            path: "preset.json".into(),
            module: 0,
            field: 0,
            index: 0,
            channel: 0,
            recall_requested: false,
        })
    }
    pub fn key(&mut self, key: char) {
        use crate::config::Layout;
        let mut next = self.config;
        match key {
            'v' => {
                self.module = (self.module + 1) % 7;
                self.field = 0;
                return;
            }
            'n' => {
                self.field = (self.field + 1) % self.field_count();
                return;
            }
            'N' => {
                self.field = (self.field + self.field_count() - 1) % self.field_count();
                return;
            }
            'j' => {
                self.index = (self.index + 1) % if self.module == 0 { 31 } else { 8 };
                return;
            }
            'k' => {
                let count = if self.module == 0 { 31 } else { 8 };
                self.index = (self.index + count - 1) % count;
                return;
            }
            'h' => {
                self.channel = 1 - self.channel;
                return;
            }
            'x' | 'X' => {
                self.adjust(&mut next, if key == 'x' { 1. } else { -1. });
            }
            _ => {}
        }
        match key {
            'x' | 'X' => {}
            '1'..='6' => {
                let ch = key as usize - '1' as usize;
                self.mutes[ch] = !self.mutes[ch];
            }
            'm' => self.mutes = [true; 6],
            'u' => self.mutes = [false; 6],
            'g' => next.input_gain_db = (next.input_gain_db + 1.).min(20.),
            'G' => next.input_gain_db = (next.input_gain_db - 1.).max(-60.),
            'a' => next.input_delay_ms = (next.input_delay_ms + 1.).min(100.),
            'A' => next.input_delay_ms = (next.input_delay_ms - 1.).max(0.),
            '[' => next.low_hz = (next.low_hz / 1.1).max(16.),
            ']' => next.low_hz *= 1.1,
            '{' => next.high_hz = (next.high_hz / 1.1).max(16.),
            '}' => next.high_hz *= 1.1,
            'b' => self.selected = (self.selected + 1) % 3,
            '+' | '=' => {
                next.bands[self.selected].gain_db =
                    (next.bands[self.selected].gain_db + 1.).min(20.)
            }
            '-' => {
                next.bands[self.selected].gain_db =
                    (next.bands[self.selected].gain_db - 1.).max(-60.)
            }
            'p' => next.bands[self.selected].inverted = !next.bands[self.selected].inverted,
            'd' => {
                next.bands[self.selected].delay_ms =
                    (next.bands[self.selected].delay_ms + 0.1).min(10.)
            }
            'D' => {
                next.bands[self.selected].delay_ms =
                    (next.bands[self.selected].delay_ms - 0.1).max(0.)
            }
            'e' => {
                next.bands[self.selected].eq[0].db =
                    (next.bands[self.selected].eq[0].db + 1.).min(12.)
            }
            'E' => {
                next.bands[self.selected].eq[0].db =
                    (next.bands[self.selected].eq[0].db - 1.).max(-12.)
            }
            't' => {
                next.bands[self.selected].limiter_db =
                    (next.bands[self.selected].limiter_db - 1.).max(-60.)
            }
            'T' => {
                next.bands[self.selected].limiter_db =
                    (next.bands[self.selected].limiter_db + 1.).min(0.)
            }
            'c' => {
                next.layout = match next.layout {
                    Layout::FullRange => Layout::External,
                    Layout::External => Layout::TwoWay,
                    Layout::TwoWay => Layout::ThreeWay,
                    Layout::ThreeWay => Layout::SixFullRange,
                    Layout::SixFullRange => Layout::FourPlusSubs,
                    Layout::FourPlusSubs => Layout::FullRange,
                }
            }
            'i' => {
                next.input_mode = if next.input_mode == crate::config::InputMode::Stereo {
                    crate::config::InputMode::MonoLeft
                } else {
                    crate::config::InputMode::Stereo
                }
            }
            'o' => next.mono_bass = !next.mono_bass,
            's' => {
                self.message = match self.config.save(&self.path) {
                    Ok(()) => {
                        self.saved = self.config;
                        "Saved; runtime mutes excluded".into()
                    }
                    Err(e) => e.to_string(),
                };
                return;
            }
            'l' => match crate::config::Config::load(&self.path) {
                Ok(c) => {
                    next = c;
                    self.saved = c;
                    self.recall_requested = true;
                    self.mutes = [true; 6];
                    self.message = "Loaded preset; muted".into();
                }
                Err(e) => {
                    self.message = e.to_string();
                    return;
                }
            },
            'r' => {
                let mut e = crate::dsp::Engine::new(self.config).expect("validated editor");
                e.set_mutes(self.mutes);
                let mut g = crate::offline::Generator::new(
                    crate::offline::Signal::Sine(1000.),
                    self.config.sample_rate,
                    self.config.sample_rate as u64,
                )
                .unwrap();
                let mut input = vec![[0.; 2]; self.config.max_block];
                let mut output = vec![[0.; 6]; self.config.max_block];
                self.peaks = [0.; 6];
                for _ in 0..(self.config.sample_rate as usize / self.config.max_block + 1) {
                    g.fill(&mut input);
                    e.render(&input, &mut output).unwrap();
                    for (a, b) in self.peaks.iter_mut().zip(e.meters.output_peak) {
                        *a = a.max(b);
                    }
                }
                self.message = "Rendered offline 1kHz / -20dBFS".into();
                return;
            }
            _ => return,
        }
        match next.validate() {
            Ok(()) => {
                self.config = next;
                self.peaks = [0.; 6];
                if key != 'l' {
                    self.message = "Edited; r to refresh offline preview".into();
                }
            }
            Err(e) => self.message = e.into(),
        }
    }
    pub fn screen(&self, page: Page, width: u16, height: u16) -> Vec<String> {
        let mut lines = screen(page, width, height);
        if width < WIDTH || height < HEIGHT {
            return lines;
        }
        if page == Page::Home {
            let b = &self.config.bands[self.selected];
            let rows = [
                format!(
                    "01 / ENGINE - OFFLINE EDITOR {}",
                    if self.config != self.saved { "*" } else { "" }
                ),
                format!("c:{:?} i:{:?}", self.config.layout, self.config.input_mode),
                format!(
                    "b:pair {} +/-:{:.1}dB p:inv {}",
                    self.selected + 1,
                    b.gain_db,
                    b.inverted
                ),
                format!("d/D:{:.1}ms e/E:EQ {:+.0}dB", b.delay_ms, b.eq[0].db),
                format!(
                    "t/T:limit {:.0}dB o:mono bass {}",
                    b.limiter_db, self.config.mono_bass
                ),
                format!(
                    "1..6 mute: {}",
                    self.mutes
                        .map(|m| if m { 'M' } else { '-' })
                        .iter()
                        .collect::<String>()
                ),
                "m:mute u:unmute r:preview s:save l:load".into(),
                format!(
                    "Peak {:.3} {:.3} {:.3}",
                    self.peaks[0], self.peaks[2], self.peaks[4]
                ),
                self.message.clone(),
            ];
            for (line, text) in lines[2..11].iter_mut().zip(rows) {
                *line = text.chars().take(WIDTH as usize).collect();
            }
        }
        if page == Page::Features {
            let rows = self.module_rows();
            for (line, text) in lines[2..11].iter_mut().zip(rows) {
                *line = text.chars().take(WIDTH as usize).collect();
            }
        }
        lines
    }
}

impl Editor {
    fn field_count(&self) -> usize {
        [3, 6, 6, 7, 2, 6, 5][self.module]
    }
    fn adjust(&self, c: &mut crate::config::Config, d: f64) {
        use crate::config::{EqKind, InputMode, Layout};
        let pair = self.selected;
        let f = self.field;
        match self.module {
            0 => match f {
                0 => c.geq.enabled = !c.geq.enabled,
                1 => c.geq.linked = !c.geq.linked,
                _ => {
                    let ch = if c.geq.linked { 0 } else { self.channel };
                    c.geq.db[ch][self.index] =
                        (c.geq.db[ch][self.index] + d * 0.1).clamp(-12., 12.);
                }
            },
            1 | 2 => {
                if f == 0 {
                    if self.module == 1 {
                        c.input_eq_enabled = !c.input_eq_enabled;
                    } else {
                        c.bands[pair].eq_enabled = !c.bands[pair].eq_enabled;
                    }
                    return;
                }
                let e = if self.module == 1 {
                    &mut c.input_eq[self.channel][self.index % 8]
                } else {
                    &mut c.bands[pair].eq[self.index % 8]
                };
                match f {
                    1 => {
                        e.kind = match (e.kind, d > 0.) {
                            (EqKind::Bell, true) | (EqKind::HighShelf, false) => EqKind::LowShelf,
                            (EqKind::LowShelf, true) | (EqKind::Bell, false) => EqKind::HighShelf,
                            _ => EqKind::Bell,
                        }
                    }
                    2 => {
                        e.hz = (e.hz * 2_f64.powf(d / 12.))
                            .clamp(20., (c.sample_rate as f64 * 0.45).min(20000.))
                    }
                    3 => e.db = (e.db + d * 0.1).clamp(-12., 12.),
                    4 => e.q = (e.q * 2_f64.powf(d / 12.)).clamp(0.1, 15.909),
                    _ => e.slope = (e.slope + d * 0.05).clamp(0.1, 1.),
                }
            }
            3 => {
                let c = &mut c.compressor;
                match f {
                    0 => c.enabled = !c.enabled,
                    1 => c.threshold_db = (c.threshold_db + d).clamp(-60., 0.),
                    2 => c.ratio = (c.ratio + d * 0.5).clamp(1., 100.),
                    3 => c.knee_db = (c.knee_db + d).clamp(0., 24.),
                    4 => c.makeup_db = (c.makeup_db + d * 0.5).clamp(-20., 20.),
                    5 => c.attack_ms = (c.attack_ms * 2_f64.powf(d / 6.)).clamp(0.1, 200.),
                    6 => c.release_ms = (c.release_ms * 2_f64.powf(d / 6.)).clamp(1., 2000.),
                    _ => {
                        c.enabled = !c.enabled;
                    }
                }
            }
            4 => {
                let b = &mut c.bands[pair];
                if f == 0 {
                    b.limiter_db = (b.limiter_db + d).clamp(-60., 0.);
                } else {
                    b.release_ms = (b.release_ms * 2_f64.powf(d / 6.)).clamp(1., 2000.);
                }
            }
            5 => match f {
                0 => c.input_gain_db = (c.input_gain_db + d).clamp(-60., 20.),
                1 => c.input_delay_ms = (c.input_delay_ms + d).clamp(0., 100.),
                2 => c.bands[pair].gain_db = (c.bands[pair].gain_db + d).clamp(-60., 20.),
                3 => c.bands[pair].inverted = !c.bands[pair].inverted,
                4 => c.bands[pair].delay_ms = (c.bands[pair].delay_ms + d * 0.1).clamp(0., 10.),
                _ => c.mono_bass = !c.mono_bass,
            },
            _ => match f {
                0 => {
                    let layouts = [
                        Layout::FullRange,
                        Layout::External,
                        Layout::TwoWay,
                        Layout::ThreeWay,
                        Layout::SixFullRange,
                        Layout::FourPlusSubs,
                    ];
                    let i = layouts.iter().position(|&l| l == c.layout).unwrap();
                    c.layout = layouts[(i + if d > 0. { 1 } else { 5 }) % 6];
                }
                1 => {
                    c.input_mode = if c.input_mode == InputMode::Stereo {
                        InputMode::MonoLeft
                    } else {
                        InputMode::Stereo
                    }
                }
                2 => c.low_hz = (c.low_hz * 1.1_f64.powf(d)).max(16.),
                3 => {
                    c.high_hz =
                        (c.high_hz * 1.1_f64.powf(d)).min((c.sample_rate as f64 * 0.45).min(20000.))
                }
                _ => c.mono_bass = !c.mono_bass,
            },
        }
    }
    fn module_rows(&self) -> Vec<String> {
        let c = &self.config;
        let b = &c.bands[self.selected];
        let fields: Vec<String> = match self.module {
            0 => vec![
                format!("Enabled {}", c.geq.enabled),
                format!("Linked {} (L owns linked)", c.geq.linked),
                format!(
                    "Gain {:+.1} dB",
                    c.geq.db[if c.geq.linked { 0 } else { self.channel }][self.index]
                ),
            ],
            1 | 2 => {
                let e = if self.module == 1 {
                    c.input_eq[self.channel][self.index % 8]
                } else {
                    b.eq[self.index % 8]
                };
                vec![
                    format!(
                        "Enabled {}",
                        if self.module == 1 {
                            c.input_eq_enabled
                        } else {
                            b.eq_enabled
                        }
                    ),
                    format!("Type {:?}", e.kind),
                    format!("Frequency {:.1} Hz", e.hz),
                    format!("Gain {:+.1} dB", e.db),
                    format!("Bell Q {:.3}", e.q),
                    format!("Shelf slope S {:.2}", e.slope),
                ]
            }
            3 => {
                let d = c.compressor;
                vec![
                    format!("Enabled {}", d.enabled),
                    format!("Threshold {:.1} dBFS", d.threshold_db),
                    format!("Ratio {:.1}:1", d.ratio),
                    format!("Knee {:.1} dB", d.knee_db),
                    format!("Makeup {:+.1} dB", d.makeup_db),
                    format!("Attack {:.2} ms", d.attack_ms),
                    format!("Release {:.2} ms", d.release_ms),
                ]
            }
            4 => vec![
                format!("Ceiling {:.1} dBFS", b.limiter_db),
                format!("Release {:.2} ms", b.release_ms),
            ],
            5 => vec![
                format!("Input gain {:+.1} dB", c.input_gain_db),
                format!("Input delay {:.1} ms", c.input_delay_ms),
                format!("Pair gain {:+.1} dB", b.gain_db),
                format!("Pair inverted {}", b.inverted),
                format!("Pair delay {:.1} ms", b.delay_ms),
                format!("Mono bass {}", c.mono_bass),
            ],
            _ => vec![
                format!("Layout {:?}", c.layout),
                format!("Input {:?}", c.input_mode),
                format!("Low split {:.1} Hz", c.low_hz),
                format!("High split {:.1} Hz", c.high_hz),
                format!("Mono bass {}", c.mono_bass),
            ],
        };
        vec![
            format!(
                "v:{} {}",
                [
                    "GEQ",
                    "INPUT PEQ",
                    "PAIR PEQ",
                    "COMPRESSOR",
                    "LIMITER",
                    "GAIN/DELAY",
                    "CROSSOVER"
                ][self.module],
                if self.config != self.saved {
                    "*modified"
                } else {
                    "saved"
                }
            ),
            format!(
                "b:pair {} h:input {} j/k:band {}",
                ["H", "M", "L"][self.selected],
                self.channel + 1,
                if self.module == 0 {
                    self.index + 1
                } else {
                    self.index % 8 + 1
                }
            ),
            if self.module == 0 {
                format!("GEQ {:.1} Hz", crate::config::GEQ_HZ[self.index])
            } else {
                format!("Layout {:?}", c.layout)
            },
            format!("n/N field {}/{}", self.field + 1, fields.len()),
            format!("> {}", fields[self.field]),
            format!("  {}", fields[(self.field + 1) % fields.len()]),
            "x/X +/- or toggle; v next module".into(),
            "s:save l:recall  m:mute u:unmute".into(),
            self.message.clone(),
        ]
    }
}

pub fn live(
    config: crate::config::Config,
    preset_path: &str,
    options: crate::transport::LiveOptions<'_>,
    shared: std::sync::Arc<crate::transport::Shared>,
) -> crate::offline::Result<crate::transport::LiveReport> {
    use crossterm::{
        cursor::{Hide, MoveTo, Show},
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::{
        io::{self, IsTerminal, Write},
        sync::atomic::Ordering,
        time::Duration,
    };
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("--ui requires a terminal".into());
    }
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
            let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        }
    }
    terminal::enable_raw_mode()?;
    let _guard = Guard;
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    let physical_outputs = options.map.outputs;
    let mut editor = Editor::new()?;
    editor.config = config;
    editor.saved = config;
    editor.path = preset_path.into();
    let mut sent = config;
    let mut modules = false;
    let mut queued_recall = false;

    std::thread::scope(|scope| {
        let s = shared.clone();
        let worker = scope.spawn(move || {
            let result = crate::transport::run(config, options, &s);
            s.stop.store(true, Ordering::Relaxed);
            result
        });
        let mut last_paint = std::time::Instant::now() - Duration::from_millis(100);
        let ui_result: io::Result<()> = (|| {
            while !shared.stop.load(Ordering::Relaxed) {
                let actual = shared.actual_block.load(Ordering::Acquire) as usize;
                if actual > 0
                    && (editor.config != sent || queued_recall)
                    && !shared.controls.pending()
                    && !shared.fault.load(Ordering::Relaxed)
                {
                    let mut c = editor.config;
                    c.max_block = actual;
                    match crate::dsp::Prepared::new(c, queued_recall)
                        .and_then(|p| shared.controls.publish(p))
                    {
                        Ok(()) => {
                            sent = editor.config;
                            queued_recall = false;
                        }
                        Err(e) => editor.message = e.into(),
                    }
                }
                if last_paint.elapsed() >= Duration::from_millis(100) {
                    last_paint = std::time::Instant::now();
                    let (w, h) = terminal::size()?;
                    let mut out = io::stdout().lock();
                    execute!(out, MoveTo(0, 0), Clear(ClearType::All))?;
                    let rows = if w < WIDTH || h < HEIGHT {
                        vec!["SHR PA: resize to 40x13".into(), "q: stop".into()]
                    } else if modules {
                        let mut rows = vec![format!(
                            "LIVE {} Tab:meters",
                            editor
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        )];
                        rows.extend(editor.module_rows());
                        rows.push(format!(
                            "GR comp {:.1} limit {:.1}/{:.1}/{:.1}",
                            f32::from_bits(shared.reduction[3].load(Ordering::Relaxed)),
                            f32::from_bits(shared.reduction[0].load(Ordering::Relaxed)),
                            f32::from_bits(shared.reduction[1].load(Ordering::Relaxed)),
                            f32::from_bits(shared.reduction[2].load(Ordering::Relaxed))
                        ));
                        rows.push(format!(
                            "Fault {} pending {} busy {}",
                            shared.fault.load(Ordering::Relaxed),
                            editor.config != sent || queued_recall || shared.controls.pending(),
                            shared.transitioning.load(Ordering::Relaxed)
                        ));
                        rows.push("Tab:meters q:stop".into());
                        rows
                    } else {
                        let mask = shared.mutes.load(Ordering::Relaxed);
                        let mut rows = vec![
                            format!(
                                "SHR PA / LIVE 2 x 6 {}",
                                if editor.config != editor.saved {
                                    "*"
                                } else {
                                    ""
                                }
                            ),
                            "1..6: mute toggle  m:all mute  u:unmute".into(),
                        ];
                        rows.push(format!(
                            "Input {:.3} {:.3}",
                            f32::from_bits(shared.input[0].load(Ordering::Relaxed)),
                            f32::from_bits(shared.input[1].load(Ordering::Relaxed))
                        ));
                        for (ch, physical) in physical_outputs.iter().enumerate() {
                            rows.push(format!(
                                "{} -> {} {} peak {:.4}",
                                crate::dsp::OUTPUT_NAMES[ch],
                                physical.map_or("-".into(), |p| p.to_string()),
                                if mask & (1 << ch) != 0 {
                                    "MUTE"
                                } else {
                                    "OPEN"
                                },
                                f32::from_bits(shared.output[ch].load(Ordering::Relaxed))
                            ));
                        }
                        rows.push(format!(
                            "Blocks {}  q:stop",
                            shared.blocks.load(Ordering::Relaxed)
                        ));
                        rows.push(format!(
                            "Tab:controls pair {} clips {}",
                            editor.selected + 1,
                            shared.clips.load(Ordering::Relaxed)
                        ));
                        rows.push(format!(
                            "Txn {} rejected {} busy {}",
                            shared.controls.accepted.load(Ordering::Acquire),
                            shared.controls.rejected.load(Ordering::Acquire),
                            shared.transitioning.load(Ordering::Relaxed)
                        ));
                        rows.push(format!(
                            "Fault {} {}",
                            shared.fault.load(Ordering::Relaxed),
                            editor.message
                        ));
                        rows
                    };
                    for (row, line) in rows.iter().take(h as usize).enumerate() {
                        execute!(out, MoveTo(0, row as u16))?;
                        write!(out, "{}", line.chars().take(w as usize).collect::<String>())?;
                    }
                    out.flush()?;
                    drop(out);
                }
                if event::poll(Duration::from_millis(100))?
                    && let Event::Key(k) = event::read()?
                    && k.kind != event::KeyEventKind::Release
                {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                        KeyCode::Char('1'..='6') => {
                            if let KeyCode::Char(c) = k.code {
                                let bit = 1 << (c as u8 - b'1');
                                let muted = shared.mutes.load(Ordering::Relaxed) & bit != 0;
                                if !muted
                                    || (!shared.fault.load(Ordering::Relaxed)
                                        && !queued_recall
                                        && !shared.controls.pending()
                                        && !shared.transitioning.load(Ordering::Relaxed))
                                {
                                    shared.mutes.fetch_xor(bit, Ordering::Relaxed);
                                } else {
                                    editor.message = "Unmute deferred: wait, then retry".into();
                                }
                            }
                        }
                        KeyCode::Char('m') => {
                            shared.mutes.store(63, Ordering::Relaxed);
                        }
                        KeyCode::Char('u') => {
                            if !shared.fault.load(Ordering::Relaxed)
                                && !queued_recall
                                && !shared.controls.pending()
                                && !shared.transitioning.load(Ordering::Relaxed)
                            {
                                shared.mutes.store(0, Ordering::Relaxed);
                            } else {
                                editor.message = "Unmute deferred: wait, then press u".into();
                            }
                        }
                        KeyCode::Tab => modules = !modules,
                        KeyCode::Char(c) if c != 'r' => {
                            let before = editor.config;
                            let saved = editor.saved;
                            editor.key(c);
                            if editor.message == "Edited; r to refresh offline preview" {
                                editor.message = "Edited; pending until audio accepts".into();
                            }
                            if editor.config.sample_rate != config.sample_rate
                                || editor.config.max_block != config.max_block
                            {
                                editor.config = before;
                                editor.saved = saved;
                                editor.recall_requested = false;
                                editor.message = "Rate/block recall requires restart".into();
                            }
                            if editor.recall_requested {
                                shared.mutes.store(63, Ordering::Relaxed);
                                queued_recall = true;
                                editor.recall_requested = false;
                            }
                            if matches!(c, 'v' | 'n' | 'N' | 'x' | 'X' | 'j' | 'k' | 'h') {
                                modules = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(())
        })();
        shared.stop.store(true, Ordering::Relaxed);
        let result = worker.join().map_err(|_| "audio worker panicked")?;
        ui_result?;
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pages_fit_small_terminal_and_disclose_status() {
        for page in [Page::Home, Page::Features, Page::Hardware] {
            let lines = screen(page, WIDTH, HEIGHT);
            assert_eq!(lines.len(), usize::from(HEIGHT));
            assert!(
                lines
                    .iter()
                    .all(|line| line.is_ascii() && line.len() <= usize::from(WIDTH))
            );
            assert!(lines[0].contains("EXPERIMENTAL"));
            assert!(lines.last().unwrap().contains("EXIT"));
        }
    }

    #[test]
    fn resize_never_overflows_or_activates_hidden_buttons() {
        for (width, height) in [(0, 0), (1, 1), (20, 5), (40, 12), (39, 13)] {
            let lines = screen(Page::Home, width, height);
            assert!(lines.len() <= usize::from(height));
            assert!(lines.iter().all(|line| line.len() <= usize::from(width)));
            assert_eq!(hit_test(30, 12, width, height), None);
        }
    }

    #[test]
    fn touch_and_keyboard_share_navigation() {
        let page = Page::Home;
        assert_eq!(page.next().previous(), page);
        assert_eq!(page.previous().next(), page);
        assert_eq!(page.next().next().next(), page);
        assert_eq!(hit_test(4, 12, 40, 13), Some(Action::Previous));
        assert_eq!(hit_test(20, 12, 40, 13), Some(Action::Next));
        assert_eq!(hit_test(33, 12, 80, 25), Some(Action::Exit));
        assert_eq!(hit_test(40, 12, 80, 25), None);
        assert_eq!(hit_test(33, 11, 40, 13), None);
    }
}
