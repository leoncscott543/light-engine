use light_contracts::LogLevel;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::app::App;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, Button, hovered, panel};

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [bar, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    filter_bar(app, frame, bar, hits);
    log_list(app, frame, list);
}

fn filter_bar(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let mut x = area.x + 1;
    let label = "min level ";
    frame.render_widget(
        Paragraph::new(Span::styled(label, theme::label())),
        Rect::new(x, area.y, label.len() as u16, 1),
    );
    x += label.len() as u16;
    for level in LogLevel::ALL {
        let text = format!(" {level} ");
        let w = text.len() as u16;
        if x + w > area.right() {
            return;
        }
        let rect = Rect::new(x, area.y, w, 1);
        let style = if level == app.logs.min_level {
            theme::log_level(level)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else if hovered(app.mouse, rect) {
            theme::log_level(level).bg(theme::HOVER)
        } else {
            theme::log_level(level).bg(theme::SURFACE)
        };
        frame.render_widget(Paragraph::new(Span::styled(text, style)), rect);
        hits.push(rect, Action::SetLogLevel(level));
        x += w + 1;
    }

    let follow_label = if app.logs.follow {
        "following ✓"
    } else {
        "follow"
    };
    let mut follow = Button::new(follow_label, Action::ToggleFollow).key("f");
    if app.logs.follow {
        follow = follow.primary();
    }
    let buttons = [
        follow,
        Button::new("Clear", Action::ClearLogs).key("c").danger(),
    ];
    let x = x + 2;
    if x < area.right() {
        widgets::button_row(
            frame,
            Rect::new(x, area.y, area.right() - x, 1),
            &buttons,
            app.mouse,
            hits,
        );
    }
}

fn log_list(app: &App, frame: &mut Frame, area: Rect) {
    let visible: Vec<_> = app.logs.visible().collect();
    let title = format!("Logs ({}/{})", visible.len(), app.logs.entries.len());
    let block = panel(&title, true);
    let block = if !app.logs.follow && app.logs.scroll > 0 {
        block.title(
            Line::from(Span::styled(
                format!(" ↑ {} newer · End to follow ", app.logs.scroll),
                Style::new().fg(theme::WARN),
            ))
            .right_aligned(),
        )
    } else {
        block
    };
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if visible.is_empty() {
        widgets::empty_state(
            frame,
            inner,
            "no log entries",
            "at or above the selected level",
        );
        return;
    }

    let height = inner.height as usize;
    let end = visible.len().saturating_sub(app.logs.scroll);
    let start = end.saturating_sub(height);
    let lines: Vec<Line> = visible[start..end]
        .iter()
        .map(|e| {
            let secs = e.at.as_secs_f64();
            Line::from(vec![
                Span::styled(format!("{secs:>9.3}s "), theme::faint()),
                Span::styled(format!("{:<5} ", e.level), theme::log_level(e.level)),
                Span::styled(format!("{:<6} ", e.source), theme::dim()),
                Span::styled(e.message.clone(), theme::base()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}
