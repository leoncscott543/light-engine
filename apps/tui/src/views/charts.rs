use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Sparkline};

use crate::app::App;
use crate::fmt;
use crate::theme;
use crate::widgets::{self, panel};

/// Relative energy error over ticks: the primary invariant health signal.
pub fn energy(app: &App, frame: &mut Frame, area: Rect, focused: bool) {
    let points: Vec<(f64, f64)> = app.sim.energy.iter().collect();
    let latest = points.last().map_or(0.0, |p| p.1);
    let block = panel("Energy drift |H-H₀|/|H₀|", focused).title(
        Line::from(Span::styled(
            format!(" {} ", fmt::sci(latest)),
            theme::energy_error(latest),
        ))
        .right_aligned(),
    );

    if points.len() < 2 {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let hint = if app.has_session() {
            "start or step the simulation to collect samples"
        } else {
            "no active session · press n or space"
        };
        widgets::empty_state(frame, inner, "no samples yet", hint);
        return;
    }

    let (x0, x1) = (
        points[0].0,
        points[points.len() - 1].0.max(points[0].0 + 1.0),
    );
    let y_max = points.iter().map(|p| p.1).fold(0.0, f64::max).max(1e-16) * 1.15;
    let axis_style = Style::new().fg(theme::TEXT_DIM);
    let chart = Chart::new(vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().fg(theme::GOLD))
            .data(&points),
    ])
    .block(block)
    .style(theme::base())
    .x_axis(
        Axis::default()
            .style(axis_style)
            .bounds([x0, x1])
            .labels(vec![fmt::count(x0 as u64), fmt::count(x1 as u64)]),
    )
    .y_axis(
        Axis::default()
            .style(axis_style)
            .bounds([0.0, y_max])
            .labels(vec!["0".to_string(), fmt::sci(y_max)]),
    );
    frame.render_widget(chart, area);
}

/// Sparkline over the newest values that fit, with latest/peak in the title.
pub fn sparkline(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    values: &[f64],
    color: Color,
    format: impl Fn(f64) -> String,
    focused: bool,
) {
    let latest = values.last().copied();
    let peak = values.iter().copied().fold(f64::NAN, f64::max);
    let summary = match latest {
        Some(v) => format!(" {} · peak {} ", format(v), format(peak)),
        None => " — ".into(),
    };
    let block = panel(title, focused)
        .title(Line::from(Span::styled(summary, theme::dim())).right_aligned());
    let inner = block.inner(area);
    if values.is_empty() {
        frame.render_widget(block, area);
        widgets::empty_state(frame, inner, "no data", "");
        return;
    }
    let width = inner.width as usize;
    let tail = &values[values.len().saturating_sub(width)..];
    let max = peak.max(f64::MIN_POSITIVE);
    let data: Vec<u64> = tail.iter().map(|v| (v / max * 1000.0) as u64).collect();
    frame.render_widget(
        Sparkline::default()
            .block(block)
            .style(Style::new().fg(color).bg(theme::BG))
            .max(1000)
            .data(data),
        area,
    );
}
