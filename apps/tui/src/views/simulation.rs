use std::time::Duration;

use light_contracts::SimStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use super::charts;
use crate::action::Action;
use crate::app::{App, ConfigField};
use crate::fmt;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, Button, hovered, panel};

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(44), Constraint::Min(0)]).areas(area);
    let [session, config] =
        Layout::vertical([Constraint::Length(9), Constraint::Min(0)]).areas(left);
    session_panel(app, frame, session, hits);
    config_panel(app, frame, config, hits);

    let [tables, energy, sparks] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Min(6),
        Constraint::Length(7),
    ])
    .areas(right);
    let [inv, perf] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(tables);
    invariants_panel(app, frame, inv);
    perf_panel(app, frame, perf);
    charts::energy(app, frame, energy, true);

    let [step, thru] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(sparks);
    let step_us: Vec<f64> = app.sim.step_us.iter().collect();
    charts::sparkline(
        frame,
        step,
        "Step time",
        &step_us,
        theme::HONEY,
        |us| fmt::duration(Duration::from_secs_f64(us * 1e-6)),
        false,
    );
    let throughput: Vec<f64> = app.sim.throughput.iter().collect();
    charts::sparkline(
        frame,
        thru,
        "Throughput",
        &throughput,
        theme::GOLD,
        fmt::rate,
        false,
    );
}

fn session_panel(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let block = panel("Session", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [status, _, buttons] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    let state = app.sim.status();
    let detail = match &app.sim.snapshot {
        Some(s) => format!("  tick {} · t={:.4}", fmt::count(s.tick), s.sim_time),
        None => "  no session".into(),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ", theme::sim_status(&state)),
            Span::styled(state.to_string(), theme::sim_status(&state)),
            Span::styled(detail, theme::dim()),
        ])),
        status,
    );

    let has = app.has_session();
    let running = state == SimStatus::Running;
    let pending = !app.sim.pending_fields().is_empty();
    let run_button = if running {
        Button::new("⏸ Pause", Action::ToggleRun)
            .key("space")
            .primary()
    } else {
        Button::new("▶ Start", Action::ToggleRun)
            .key("space")
            .primary()
    };
    let lifecycle = if has {
        Button::new("Destroy", Action::DestroySession)
            .key("D")
            .danger()
    } else {
        Button::new("Create", Action::CreateSession).key("n")
    };
    widgets::button_row(
        frame,
        buttons,
        &[
            run_button,
            Button::new("Step", Action::Step(1))
                .key(".")
                .enabled(has && !running),
            Button::new("+100", Action::Step(100))
                .key(">")
                .enabled(has && !running),
            lifecycle,
            Button::new("Reset", Action::ResetSession)
                .key("r")
                .enabled(has),
            Button::new("Recreate", Action::RecreateSession)
                .key("R")
                .enabled(has && pending),
        ],
        app.mouse,
        hits,
    );
}

fn config_panel(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let title = if app.has_session() {
        "Config"
    } else {
        "Config (draft)"
    };
    let block = panel(title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let pending = app.sim.pending_fields();
    for (i, field) in ConfigField::ALL.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.bottom() {
            break;
        }
        let row = Rect::new(inner.x, y, inner.width, 1);
        let selected = *field == app.sim.field;
        let row_style = if selected {
            theme::selected()
        } else if hovered(app.mouse, row) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            theme::base()
        };
        frame.render_widget(Paragraph::new("").style(row_style), row);
        hits.push(row, Action::FocusField(*field));

        let marker = if pending.contains(field) { "•" } else { " " };
        let live = if field.is_live() && app.has_session() {
            "live"
        } else {
            ""
        };
        let label = format!("{marker} {:<12}", field.label());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(label, row_style.fg(theme::TEXT_DIM)),
                Span::styled(format!("{live:<5}"), row_style.fg(theme::OK)),
            ])),
            row,
        );

        // ◂ value ▸ with clickable arrows
        let value = field.display(&app.sim.draft);
        let value_w = 18u16;
        let x = row.right().saturating_sub(value_w + 4);
        if x <= row.x + 19 {
            continue;
        }
        let dec = Rect::new(x, y, 2, 1);
        let inc = Rect::new(x + 2 + value_w, y, 2, 1);
        let arrow = |r: Rect| {
            if hovered(app.mouse, r) {
                row_style.fg(theme::BORDER_FOCUS).bg(theme::ACCENT)
            } else {
                row_style.fg(theme::GOLD)
            }
        };
        frame.render_widget(Paragraph::new(Span::styled(" ◂", arrow(dec))), dec);
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{value:^w$}", w = value_w as usize),
                row_style,
            ))
            .style(row_style),
            Rect::new(x + 2, y, value_w, 1),
        );
        frame.render_widget(Paragraph::new(Span::styled("▸ ", arrow(inc))), inc);
        hits.push(dec, Action::AdjustField(*field, -1));
        hits.push(inc, Action::AdjustField(*field, 1));
    }

    let hint_y = inner.y + ConfigField::ALL.len() as u16 + 1;
    if hint_y < inner.bottom() {
        let hint = if !pending.is_empty() {
            Line::styled(
                "• changed — Recreate (R) to apply",
                Style::new().fg(theme::WARN),
            )
        } else if app.has_session() {
            Line::styled("dt is live · others need R", theme::faint())
        } else {
            Line::styled("↑↓ select · ←→ adjust · n create", theme::faint())
        };
        frame.render_widget(
            Paragraph::new(hint),
            Rect::new(inner.x + 1, hint_y, inner.width.saturating_sub(1), 1),
        );
    }
}

fn invariants_panel(app: &App, frame: &mut Frame, area: Rect) {
    let block = panel("Invariants", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(s) = &app.sim.snapshot else {
        widgets::empty_state(frame, inner, "no session", "invariants appear once created");
        return;
    };
    let inv = &s.invariants;
    let rel = inv.relative_energy_error();
    let l = inv.angular_momentum;
    let w = 14;
    let lines = vec![
        widgets::kv(
            "H",
            Span::styled(format!("{:.6e}", inv.energy), theme::value()),
            w,
        ),
        widgets::kv(
            "H₀",
            Span::styled(format!("{:.6e}", inv.initial_energy), theme::value()),
            w,
        ),
        widgets::kv(
            "|ΔH|/|H₀|",
            Span::styled(fmt::sci(rel), theme::energy_error(rel)),
            w,
        ),
        widgets::kv(
            "|p|",
            Span::styled(fmt::sci(inv.momentum_magnitude()), theme::value()),
            w,
        ),
        widgets::kv(
            "L",
            Span::styled(
                format!("[{:.3}, {:.3}, {:.3}]", l[0], l[1], l[2]),
                theme::value(),
            ),
            w,
        ),
        widgets::kv(
            "ω error",
            Span::styled(fmt::sci(inv.symplectic_error), theme::value()),
            w,
        ),
        widgets::kv(
            "symplectic",
            Span::styled(
                if s.config.integrator.is_symplectic() {
                    "yes"
                } else {
                    "no"
                },
                theme::value(),
            ),
            w,
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn perf_panel(app: &App, frame: &mut Frame, area: Rect) {
    let block = panel("Performance", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(s) = &app.sim.snapshot else {
        widgets::empty_state(
            frame,
            inner,
            "no session",
            "performance appears once created",
        );
        return;
    };
    let p = &s.perf;
    let w = 14;
    let [text, _, gauge_label, gauge] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let lines = vec![
        widgets::kv(
            "step",
            Span::styled(fmt::duration(p.step_time), theme::value()),
            w,
        ),
        widgets::kv(
            "throughput",
            Span::styled(fmt::rate(p.particles_per_sec), theme::value()),
            w,
        ),
        widgets::kv(
            "memory",
            Span::styled(fmt::bytes(p.memory_bytes), theme::value()),
            w,
        ),
        widgets::kv(
            "layout",
            Span::styled(s.config.layout.to_string(), theme::value()),
            w,
        ),
    ];
    frame.render_widget(Paragraph::new(lines), text);
    frame.render_widget(
        Paragraph::new(Span::styled("SIMD lane utilization", theme::label())),
        gauge_label,
    );
    frame.render_widget(
        Gauge::default()
            .gauge_style(Style::new().fg(theme::ACCENT).bg(theme::SURFACE))
            .ratio(p.lane_utilization.clamp(0.0, 1.0))
            .label(Span::styled(
                format!("{:.1}%", p.lane_utilization * 100.0),
                theme::value(),
            )),
        gauge,
    );
}
