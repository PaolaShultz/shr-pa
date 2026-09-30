//! Command-line operations; no hardware opens unless `live` is explicitly selected.
use crate::{
    config::Config,
    offline,
    transport::{self, LiveOptions, Mapping, Shared},
};
use std::sync::{Arc, atomic::Ordering};
pub fn execute(args: &[String]) -> offline::Result<bool> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(false);
    };
    match command {
        "migrate" if args.len() == 3 => {
            crate::config::migrate(&args[1], &args[2])?;
            println!("Migrated to current schema: {} (source retained)", args[2]);
        }
        "init" if args.len() == 2 => {
            Config::default().save(&args[1])?;
            println!("Saved validated preset {}", args[1]);
        }
        "check" if args.len() == 2 => {
            let c = Config::load(&args[1])?;
            println!(
                "Valid v{} {:?}, {} Hz, max {} frames, six logical outputs",
                c.version, c.layout, c.sample_rate, c.max_block
            );
        }
        "render" if args.len() == 6 => {
            if args[5] != "--unmute" {
                return Err("render requires explicit --unmute".into());
            }
            let c = Config::load(&args[1])?;
            println!(
                "{:?}",
                offline::render(c, &args[2], &args[3], args[4].parse()?)?
            );
        }
        "devices" if args.len() == 1 => {
            println!("{}", std::fs::read_to_string("/proc/asound/cards")?);
            for entry in std::fs::read_dir("/proc/asound")? {
                let path = entry?.path();
                if path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("card"))
                    && path.is_dir()
                {
                    for f in std::fs::read_dir(path)? {
                        let p = f?.path();
                        if p.file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with("stream"))
                        {
                            println!("{}\n{}", p.display(), std::fs::read_to_string(&p)?);
                        }
                    }
                }
            }
        }
        "live" if (9..=12).contains(&args.len()) => {
            let c = Config::load(&args[1])?;
            let map = Mapping::parse(args[4].parse()?, args[5].parse()?, &args[6], &args[7])?;
            let shared = Arc::new(Shared::default());
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            for signal in [
                signal_hook::consts::SIGINT,
                signal_hook::consts::SIGTERM,
                signal_hook::consts::SIGHUP,
            ] {
                signal_hook::flag::register(signal, stop.clone())?;
            }
            let mut signal = None;
            let mut terminal = false;
            for arg in &args[9..] {
                if arg == "--unmute" {
                    shared.mutes.store(0, Ordering::Relaxed);
                } else if arg == "--ui" {
                    terminal = true;
                } else if let Some(s) = arg.strip_prefix("--signal=") {
                    signal = Some(offline::Signal::parse(s)?);
                } else {
                    return Err("unknown live option".into());
                }
            }
            let options = LiveOptions {
                capture: &args[2],
                playback: &args[3],
                map,
                seconds: args[8].parse()?,
                signal,
            };
            let result = std::thread::scope(|scope| {
                let s = shared.clone();
                let stop = stop.clone();
                let monitor = scope.spawn(move || {
                    while !s.stop.load(Ordering::Relaxed) {
                        if stop.load(Ordering::Relaxed) {
                            s.stop.store(true, Ordering::Relaxed);
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                });
                let result = if terminal {
                    crate::ui::live(c, &args[1], options, shared.clone())
                } else {
                    transport::run(c, options, &shared)
                };
                shared.stop.store(true, Ordering::Relaxed);
                monitor.join().expect("signal monitor panicked");
                result
            })?;
            println!("{result:?}");
            if result.fault.is_some() {
                return Err("audio session stopped on fault; restart explicitly".into());
            }
        }
        "migrate" | "init" | "check" | "render" | "live" | "devices" => {
            return Err("invalid command arguments; use --help".into());
        }
        _ => return Ok(false),
    }
    Ok(true)
}
