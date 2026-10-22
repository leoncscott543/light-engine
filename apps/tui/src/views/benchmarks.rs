use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Gauge, Paragraph};

use super::charts;
use crate::action::Action;
use crate::app::{App, BenchRecord, RunState};
use crate::fmt;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, Button, hovered, panel};

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [list, detail] =
        Layout::horizontal([Constraint::Length(46), Constraint::Min(0)]).areas(area);
    bench_list(app, frame, list, hits);

    let Some(desc) = app.bench.selected() else {
        let block = panel("Details", false);
        let inner = block.inner(detail);
        frame.render_widget(block, detail);
        widgets::empty_state(
            frame,
            inner,
            "no benchmarks registered",
            "register workloads in light_bench::Registry",
        );
        return;
    };
    let record = app.bench.records.get(&desc.id);

    let [head, progress, samples, bottom] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(6),
    ])
    .areas(detail);

    header_panel(app, frame, head, hits);
    progress_panel(record, frame, progress);

    let values: Vec<f64> = record
        .map(|r| r.samples.iter().map(|d| d.as_secs_f64() * 1e6).collect())
        .unwrap_or_default();
    charts::sparkline(
        frame,
        samples,
        "Samples",
        &values,
        theme::HONEY,
        |us| fmt::duration(Duration::from_secs_f64(us * 1e-6)),
        false,
    );

    let [hist, stats] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(bottom);
    histogram(record, frame, hist);
    stats_panel(record, frame, stats);
}

fn bench_list(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let title = format!("Benchmarks ({})", app.bench.list.len());
    let block = panel(&title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.bench.list.is_empty() {
        widgets::empty_state(frame, inner, "nothing registered", "");
        return;
    }

    let height = inner.height as usize;
    let offset = widgets::scroll_offset(app.bench.selected, height);
    for (row, (i, desc)) in app
        .bench
        .list
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .enumerate()
    {
        let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let style = if i == app.bench.selected {
            theme::selected()
        } else if hovered(app.mouse, rect) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            theme::base()
        };
        let record = app.bench.records.get(&desc.id);
        let badge = match record {
            Some(r) if r.state == RunState::Running => {
                format!("{:>3.0}%", r.progress() * 100.0)
            }
            Some(BenchRecord { stats: Some(s), .. }) => fmt::duration(s.mean),
            Some(r) if r.state == RunState::Cancelled => "cancel".into(),
            Some(r) if r.state == RunState::Failed => "failed".into(),
            _ => String::new(),
        };
        let name = desc
            .id
            .0
            .split_once('/')
            .map_or(desc.id.0.as_str(), |(_, n)| n);
        let badge_w = 9usize;
        let name_w = (inner.width as usize).saturating_sub(badge_w + 10);
        let marker = if app.bench.active.as_ref() == Some(&desc.id) {
            "▶"
        } else {
            " "
        };
        let line = Line::from(vec![
            Span::styled(marker, style.fg(theme::OK)),
            Span::styled(format!("{:<8} ", desc.group), style.fg(theme::TEXT_DIM)),
            Span::styled(format!("{name:<name_w$.name_w$}"), style),
            Span::styled(format!("{badge:>badge_w$}"), style.fg(theme::GOLD)),
        ]);
        frame.render_widget(Paragraph::new(line).style(style), rect);
        hits.push(rect, Action::SelectRow(i));
    }
}

fn header_panel(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let Some(desc) = app.bench.selected() else {
        return;
    };
    let block = panel(&desc.id.0, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [info, _, config, _, buttons] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(desc.description.clone(), theme::base()),
            widgets::kv(
                "group ",
                Span::styled(desc.group.clone(), theme::value()),
                6,
            ),
        ]),
        info,
    );

    let c = app.bench.config;
    let mut x = config.x;
    for (label, value, action) in [
        (
            "samples",
            c.samples,
            Action::AdjustSamples as fn(i32) -> Action,
        ),
        ("warmup", c.warmup_iters, Action::AdjustWarmup),
        (
            "iters/sample",
            c.iters_per_sample,
            Action::AdjustItersPerSample,
        ),
    ] {
        x = stepper(app, frame, Rect { x, ..config }, label, value, action, hits) + 2;
    }

    let running = app.bench.active.is_some();
    let busy_other = running && app.bench.active.as_ref() != Some(&desc.id);
    widgets::button_row(
        frame,
        buttons,
        &[
            Button::new("▶ Run", Action::RunBench)
                .key("enter")
                .primary()
                .enabled(!running),
            Button::new("Cancel", Action::CancelBench)
                .key("x")
                .danger()
                .enabled(running),
        ],
        app.mouse,
        hits,
    );
    if busy_other {
        let note = "another benchmark is running";
        let w = note.len() as u16;
        if buttons.width > w + 24 {
            frame.render_widget(
                Paragraph::new(Span::styled(note, Style::new().fg(theme::WARN))),
                Rect::new(buttons.right() - w, buttons.y, w, 1),
            );
        }
    }
}

/// `label ◂ value ▸` with clickable arrows. Returns the x just past it.
fn stepper(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: u32,
    action: fn(i32) -> Action,
    hits: &mut HitMap,
) -> u16 {
    let label_s = format!("{label} ");
    let value_s = format!("{value:^6}");
    let lw = label_s.chars().count() as u16;
    let vw = value_s.chars().count() as u16;
    if area.x + lw + vw + 4 > area.right() {
        return area.right();
    }
    let y = area.y;
    let dec = Rect::new(area.x + lw, y, 2, 1);
    let val = Rect::new(dec.right(), y, vw, 1);
    let inc = Rect::new(val.right(), y, 2, 1);
    let arrow = |r: Rect| {
        if hovered(app.mouse, r) {
            Style::new().bg(theme::ACCENT).fg(theme::TEXT)
        } else {
            Style::new().bg(theme::SURFACE).fg(theme::GOLD)
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(label_s, theme::label())),
        Rect::new(area.x, y, lw, 1),
    );
    frame.render_widget(Paragraph::new(Span::styled(" ◂", arrow(dec))), dec);
    frame.render_widget(
        Paragraph::new(Span::styled(
            value_s,
            Style::new().bg(theme::SURFACE).fg(theme::TEXT),
        )),
        val,
    );
    frame.render_widget(Paragraph::new(Span::styled("▸ ", arrow(inc))), inc);
    hits.push(dec, action(-1));
    hits.push(inc, action(1));
    inc.right()
}

fn progress_panel(record: Option<&BenchRecord>, frame: &mut Frame, area: Rect) {
    let block = panel("Progress", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (ratio, label) = match record {
        None => (0.0, "not run yet".to_string()),
        Some(r) => {
            let state = match r.state {
                RunState::Running if r.samples.is_empty() => "warming up".to_string(),
                RunState::Running => "running".into(),
                RunState::Finished => "finished".into(),
                RunState::Cancelled => "cancelled".into(),
                RunState::Failed => {
                    format!("failed: {}", r.error.as_deref().unwrap_or("unknown error"))
                }
            };
            (
                r.progress(),
                format!("{state} · {}/{} samples", r.samples.len(), r.config.samples),
            )
        }
    };
    frame.render_widget(
        Gauge::default()
            .gauge_style(Style::new().fg(theme::ACCENT).bg(theme::SURFACE))
            .ratio(ratio)
            .label(Span::styled(label, theme::value())),
        inner,
    );
}

fn histogram(record: Option<&BenchRecord>, frame: &mut Frame, area: Rect) {
    let block = panel("Distribution", false);
    let inner = block.inner(area);
    let samples = record.map_or(&[][..], |r| r.samples.as_slice());
    if samples.len() < 2 || inner.width < 8 {
        frame.render_widget(block, area);
        widgets::empty_state(frame, inner, "no distribution yet", "run the benchmark");
        return;
    }

    let ns: Vec<f64> = samples.iter().map(|d| d.as_nanos() as f64).collect();
    let (lo, hi) = ns
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let bar_w = 3u16;
    let bins = ((inner.width / (bar_w + 1)) as usize).clamp(1, 24);
    let span = (hi - lo).max(1.0);
    let mut counts = vec![0u64; bins];
    for v in &ns {
        let b = (((v - lo) / span) * bins as f64) as usize;
        counts[b.min(bins - 1)] += 1;
    }
    let bars: Vec<Bar> = counts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let label = if i == 0 || i == bins - 1 || i == bins / 2 {
                let at = lo + span * (i as f64 + 0.5) / bins as f64;
                fmt::duration(Duration::from_nanos(at as u64))
            } else {
                String::new()
            };
            Bar::new(*c)
                .label(Line::styled(label, theme::faint()))
                .style(Style::new().fg(theme::HONEY))
                .value_style(Style::new().fg(theme::TEXT).bg(theme::HONEY))
        })
        .collect();
    frame.render_widget(
        BarChart::default()
            .block(block)
            .bar_width(bar_w)
            .bar_gap(1)
            .data(BarGroup::default().bars(&bars)),
        area,
    );
}

fn stats_panel(record: Option<&BenchRecord>, frame: &mut Frame, area: Rect) {
    let block = panel("Statistics", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let stats = record.and_then(|r| {
        r.stats
            .or_else(|| light_contracts::BenchStats::from_samples(&r.samples))
    });
    let Some(s) = stats else {
        widgets::empty_state(frame, inner, "no statistics", "");
        return;
    };
    let w = 8;
    let d = |v| Span::styled(fmt::duration(v), theme::value());
    let cv = if s.mean.is_zero() {
        0.0
    } else {
        s.stddev.as_secs_f64() / s.mean.as_secs_f64() * 100.0
    };
    let lines = vec![
        widgets::kv("n", Span::styled(s.count.to_string(), theme::value()), w),
        widgets::kv("mean", d(s.mean), w),
        widgets::kv(
            "σ",
            Span::styled(
                format!("{} ({cv:.1}%)", fmt::duration(s.stddev)),
                theme::value(),
            ),
            w,
        ),
        widgets::kv("min", d(s.min), w),
        widgets::kv("p50", d(s.p50), w),
        widgets::kv("p95", d(s.p95), w),
        widgets::kv("p99", d(s.p99), w),
        widgets::kv("max", d(s.max), w),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}
