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
            "01 / PROJECT",
            "PA management for Raspberry Pi 5",
            "",
            "First target: DriveRack capabilities",
            "Rust / Linux Lite / terminal UI",
            "",
            "Scaffold only. Audio is not open.",
            "Routing and DSP are planned.",
            "Channel use will be defined later.",
        ],
        Page::Features => [
            "02 / PLANNED FEATURES",
            "LR24 crossovers + matrix patching",
            "Mono sums / delays / limiters",
            "EQ / automatic EQ / feedback control",
            "",
            "RTA: up to 8 mics at venue setup",
            "Later use: 9 channels; details TBD",
            "",
            "All audio features are pending.",
        ],
        Page::Hardware => [
            "03 / TARGET HARDWARE",
            "Raspberry Pi 5 / 64-bit Linux Lite",
            "Behringer UMC1820 / USB audio",
            "Small touchscreen / terminal",
            "Optional controllers, including MIDI",
            "",
            "Device validation is pending.",
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
