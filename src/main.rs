use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use shr_pa::ui::{self, Action, Page};
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("shr-pa: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if shr_pa::commands::execute(&args).map_err(|e| io::Error::other(e.to_string()))? {
        return Ok(());
    }
    match args.as_slice() {
        [] => interactive(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!(
                "SHR PA — experimental 2 x 6 PA processor\n\nUsage:\n  shr-pa [--snapshot [home|features|hardware]]\n  shr-pa init PRESET.json\n  shr-pa migrate OLD.json NEW.json\n  shr-pa check PRESET.json\n  shr-pa render PRESET.json SOURCE OUT.wav SECONDS --unmute\n  shr-pa devices\n  shr-pa live PRESET.json CAPTURE PLAYBACK CAP_CH PLAY_CH IN_MAP OUT_MAP SECONDS [--unmute|--ui] [--signal=SOURCE]\n\nSOURCE: silence, impulse, noise, pink, sweep, sine:HZ, or stereo WAV (offline).\nMaps use zero-based indices; OUT_MAP has six entries, '-' means unmapped.\nExample stereo map: 0,1,-,-,-,- . Devices: hw:CARD=id,DEV=0.\nLive defaults muted; --ui provides live parameter editing and ramped mutes.\nDefault terminal is an offline preset editor/preview. No hardware opens.\nUI: arrows / Tab / footer touch; q / Esc / Ctrl+C exit.\nMinimum terminal: 40x13. See docs/RUNNING.md."
            );
            Ok(())
        }
        [arg] if arg == "--version" => {
            println!("shr-pa {} (highly experimental)", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [arg] if arg == "--snapshot" => snapshot(Page::Home),
        [arg, page] if arg == "--snapshot" => snapshot(Page::parse(page).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown page; use home, features, or hardware",
            )
        })?),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown arguments; use --help",
        )),
    }
}

fn snapshot(page: Page) -> io::Result<()> {
    let mut out = io::stdout().lock();
    for line in ui::Editor::new()?.screen(page, ui::WIDTH, ui::HEIGHT) {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let guard = Self;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide)?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            ResetColor,
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
    }
}

fn interactive() -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "interactive mode needs a terminal; use --snapshot for plain text",
        ));
    }
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ] {
        signal_hook::flag::register(signal, stop.clone())?;
    }
    // Initialize the event source before painting: a resize immediately after
    // the first frame must not arrive before SIGWINCH handling is installed.
    let _guard = TerminalGuard::enter()?;
    event::poll(Duration::ZERO)?;
    let mut editor = ui::Editor::new()?;
    editor.attach_library(std::path::PathBuf::from(".shr-pa"));
    let mut page = Page::Home;
    let mut dirty = true;
    while !stop.load(Ordering::Relaxed) {
        if dirty {
            let (width, height) = terminal::size()?;
            let mut out = io::stdout().lock();
            queue!(out, Clear(ClearType::All))?;
            for (row, line) in editor.screen(page, width, height).iter().enumerate() {
                queue!(
                    out,
                    MoveTo(0, row as u16),
                    SetForegroundColor(if row == 0 { Color::Cyan } else { Color::White }),
                    Print(line)
                )?;
            }
            out.flush()?;
            dirty = false;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let action = match event::read()? {
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && !(key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                    && editor.command_key(key.code) =>
            {
                dirty = true;
                None
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Exit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::Exit)
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Tab => Some(Action::Next),
                KeyCode::Left | KeyCode::Up | KeyCode::BackTab => Some(Action::Previous),
                KeyCode::Char(c) => {
                    editor.key(c);
                    dirty = true;
                    None
                }
                _ => None,
            },
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let (width, height) = terminal::size()?;
                ui::hit_test(mouse.column, mouse.row, width, height)
            }
            Event::Resize(_, _) => {
                dirty = true;
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            match action {
                Action::Exit => break,
                Action::Next => page = page.next(),
                Action::Previous => page = page.previous(),
            }
            dirty = true;
        }
    }
    Ok(())
}
