use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::action::Action;
use crate::app::{App, Tab};
use crate::explorer::ExplorerAction;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, hovered};

pub fn header(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme::palette().raised)),
        area,
    );

    let badge_width = super::build_indicator::width(app.build_state).min(area.width);
    let badge = Rect::new(
        area.right() - badge_width,
        area.y,
        badge_width,
        area.height.min(1),
    );
    let content_right = badge.x;
    let brand = " ◆ LIGHT ";
    let brand_w = brand.chars().count() as u16;
    frame.render_widget(
        Paragraph::new(Span::styled(brand, theme::key_hint())),
        Rect {
            width: brand_w.min(content_right.saturating_sub(area.x)),
            ..area
        },
    );

    let mut x = area.x + brand_w + 1;
    let compact = content_right.saturating_sub(x) < 96;
    let short = ["Ov", "Si", "Pa", "Be", "Pr", "Lo", "Te"];
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let label = format!(
            " {} {} ",
            i + 1,
            if compact { short[i] } else { tab.title() }
        );
        let w = label.chars().count() as u16;
        if x + w > content_right {
            break;
        }
        let rect = Rect::new(x, area.y, w, 1);
        let style = if *tab == app.tab {
            Style::new()
                .bg(theme::palette().accent)
                .fg(theme::palette().text)
                .add_modifier(Modifier::BOLD)
        } else if hovered(app.mouse, rect) {
            Style::new()
                .bg(theme::palette().hover)
                .fg(theme::palette().text)
        } else {
            Style::new()
                .bg(theme::palette().raised)
                .fg(theme::palette().text_dim)
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
    if x + right_w < content_right {
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(Style::new().bg(theme::palette().raised)),
            Rect::new(content_right - right_w, area.y, right_w, 1),
        );
    }
    super::build_indicator::render(app.build_state, app.uptime(), frame, badge);
}

pub fn instruments(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let palette = theme::palette();
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(palette.surface)),
        area,
    );
    if area.height == 0 {
        return;
    }
    let origin = app
        .sim
        .snapshot
        .as_ref()
        .map_or("—".into(), |s| s.meta.label().to_owned());
    let health = app.snapshot_health();
    let age = app
        .snapshot_feed
        .last_received
        .map_or("—".into(), |t| crate::fmt::duration(t.elapsed()));
    let text = if area.width < 100 {
        format!(
            " SIM {origin:7} {health:10} │ SNAP {}Hz │ UI {}Hz",
            widgets::fixed(&format!("{:.1}", app.snapshot_feed.hz()), 7, true),
            widgets::fixed(&format!("{:.1}", app.ui.fps()), 5, true)
        )
    } else {
        format!(
            " SIM {:7} {:10} │ SNAP {:5.1} Hz {:>8} │ UI {:5.1}/{:<3} Hz",
            origin,
            health,
            app.snapshot_feed.hz(),
            age,
            app.ui.fps(),
            app.ui.target_fps
        )
    };
    let theme_label = format!(" t {} ", app.theme.name());
    let w = (theme_label.len() as u16).min(area.width);
    let left_w = area.width.saturating_sub(w + 1);
    frame.render_widget(
        Paragraph::new(Span::styled(text, theme::dim())),
        Rect::new(area.x, area.y, left_w, 1),
    );
    let rect = Rect::new(area.right() - w, area.y, w, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(theme_label, theme::key_hint())),
        rect,
    );
    hits.push(rect, Action::CycleTheme(1));
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
        Tab::Particles if app.explorer.editing => vec![
            ("enter", "apply", Action::Explore(ExplorerAction::Submit)),
            ("esc", "cancel", Action::Explore(ExplorerAction::CancelEdit)),
            ("tab", "complete", Action::Explore(ExplorerAction::Complete)),
            (
                "↑↓",
                "history",
                Action::Explore(ExplorerAction::History(-1)),
            ),
        ],
        Tab::Particles => vec![
            ("/", "query", Action::Explore(ExplorerAction::EditQuery)),
            ("x y", "axes", Action::Explore(ExplorerAction::CycleX(1))),
            ("m", "mode", Action::Explore(ExplorerAction::CycleMode)),
            (
                "←→",
                "field",
                Action::Explore(ExplorerAction::CycleField(1)),
            ),
            (
                "c",
                "colors",
                Action::Explore(ExplorerAction::CycleColormap),
            ),
            (
                "⌫",
                "pop filter",
                Action::Explore(ExplorerAction::PopFilter),
            ),
            ("a", "live", Action::Explore(ExplorerAction::ToggleLive)),
        ],
        Tab::Benchmarks => vec![
            ("enter", "run", Action::RunBench),
            ("x", "cancel", Action::CancelBench),
            ("←→", "samples", Action::AdjustSamples(1)),
        ],
        Tab::Processes => vec![("↑↓", "select", Action::Down)],
        Tab::Telemetry => vec![
            ("↑↓", "scroll", Action::Down),
            ("pgdn", "page", Action::PageDown),
        ],
        Tab::Logs => vec![
            ("f", "follow", Action::ToggleFollow),
            ("v", "level", Action::CycleLogLevel),
            ("c", "clear", Action::ClearLogs),
        ],
    };
    v.extend([
        ("i", "bits", Action::ToggleInspector),
        ("t", "theme", Action::CycleTheme(1)),
        ("tab", "next", Action::NextTab),
        ("?", "help", Action::ToggleHelp),
        ("q", "quit", Action::Quit),
    ]);
    v
}

pub fn footer(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    if let Some(toast) = &app.toast {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    " {} · {} · 6 Logs ",
                    toast.level,
                    widgets::clean(&toast.text)
                ),
                theme::log_level(toast.level).bg(theme::palette().surface),
            )),
            area,
        );
        hits.push(area, Action::GoTo(Tab::Logs));
        return;
    }
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme::palette().surface)),
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
            Style::new()
                .bg(theme::palette().hover)
                .fg(theme::palette().text)
        } else {
            Style::new()
                .bg(theme::palette().surface)
                .fg(theme::palette().text_dim)
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
            let style = theme::log_level(toast.level).bg(theme::palette().surface);
            frame.render_widget(
                Paragraph::new(Span::styled(text, style)).alignment(Alignment::Right),
                Rect::new(area.right() - w, area.y, w, 1),
            );
        }
    }
}

const HELP: &[(&str, &str)] = &[
    ("i", "inspect exact payload bits (frozen capture)"),
    ("1-7 / tab", "switch tab (or click a tab)"),
    ("t / T", "next / previous theme"),
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
    ("/ e", "particles: edit query (tab completes)"),
    ("x X  y Y", "heatmap axes"),
    ("m  ←→ f F", "bin mode, aggregated field"),
    ("c / o / z", "colormap, log scale, zoom heatmap"),
    ("click heatmap", "drill into bin (⌫ pops, r resets)"),
    ("click header / s", "sort table by column / heat field"),
    ("a / enter", "live refresh, refresh now"),
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
