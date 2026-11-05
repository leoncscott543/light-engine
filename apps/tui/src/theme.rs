//! Scoped palettes: rendering two apps or parallel tests never shares theme state.
use light_contracts::{LogLevel, ProcessState, SimStatus};
use ratatui::style::{Color, Modifier, Style};
use std::cell::Cell;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    DarkLab,
    Spectrum,
    Blue,
    Amber,
    Sage,
    Graphite,
}

impl Theme {
    pub const ALL: [Self; 6] = [
        Self::DarkLab,
        Self::Spectrum,
        Self::Blue,
        Self::Amber,
        Self::Sage,
        Self::Graphite,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::DarkLab => "dark-lab",
            Self::Spectrum => "spectrum",
            Self::Blue => "blue",
            Self::Amber => "amber",
            Self::Sage => "sage",
            Self::Graphite => "graphite",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == value)
    }
    pub fn cycle(self, delta: i32) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0) as i32;
        Self::ALL[(i + delta).rem_euclid(Self::ALL.len() as i32) as usize]
    }
    pub fn is_light(self) -> bool {
        !matches!(self, Self::DarkLab | Self::Spectrum)
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub raised: Color,
    pub border: Color,
    pub border_focus: Color,
    pub accent: Color,
    pub honey: Color,
    pub gold: Color,
    pub select: Color,
    pub hover: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_faint: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
}

fn rgb(c: u32) -> Color {
    Color::Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
thread_local! { static ACTIVE: Cell<Theme> = const { Cell::new(Theme::DarkLab) }; }

pub fn using<T>(theme: Theme, render: impl FnOnce() -> T) -> T {
    struct Reset(Theme);
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.set(self.0);
        }
    }
    let _reset = Reset(ACTIVE.replace(theme));
    render()
}

pub fn current() -> Theme {
    ACTIVE.get()
}

pub fn palette() -> Palette {
    let theme = current();
    let values = match theme {
        Theme::DarkLab => [
            0x0c121b, 0x121d2a, 0x1a293b, 0x3c526b, 0x60c5e8, 0x1c4356, 0xafb9fa, 0x60c5e8,
            0x213e57, 0x1c3045, 0xe3edf7, 0xa6b6ca, 0x7e91ab, 0x73d6b2, 0xf3ca78, 0xff8c9b,
        ],
        Theme::Spectrum => [
            0x11101c, 0x1c192d, 0x28223f, 0x594c79, 0xc3a0ff, 0x44315e, 0x63dddf, 0xc3a0ff,
            0x493367, 0x35294d, 0xf0eafa, 0xbeb0d7, 0x9584b3, 0x8fdfac, 0xffcc78, 0xff8fab,
        ],
        Theme::Blue => [
            0xf4f8fc, 0xe9f0f8, 0xdbe7f4, 0x97b1cc, 0x315f8c, 0xc9dcef, 0x315f8c, 0x315f8c,
            0xc9dcef, 0xddeaf7, 0x19344f, 0x496683, 0x607c98, 0x315f8c, 0x315f8c, 0x19344f,
        ],
        Theme::Amber => [
            0xfbf7ec, 0xf3ead7, 0xeaddbd, 0xb5a071, 0x775818, 0xe6d4a8, 0x775818, 0x775818,
            0xe6d4a8, 0xf0e3c3, 0x443517, 0x76613b, 0x88734e, 0x775818, 0x775818, 0x443517,
        ],
        Theme::Sage => [
            0xf3f8f3, 0xe6eee6, 0xd6e4d7, 0x94ac98, 0x365e43, 0xc9dfcd, 0x365e43, 0x365e43,
            0xc9dfcd, 0xddeade, 0x233e2c, 0x506b57, 0x667f6d, 0x365e43, 0x365e43, 0x233e2c,
        ],
        Theme::Graphite => [
            0xf7f8f8, 0xeceeee, 0xdde1e2, 0xa1aaad, 0x45565e, 0xd2dade, 0x45565e, 0x45565e,
            0xd2dade, 0xe1e6e8, 0x26373e, 0x55666d, 0x6d7c83, 0x45565e, 0x45565e, 0x26373e,
        ],
    };
    let [
        bg,
        surface,
        raised,
        border,
        border_focus,
        accent,
        honey,
        gold,
        select,
        hover,
        text,
        text_dim,
        text_faint,
        ok,
        warn,
        err,
    ] = values.map(rgb);
    Palette {
        bg,
        surface,
        raised,
        border,
        border_focus,
        accent,
        honey,
        gold,
        select,
        hover,
        text,
        text_dim,
        text_faint,
        ok,
        warn,
        err,
    }
}

pub fn base() -> Style {
    Style::new().fg(palette().text).bg(palette().bg)
}

pub fn dim() -> Style {
    Style::new().fg(palette().text_dim)
}

pub fn faint() -> Style {
    Style::new().fg(palette().text_faint)
}

pub fn label() -> Style {
    Style::new().fg(palette().text_dim)
}

pub fn value() -> Style {
    Style::new().fg(palette().text).add_modifier(Modifier::BOLD)
}

pub fn title() -> Style {
    Style::new().fg(palette().gold).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused {
        palette().border_focus
    } else {
        palette().border
    })
}

pub fn selected() -> Style {
    Style::new()
        .bg(palette().select)
        .fg(palette().text)
        .add_modifier(Modifier::BOLD)
}

pub fn key_hint() -> Style {
    Style::new()
        .fg(palette().bg)
        .bg(palette().gold)
        .add_modifier(Modifier::BOLD)
}

pub fn sim_status(status: &SimStatus) -> Style {
    let color = match status {
        SimStatus::Inactive => palette().text_faint,
        SimStatus::Idle => palette().text_dim,
        SimStatus::Running => palette().ok,
        SimStatus::Paused => palette().honey,
        SimStatus::Faulted(_) => palette().err,
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

pub fn process_state(state: &ProcessState) -> Style {
    let color = match state {
        ProcessState::Idle => palette().text_dim,
        ProcessState::Running => palette().ok,
        ProcessState::Stopped => palette().text_faint,
        ProcessState::Failed(_) => palette().err,
    };
    Style::new().fg(color)
}

pub fn log_level(level: LogLevel) -> Style {
    match level {
        LogLevel::Trace => Style::new().fg(palette().text_faint),
        LogLevel::Debug => Style::new().fg(palette().text_dim),
        LogLevel::Info => Style::new().fg(palette().gold),
        LogLevel::Warn => Style::new().fg(palette().warn).add_modifier(Modifier::BOLD),
        LogLevel::Error => Style::new().fg(palette().err).add_modifier(Modifier::BOLD),
    }
}

/// Green-ish when small, amber then burnt orange as the error grows.
pub fn energy_error(rel: f64) -> Style {
    let color = if rel < 1e-6 {
        palette().ok
    } else if rel < 1e-3 {
        palette().honey
    } else if rel < 1e-1 {
        palette().warn
    } else {
        palette().err
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scopes_restore_and_every_palette_has_distinct_surfaces() {
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(theme.name()), Some(theme));
            using(theme, || {
                assert_eq!(current(), theme);
                let p = palette();
                assert_ne!(p.bg, p.text);
                assert_ne!(p.bg, p.border);
                using(theme.cycle(1), || assert_eq!(current(), theme.cycle(1)));
                assert_eq!(current(), theme);
            });
            assert_eq!(current(), Theme::default());
        }
        assert_eq!(Theme::default().cycle(6), Theme::default());
        assert!(Theme::parse("not-a-theme").is_none());
    }
}
