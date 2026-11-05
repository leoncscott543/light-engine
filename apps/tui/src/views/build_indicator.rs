//! Compact build badge; its highlight travels clockwise around a rectangle.

use std::time::Duration;

use light_tui::build_status::BuildState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme;

const RECTANGLE: [&str; 8] = ["▛", "▀", "▜", "▐", "▟", "▄", "▙", "▌"];

fn color(state: BuildState) -> Color {
    // Light themes otherwise use a monochrome status palette. Keep compiler
    // outcomes unambiguous while retaining sufficient background contrast.
    if theme::current().is_light() {
        return match state {
            BuildState::Pending | BuildState::Compiling => Color::Rgb(0x91, 0x60, 0x00),
            BuildState::Success => Color::Rgb(0x27, 0x70, 0x42),
            BuildState::Failed | BuildState::Unavailable => Color::Rgb(0xAC, 0x2C, 0x3B),
            BuildState::Standalone => theme::palette().text_dim,
        };
    }
    match state {
        BuildState::Pending | BuildState::Compiling => theme::palette().warn,
        BuildState::Success => theme::palette().ok,
        BuildState::Failed | BuildState::Unavailable => theme::palette().err,
        BuildState::Standalone => theme::palette().text_dim,
    }
}

fn symbol(state: BuildState, elapsed: Duration) -> &'static str {
    if state.animated() {
        RECTANGLE[((elapsed.as_millis() / 100) % RECTANGLE.len() as u128) as usize]
    } else {
        match state {
            BuildState::Success => "✓",
            BuildState::Failed | BuildState::Unavailable => "×",
            _ => "□",
        }
    }
}

pub fn width(state: BuildState) -> u16 {
    state.label().len() as u16 + 4
}

pub fn render(state: BuildState, elapsed: Duration, frame: &mut Frame, area: Rect) {
    let style = Style::new()
        .fg(color(state))
        .bg(theme::palette().raised)
        .add_modifier(Modifier::BOLD);
    let label = if area.width >= width(state) {
        state.label()
    } else {
        ""
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {} ", symbol(state, elapsed)), style),
            Span::styled(label, style),
            Span::styled(" ", style),
        ])),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::mock_app;
    use crate::hit::HitMap;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn rectangle_makes_a_clockwise_lap_independent_of_fps() {
        for (index, expected) in RECTANGLE.iter().enumerate() {
            assert_eq!(
                symbol(
                    BuildState::Compiling,
                    Duration::from_millis(index as u64 * 100)
                ),
                *expected
            );
        }
        assert_eq!(
            symbol(BuildState::Pending, Duration::from_millis(800)),
            RECTANGLE[0]
        );
        for state in [
            BuildState::Success,
            BuildState::Failed,
            BuildState::Standalone,
            BuildState::Unavailable,
        ] {
            assert_eq!(
                symbol(state, Duration::ZERO),
                symbol(state, Duration::from_secs(1))
            );
        }
    }

    #[test]
    fn states_use_distinct_semantic_colors_and_labels() {
        assert_eq!(color(BuildState::Compiling), theme::palette().warn);
        assert_eq!(color(BuildState::Pending), theme::palette().warn);
        assert_eq!(color(BuildState::Success), theme::palette().ok);
        assert_eq!(color(BuildState::Failed), theme::palette().err);
        assert_ne!(BuildState::Failed.label(), BuildState::Success.label());
        for theme in theme::Theme::ALL {
            theme::using(theme, || {
                assert_ne!(color(BuildState::Success), color(BuildState::Failed));
                assert_ne!(color(BuildState::Pending), color(BuildState::Success));
            });
        }
    }

    #[test]
    fn badge_is_in_the_right_corner_without_covering_clickable_tabs() {
        for width in [0, 1, 3, 12, 24, 80, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, 5)).unwrap();
            let mut app = mock_app(1);
            app.build_state = BuildState::Failed;
            let mut hits = HitMap::default();
            terminal
                .draw(|frame| {
                    super::super::chrome::header(&app, frame, Rect::new(0, 0, width, 1), &mut hits);
                })
                .unwrap();
            let badge_width = super::width(app.build_state).min(width);
            for x in width.saturating_sub(badge_width)..width {
                assert!(
                    hits.at(x, 0).is_none(),
                    "badge must not overlay tab hit targets"
                );
            }
            if width >= super::width(app.build_state) {
                let buffer = terminal.backend().buffer();
                let text: String = (0..width).map(|x| buffer[(x, 0)].symbol()).collect();
                assert!(text.ends_with("build failed "), "{text}");
                assert_eq!(buffer[(width - 2, 0)].fg, color(BuildState::Failed));
            }
        }
    }
}
