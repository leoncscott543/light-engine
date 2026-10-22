use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use super::charts;
use crate::action::Action;
use crate::app::{App, RunState, Tab};
use crate::fmt;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, Button, panel};

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [cards, charts_row, bottom] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(9),
    ])
    .areas(area);

    stat_cards(app, frame, cards, hits);

    let [energy, step] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .areas(charts_row);
    charts::energy(app, frame, energy, false);
    hits.push(energy, Action::GoTo(Tab::Simulation));
    let step_us: Vec<f64> = app.sim.step_us.iter().collect();
    charts::sparkline(
        frame,
        step,
        "Step time",
        &step_us,
        theme::HONEY,
        |us| fmt::duration(std::time::Duration::from_secs_f64(us * 1e-6)),
        false,
    );

    let [bench, system] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .areas(bottom);
    bench_summary(app, frame, bench, hits);
    system_panel(app, frame, system);
}

fn stat_cards(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let cols: [Rect; 4] = Layout::horizontal([Constraint::Ratio(1, 4); 4]).areas(area);
    let snap = app.sim.snapshot.as_ref();
    let status = app.sim.status();
    let none = || Line::styled("—", theme::faint());
    let go_sim = Action::GoTo(Tab::Simulation);

    widgets::stat_card(
        frame,
        cols[0],
        "Session",
        Line::styled(status.to_string(), theme::sim_status(&status)),
        snap.map_or_else(
            || Line::styled("press n or space to create", theme::dim()),
            |s| {
                Line::styled(
                    format!("{} · {}", s.config.integrator, s.config.layout),
                    theme::dim(),
                )
            },
        ),
        go_sim,
        app.mouse,
        hits,
    );
    widgets::stat_card(
        frame,
        cols[1],
        "Tick",
        snap.map_or_else(none, |s| Line::styled(fmt::count(s.tick), theme::value())),
        snap.map_or_else(none, |s| {
            Line::styled(format!("t = {:.4}", s.sim_time), theme::dim())
        }),
        go_sim,
        app.mouse,
        hits,
    );
    widgets::stat_card(
        frame,
        cols[2],
        "Throughput",
        snap.map_or_else(none, |s| {
            Line::styled(fmt::rate(s.perf.particles_per_sec), theme::value())
        }),
        snap.map_or_else(none, |s| {
            Line::styled(
                format!("{} particles", fmt::count(u64::from(s.config.particle_count))),
                theme::dim(),
            )
        }),
        go_sim,
        app.mouse,
        hits,
    );
    widgets::stat_card(
        frame,
        cols[3],
        "Energy error",
        snap.map_or_else(none, |s| {
            let e = s.invariants.relative_energy_error();
            Line::styled(fmt::sci(e), theme::energy_error(e))
        }),
        snap.map_or_else(none, |s| {
            Line::styled(
                format!("|p| {}", fmt::sci(s.invariants.momentum_magnitude())),
                theme::dim(),
            )
        }),
        go_sim,
        app.mouse,
        hits,
    );
}

fn bench_summary(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let block = panel("Benchmarks", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    hits.push(area, Action::GoTo(Tab::Benchmarks));

    let [info, gauge, _, buttons] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let selected = app.bench.selected();
    let shown = app.bench.active.as_ref().or(selected.map(|d| &d.id));
    let record = shown.and_then(|id| app.bench.records.get(id));

    let mut lines = vec![widgets::kv(
        "registered",
        Span::styled(app.bench.list.len().to_string(), theme::value()),
        12,
    )];
    if let Some(id) = shown {
        lines.push(widgets::kv("benchmark", Span::styled(id.to_string(), theme::value()), 12));
    }
    if let Some(stats) = record.and_then(|r| r.stats) {
        lines.push(widgets::kv(
            "mean / p99",
            Span::styled(
                format!("{} / {}", fmt::duration(stats.mean), fmt::duration(stats.p99)),
                theme::value(),
            ),
            12,
        ));
    }
    frame.render_widget(Paragraph::new(lines), info);

    if let Some(r) = record {
        let label = match r.state {
            RunState::Running => format!("{}/{} samples", r.samples.len(), r.config.samples),
            RunState::Finished => "finished".into(),
            RunState::Cancelled => "cancelled".into(),
        };
        frame.render_widget(
            Gauge::default()
                .gauge_style(ratatui::style::Style::new().fg(theme::ACCENT).bg(theme::SURFACE))
                .ratio(r.progress())
                .label(Span::styled(label, theme::value())),
            gauge,
        );
    }

    let running = app.bench.active.is_some();
    widgets::button_row(
        frame,
        buttons,
        &[
            Button::new("Run selected", Action::RunBench)
                .key("enter")
                .primary()
                .enabled(!running && selected.is_some()),
            Button::new("Cancel", Action::CancelBench)
                .key("x")
                .danger()
                .enabled(running),
            Button::new("Open", Action::GoTo(Tab::Benchmarks)).key("3"),
        ],
        app.mouse,
        hits,
    );
}

fn system_panel(app: &App, frame: &mut Frame, area: Rect) {
    let block = panel("Dashboard", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [text, spark] =
        Layout::vertical([Constraint::Length(4), Constraint::Min(1)]).areas(inner);

    let render_ms = app.ui.render_ms.latest().unwrap_or(0.0);
    let lines = vec![
        widgets::kv(
            "backend",
            Span::styled(format!("{} ({})", app.backend.name, app.backend.kind), theme::value()),
            10,
        ),
        widgets::kv("uptime", Span::styled(fmt::uptime(app.uptime()), theme::value()), 10),
        widgets::kv("fps", Span::styled(format!("{:.0}", app.ui.fps()), theme::value()), 10),
        widgets::kv("render", Span::styled(format!("{render_ms:.2} ms"), theme::value()), 10),
    ];
    frame.render_widget(Paragraph::new(lines), text);

    let values: Vec<f64> = app.ui.render_ms.iter().collect();
    let width = spark.width as usize;
    let tail = &values[values.len().saturating_sub(width)..];
    let max = tail.iter().copied().fold(f64::MIN_POSITIVE, f64::max);
    frame.render_widget(
        ratatui::widgets::Sparkline::default()
            .style(ratatui::style::Style::new().fg(theme::BORDER_FOCUS))
            .max(1000)
            .data(tail.iter().map(|v| (v / max * 1000.0) as u64).collect::<Vec<_>>()),
        spark,
    );
}
