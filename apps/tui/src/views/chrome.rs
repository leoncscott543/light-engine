use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::action::Action;
use crate::app::{App, Tab};
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, hovered};

pub fn header(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme::RAISED)),
        area,
    );

    let brand = " ◆ LIGHT ";
    let brand_w = brand.chars().count() as u16;
    frame.render_widget(
        Paragraph::new(Span::styled(brand, theme::key_hint())),
        Rect {
            width: brand_w.min(area.width),
            ..area
        },
    );

    let mut x = area.x + brand_w + 1;
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let label = format!(" {} {} ", i + 1, tab.title());
        let w = label.chars().count() as u16;
        if x + w > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, w, 1);
        let style = if *tab == app.tab {
            Style::new()
                .bg(theme::ACCENT)
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else if hovered(app.mouse, rect) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            Style::new().bg(theme::RAISED).fg(theme::TEXT_DIM)
        };
        frame.render_widget(Paragraph::new(Span::styled(label, style)), rect);
        hits.push(rect, Action::GoTo(*tab));
        x += w;
    }

    let status = app.sim.status();
    let right = Line::from(vec![
        Span::styled("● ", theme::sim_status(&status)),
        Span::styled(format!("sim {status}"), theme::sim_status(&status)),
        Span::styled("  │  ", theme::faint()),
        Span::styled(
            format!("{} v{} ", app.backend.name, app.backend.version),
            theme::dim(),
        ),
        Span::styled(
            format!(" {} ", app.backend.kind.to_string().to_uppercase()),
            theme::key_hint(),
        ),
    ]);
    let right_w = right.width() as u16;
    if x + right_w < area.right() {
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(Style::new().bg(theme::RAISED)),
            Rect::new(area.right() - right_w, area.y, right_w, 1),
        );
    }
}

fn hints(app: &App) -> Vec<(&'static str, &'static str, Action)> {
    let mut v = match app.tab {
        Tab::Overview => vec![
            ("space", "run/pause", Action::ToggleRun),
            (".", "step", Action::Step(1)),
            ("r", "reset", Action::ResetSession),
        ],
        Tab::Simulation => vec![
            ("space", "run/pause", Action::ToggleRun),
            (".", "step", Action::Step(1)),
            ("←→", "adjust", Action::AdjustField(app.sim.field, 1)),
            ("R", "recreate", Action::RecreateSession),
        ],
        Tab::Benchmarks => vec![
            ("enter", "run", Action::RunBench),
            ("x", "cancel", Action::CancelBench),
            ("←→", "samples", Action::AdjustSamples(1)),
        ],
        Tab::Processes => vec![("↑↓", "select", Action::Down)],
        Tab::Logs => vec![
            ("f", "follow", Action::ToggleFollow),
            ("v", "level", Action::CycleLogLevel),
            ("c", "clear", Action::ClearLogs),
        ],
    };
    v.extend([
        ("tab", "next", Action::NextTab),
        ("?", "help", Action::ToggleHelp),
        ("q", "quit", Action::Quit),
    ]);
    v
}

pub fn footer(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme::SURFACE)),
        area,
    );
    let mut x = area.x + 1;
    for (key, label, action) in hints(app) {
        let key_s = format!(" {key} ");
        let label_s = format!(" {label} ");
        let w = (key_s.chars().count() + label_s.chars().count()) as u16;
        if x + w > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, w, 1);
        let label_style = if hovered(app.mouse, rect) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            Style::new().bg(theme::SURFACE).fg(theme::TEXT_DIM)
        };
        let line = Line::from(vec![
            Span::styled(key_s, theme::key_hint()),
            Span::styled(label_s, label_style),
        ]);
        frame.render_widget(Paragraph::new(line), rect);
        hits.push(rect, action);
        x += w + 1;
    }

    if let Some(toast) = &app.toast {
        let text = format!(" {} ", toast.text);
        let w = (text.chars().count() as u16).min(area.right().saturating_sub(x));
        if w > 4 {
            let style = theme::log_level(toast.level).bg(theme::SURFACE);
            frame.render_widget(
                Paragraph::new(Span::styled(text, style)).alignment(Alignment::Right),
                Rect::new(area.right() - w, area.y, w, 1),
            );
        }
    }
}

const HELP: &[(&str, &str)] = &[
    ("1-5 / tab", "switch tab (or click a tab)"),
    ("↑↓ j k / wheel", "move selection, scroll"),
    ("pgup pgdn home end", "jump"),
    ("", ""),
    ("space", "start / pause (creates a session if none)"),
    ("n", "create session from draft config"),
    (". / >", "step 1 / 100 while paused"),
    ("r", "reset session (sim & overview tabs)"),
    ("R / D", "recreate / destroy session"),
    ("←→ h l - +", "adjust selected config field"),
    ("", ""),
    ("enter / r", "run selected benchmark"),
    ("x", "cancel running benchmark"),
    ("←→  [ ]  { }", "samples, warmup, iters per sample"),
    ("", ""),
    ("f / v / c", "logs: follow, min level, clear"),
    ("?  esc  q", "help, close, quit"),
];

pub fn help(frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    hits.push(area, Action::CloseOverlay);
    let popup = widgets::centered(area, 64, HELP.len() as u16 + 4);
    frame.render_widget(Clear, popup);
    let block = widgets::panel("Keys & Mouse", true);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = HELP
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<20}"), theme::title()),
                Span::styled(*d, theme::base()),
            ])
        })
        .collect();
    lines.push(Line::styled(
        " everything is clickable · click outside to close",
        theme::faint(),
    ));
    frame.render_widget(Paragraph::new(lines), inner);
    // Clicks inside the popup should not close it.
    hits.push(popup, Action::Noop);
}
