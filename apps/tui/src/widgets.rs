//! Small reusable, theme-aware building blocks. Interactive widgets register
//! their click targets in the [`HitMap`] as they render.

use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::cell::Cell;

use crate::action::Action;
use crate::hit::HitMap;
use crate::theme;

pub fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Plain)
        .border_style(theme::border(focused))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(clean(title), theme::title()),
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
            (false, ..) => (theme::palette().surface, theme::palette().text_faint),
            (true, Kind::Primary, false) => (theme::palette().accent, theme::palette().text),
            (true, Kind::Primary, true) => (theme::palette().honey, theme::palette().text),
            (true, Kind::Danger, false) => (theme::palette().raised, theme::palette().err),
            (true, Kind::Normal, false) => (theme::palette().raised, theme::palette().text),
            (true, _, true) => (theme::palette().hover, theme::palette().text),
        };
        let style = Style::new().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(format!(" {}", self.label), style)];
        if let Some(key) = self.key {
            let key_fg = if self.enabled {
                theme::palette().gold
            } else {
                theme::palette().text_faint
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
            block.style(
                Style::new()
                    .bg(theme::palette().surface)
                    .fg(theme::palette().text),
            )
        } else {
            block
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let rows = centered_rows(inner, 2);
        if rows.height > 0 {
            frame.render_widget(
                Paragraph::new(self.value).alignment(Alignment::Right),
                Rect::new(rows.x, rows.y, rows.width.saturating_sub(1), 1),
            );
        }
        if rows.height > 1 {
            frame.render_widget(
                Paragraph::new(self.detail).alignment(Alignment::Right),
                Rect::new(rows.x, rows.y + 1, rows.width.saturating_sub(1), 1),
            );
        }
        hits.push(area, self.action);
    }
}

/// `label  value` pair as a line.
pub fn kv<'a>(label: &'a str, value: impl Into<Span<'a>>, width: usize) -> Line<'a> {
    let value = value.into();
    Line::from(vec![
        Span::styled(format!("{label:<width$}"), theme::label()),
        Span::styled(clean(&value.content), value.style),
    ])
}

/// Payload strings remain text, including when a producer emits terminal controls.
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\n' | '\r' => '↵',
            '\t' => ' ',
            c if c.is_control() => '�',
            c => c,
        })
        .collect()
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

/// Persistent viewport: moving up inside the viewport never shifts all rows.
#[derive(Default)]
pub struct Viewport {
    offset: Cell<usize>,
    pub height: Cell<usize>,
}

impl Viewport {
    pub fn range(&self, selected: usize, total: usize, height: usize) -> std::ops::Range<usize> {
        self.height.set(height);
        let mut offset = self.offset.get().min(total.saturating_sub(height));
        if selected < offset {
            offset = selected;
        }
        if height > 0 && selected >= offset.saturating_add(height) {
            offset = scroll_offset(selected, height);
        }
        offset = offset.min(total.saturating_sub(height));
        self.offset.set(offset);
        offset..offset.saturating_add(height).min(total)
    }
}

/// Dedicated border column; scrollbar never overwrites table content.
pub fn scrollbar(frame: &mut Frame, area: Rect, total: usize, offset: usize, visible: usize) {
    if area.height < 3 || area.width < 2 || total <= visible || visible == 0 {
        return;
    }
    let mut state = ScrollbarState::new(total)
        .position(offset)
        .viewport_content_length(visible);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("┃")
            .track_style(theme::border(false))
            .thumb_style(theme::border(true)),
        Rect::new(area.x, area.y + 1, area.width, area.height - 2),
        &mut state,
    );
}

/// Terminal-cell truncation and padding, safe for CJK, combining marks and UTF-8.
pub fn fixed(text: &str, width: usize, right: bool) -> String {
    use ratatui::text::Span;
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        if ch.is_control() {
            continue;
        }
        let w = Span::raw(ch.to_string()).width();
        if used + w > width {
            break;
        }
        out.push(ch);
        used += w;
    }
    let pad = " ".repeat(width.saturating_sub(used));
    if right {
        format!("{pad}{out}")
    } else {
        format!("{out}{pad}")
    }
}

pub fn scrollbar_hits(area: Rect, total: usize, visible: usize, hits: &mut HitMap) {
    if area.height < 3 || area.width < 2 || total <= visible || visible == 0 {
        return;
    }
    let height = usize::from(area.height - 2);
    for row in 0..height {
        let index = row * total.saturating_sub(1) / height.saturating_sub(1).max(1);
        hits.push(
            Rect::new(area.right() - 1, area.y + 1 + row as u16, 1, 1),
            Action::ScrollTo(index),
        );
    }
}

pub fn evidence(meta: &light_contracts::ObservationMeta, path: &str) -> String {
    let source = meta.field(path);
    format!("{} / {}", source.origin, source.quality)
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

    #[test]
    fn viewport_moves_only_when_selection_leaves_the_window() {
        let view = Viewport::default();
        assert_eq!(view.range(8, 30, 5), 4..9);
        assert_eq!(view.range(7, 30, 5), 4..9);
        assert_eq!(view.range(5, 30, 5), 4..9);
        assert_eq!(view.range(3, 30, 5), 3..8);
        assert_eq!(view.range(29, 30, 5), 25..30);
        assert_eq!(view.range(2, 3, 8), 0..3);
    }

    #[test]
    fn cells_are_fixed_width_for_unicode_and_long_values() {
        for text in ["界粒子", "μs", "999999999999", "a\u{301}", "\u{1b}[2J"] {
            for width in 0..12 {
                let value = fixed(text, width, true);
                assert_eq!(Span::raw(value.clone()).width(), width, "{value:?}");
                assert!(!value.contains('\u{1b}'));
            }
        }
    }
}
