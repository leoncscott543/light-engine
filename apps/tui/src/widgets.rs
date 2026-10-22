//! Small reusable, theme-aware building blocks. Interactive widgets register
//! their click targets in the [`HitMap`] as they render.

use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::action::Action;
use crate::hit::HitMap;
use crate::theme;

pub fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, theme::title()),
            Span::raw(" "),
        ]))
        .style(theme::base())
}

pub fn hovered(mouse: Option<(u16, u16)>, area: Rect) -> bool {
    mouse.is_some_and(|(x, y)| area.contains(Position::new(x, y)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Normal,
    Danger,
}

#[derive(Debug, Clone, Copy)]
pub struct Button<'a> {
    pub label: &'a str,
    pub key: Option<&'a str>,
    pub kind: Kind,
    pub enabled: bool,
    pub action: Action,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str, action: Action) -> Self {
        Self {
            label,
            key: None,
            kind: Kind::Normal,
            enabled: true,
            action,
        }
    }

    pub fn key(mut self, key: &'a str) -> Self {
        self.key = Some(key);
        self
    }

    pub fn primary(mut self) -> Self {
        self.kind = Kind::Primary;
        self
    }

    pub fn danger(mut self) -> Self {
        self.kind = Kind::Danger;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn width(&self) -> u16 {
        let key = self.key.map_or(0, |k| k.chars().count() + 1);
        (self.label.chars().count() + key + 2) as u16
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        mouse: Option<(u16, u16)>,
        hits: &mut HitMap,
    ) {
        let hover = self.enabled && hovered(mouse, area);
        let (bg, fg) = match (self.enabled, self.kind, hover) {
            (false, ..) => (theme::SURFACE, theme::TEXT_FAINT),
            (true, Kind::Primary, false) => (theme::ACCENT, theme::TEXT),
            (true, Kind::Primary, true) => (theme::HONEY, theme::TEXT),
            (true, Kind::Danger, false) => (theme::RAISED, theme::ERR),
            (true, Kind::Normal, false) => (theme::RAISED, theme::TEXT),
            (true, _, true) => (theme::HOVER, theme::TEXT),
        };
        let style = Style::new().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(format!(" {}", self.label), style)];
        if let Some(key) = self.key {
            let key_fg = if self.enabled {
                theme::GOLD
            } else {
                theme::TEXT_FAINT
            };
            spans.push(Span::styled(
                format!(" {key}"),
                Style::new().bg(bg).fg(key_fg),
            ));
        }
        spans.push(Span::styled(" ", style));
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        if self.enabled {
            hits.push(area, self.action);
        }
    }
}

/// Lays buttons out left to right, wrapping onto following rows. Returns rows used.
pub fn button_row(
    frame: &mut Frame,
    area: Rect,
    buttons: &[Button],
    mouse: Option<(u16, u16)>,
    hits: &mut HitMap,
) -> u16 {
    let (mut x, mut y) = (area.x, area.y);
    for button in buttons {
        let w = button.width().min(area.width);
        if x + w > area.right() && x > area.x {
            x = area.x;
            y += 2;
        }
        if y >= area.bottom() {
            break;
        }
        button.render(frame, Rect::new(x, y, w, 1), mouse, hits);
        x += w + 1;
    }
    y + 1 - area.y
}

/// Bordered card showing a headline value; clicking it triggers `action`.
pub struct StatCard<'a> {
    pub title: &'a str,
    pub value: Line<'a>,
    pub detail: Line<'a>,
    pub action: Action,
}

impl StatCard<'_> {
    pub fn render(
        self,
        frame: &mut Frame,
        area: Rect,
        mouse: Option<(u16, u16)>,
        hits: &mut HitMap,
    ) {
        let hover = hovered(mouse, area);
        let block = panel(self.title, hover);
        let block = if hover {
            block.style(Style::new().bg(theme::SURFACE).fg(theme::TEXT))
        } else {
            block
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(vec![self.value, self.detail]).alignment(Alignment::Center),
            centered_rows(inner, 2),
        );
        hits.push(area, self.action);
    }
}

/// `label  value` pair as a line.
pub fn kv<'a>(label: &'a str, value: impl Into<Span<'a>>, width: usize) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<width$}"), theme::label()),
        value.into(),
    ])
}

pub fn centered_rows(area: Rect, rows: u16) -> Rect {
    let rows = rows.min(area.height);
    Rect::new(area.x, area.y + (area.height - rows) / 2, area.width, rows)
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// Placeholder shown where live data would be, e.g. when no session exists.
pub fn empty_state(frame: &mut Frame, area: Rect, title: &str, hint: &str) {
    let text = vec![
        Line::styled(title.to_string(), theme::title()),
        Line::styled(hint.to_string(), theme::dim()),
    ];
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center),
        centered_rows(area, 2),
    );
}

/// First visible row so that `selected` stays on screen.
pub fn scroll_offset(selected: usize, height: usize) -> usize {
    if height == 0 {
        0
    } else {
        selected.saturating_sub(height - 1)
    }
}

/// Inline horizontal bar like `█████▍    ` for a `0..=1` fraction.
pub fn bar(fraction: f64, width: usize) -> String {
    const PARTS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
    let cells = fraction.clamp(0.0, 1.0) * width as f64;
    let full = cells.floor() as usize;
    let mut s = "█".repeat(full);
    let rem = cells - full as f64;
    if full < width {
        let idx = (rem * 8.0).floor() as usize;
        s.push(if idx == 0 { ' ' } else { PARTS[idx - 1] });
        s.push_str(&" ".repeat(width - full - 1));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_selection_visible() {
        assert_eq!(scroll_offset(0, 5), 0);
        assert_eq!(scroll_offset(4, 5), 0);
        assert_eq!(scroll_offset(7, 5), 3);
        assert_eq!(scroll_offset(3, 0), 0);
    }

    #[test]
    fn bar_has_fixed_width() {
        for f in [0.0, 0.13, 0.5, 0.99, 1.0, 2.0] {
            assert_eq!(bar(f, 10).chars().count(), 10, "fraction {f}");
        }
        assert_eq!(bar(1.0, 4), "████");
    }
}
