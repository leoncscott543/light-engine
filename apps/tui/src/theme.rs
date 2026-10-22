//! Yellow palette inspired by the light, warm aluminium of a yellow laptop:
//! a pale butter background with deeper honey/gold hues for structure.

use light_contracts::{LogLevel, ProcessState, SimStatus};
use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(0xFB, 0xF5, 0xDC);
pub const SURFACE: Color = Color::Rgb(0xF7, 0xEC, 0xBF);
pub const RAISED: Color = Color::Rgb(0xF2, 0xE0, 0x96);
pub const BORDER: Color = Color::Rgb(0xE4, 0xC8, 0x62);
pub const BORDER_FOCUS: Color = Color::Rgb(0xD0, 0x9C, 0x08);
pub const ACCENT: Color = Color::Rgb(0xF5, 0xC4, 0x00);
pub const HONEY: Color = Color::Rgb(0xE2, 0xA9, 0x00);
pub const GOLD: Color = Color::Rgb(0xBF, 0x88, 0x00);
pub const SELECT: Color = Color::Rgb(0xF6, 0xD8, 0x4C);
pub const HOVER: Color = Color::Rgb(0xF9, 0xE5, 0x86);

pub const TEXT: Color = Color::Rgb(0x3F, 0x33, 0x00);
pub const TEXT_DIM: Color = Color::Rgb(0x80, 0x6B, 0x1F);
pub const TEXT_FAINT: Color = Color::Rgb(0xB8, 0x9F, 0x4A);

pub const OK: Color = Color::Rgb(0x86, 0x96, 0x0E);
pub const WARN: Color = Color::Rgb(0xE0, 0x86, 0x00);
pub const ERR: Color = Color::Rgb(0xB5, 0x46, 0x00);

pub fn base() -> Style {
    Style::new().fg(TEXT).bg(BG)
}

pub fn dim() -> Style {
    Style::new().fg(TEXT_DIM)
}

pub fn faint() -> Style {
    Style::new().fg(TEXT_FAINT)
}

pub fn label() -> Style {
    Style::new().fg(TEXT_DIM)
}

pub fn value() -> Style {
    Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn title() -> Style {
    Style::new().fg(GOLD).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused { BORDER_FOCUS } else { BORDER })
}

pub fn selected() -> Style {
    Style::new()
        .bg(SELECT)
        .fg(TEXT)
        .add_modifier(Modifier::BOLD)
}

pub fn key_hint() -> Style {
    Style::new().fg(BG).bg(GOLD).add_modifier(Modifier::BOLD)
}

pub fn sim_status(status: &SimStatus) -> Style {
    let color = match status {
        SimStatus::Inactive => TEXT_FAINT,
        SimStatus::Idle => TEXT_DIM,
        SimStatus::Running => OK,
        SimStatus::Paused => HONEY,
        SimStatus::Faulted(_) => ERR,
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

pub fn process_state(state: &ProcessState) -> Style {
    let color = match state {
        ProcessState::Idle => TEXT_DIM,
        ProcessState::Running => OK,
        ProcessState::Stopped => TEXT_FAINT,
        ProcessState::Failed(_) => ERR,
    };
    Style::new().fg(color)
}

pub fn log_level(level: LogLevel) -> Style {
    match level {
        LogLevel::Trace => Style::new().fg(TEXT_FAINT),
        LogLevel::Debug => Style::new().fg(TEXT_DIM),
        LogLevel::Info => Style::new().fg(GOLD),
        LogLevel::Warn => Style::new().fg(WARN).add_modifier(Modifier::BOLD),
        LogLevel::Error => Style::new().fg(ERR).add_modifier(Modifier::BOLD),
    }
}

/// Green-ish when small, amber then burnt orange as the error grows.
pub fn energy_error(rel: f64) -> Style {
    let color = if rel < 1e-6 {
        OK
    } else if rel < 1e-3 {
        HONEY
    } else if rel < 1e-1 {
        WARN
    } else {
        ERR
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}
