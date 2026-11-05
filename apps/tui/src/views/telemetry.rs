//! Inspection surface for evidence, timing, and transport pressure.
use crate::{
    app::{App, p95},
    fmt,
    hit::HitMap,
    theme, widgets,
};
use light_contracts::ObservationMeta;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

fn item(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(widgets::fixed(label, 25, false), theme::label()),
        Span::styled(widgets::clean(&value.into()), theme::value()),
    ])
}

fn metadata(lines: &mut Vec<Line<'static>>, title: &str, meta: &ObservationMeta) {
    lines.push(Line::styled(
        format!(
            " {title} · {} · schema {}",
            meta.label(),
            meta.schema_version
        ),
        theme::title(),
    ));
    let p = &meta.provenance;
    lines.push(item(
        "producer / version",
        format!(
            "{} / {}",
            p.producer,
            if p.producer_version.is_empty() {
                "unspecified"
            } else {
                &p.producer_version
            }
        ),
    ));
    lines.push(item(
        "run / sequence",
        format!(
            "{} / {}",
            p.run_id.as_deref().unwrap_or("unspecified"),
            meta.sequence
        ),
    ));
    lines.push(item("model", p.model.as_deref().unwrap_or("unspecified")));
    lines.push(item(
        "seed",
        p.seed
            .map_or("unspecified".into(), |seed| format!("{seed:#x}")),
    ));
    lines.push(item(
        "precision / units",
        format!("{} / {}", meta.precision, meta.unit_system),
    ));
    lines.push(item("validation", meta.validation.clone()));
    let timestamp = meta
        .captured_at
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or("unspecified".into(), |t| {
            format!("{}.{:03} UTC unix seconds", t.as_secs(), t.subsec_millis())
        });
    lines.push(item("producer capture time", timestamp));
    lines.push(item(
        "producer uptime",
        meta.producer_uptime
            .map_or("unspecified".into(), fmt::uptime),
    ));
    lines.push(item(
        "expected interval",
        meta.expected_interval
            .map_or("unspecified".into(), fmt::duration),
    ));
    if !p.lineage.is_empty() {
        lines.push(item("lineage", p.lineage.join(", ")));
    }
    lines.push(Line::styled(
        " FIELD                       ORIGIN   QUALITY       UNIT",
        theme::title(),
    ));
    lines.push(Line::from(vec![
        Span::styled(
            widgets::fixed(" (payload default)", 29, false),
            theme::dim(),
        ),
        Span::styled(
            format!("{:<9} {:<13}", p.origin.to_string(), p.quality.to_string()),
            theme::value(),
        ),
    ]));
    for field in &meta.fields {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {}", widgets::fixed(&field.path, 28, false)),
                theme::dim(),
            ),
            Span::styled(
                widgets::fixed(&field.provenance.origin.to_string(), 9, false),
                theme::title(),
            ),
            Span::styled(
                widgets::fixed(&field.provenance.quality.to_string(), 14, false),
                theme::value(),
            ),
            Span::styled(field.unit.clone(), theme::dim()),
        ]));
        if let Some(uncertainty) = &field.uncertainty {
            lines.push(item("  absolute uncertainty", uncertainty.clone()));
        }
    }
    lines.push(Line::raw(""));
}

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(" ACQUISITION & DISPLAY", theme::title()),
        item(
            "connection",
            app.connection_error.as_ref().map_or_else(
                || {
                    if app.backend.name == "connecting" {
                        "connecting".into()
                    } else {
                        "ready".into()
                    }
                },
                |err| format!("FAULTED · {err}"),
            ),
        ),
        item(
            "render FPS / target",
            format!("{:6.1} / {:3} Hz", app.ui.fps(), app.ui.target_fps),
        ),
        item(
            "render cost / p95",
            format!(
                "{:6.2} / {:6.2} ms",
                app.ui.render_ms.latest().unwrap_or(0.0),
                p95(&app.ui.render_ms)
            ),
        ),
        item(
            "frame interval p95",
            format!(
                "{:6.2} ms · {} draws over budget",
                p95(&app.ui.frame_interval_ms),
                app.ui.over_budget
            ),
        ),
        item(
            "snapshot receipt",
            format!(
                "{:6.1} Hz · {} observations",
                app.snapshot_feed.hz(),
                app.snapshot_feed.received
            ),
        ),
        item(
            "snapshot interval p95",
            format!("{:6.2} ms", p95(&app.snapshot_feed.intervals_ms)),
        ),
        item(
            "snapshot age / state",
            format!(
                "{} / {}",
                app.snapshot_feed
                    .last_received
                    .map_or("—".into(), |t| fmt::duration(t.elapsed())),
                app.snapshot_health()
            ),
        ),
        item(
            "particle acquisition",
            format!(
                "{} · {} · {:5.1} Hz",
                if app.explorer.live { "auto" } else { "manual" },
                if app.explorer.loading {
                    "query pending"
                } else {
                    "ready"
                },
                app.particle_feed.hz()
            ),
        ),
        item(
            "particle frame age",
            app.particle_feed
                .last_received
                .map_or("—".into(), |t| fmt::duration(t.elapsed())),
        ),
    ];
    if let Some(d) = app.delivery {
        lines.push(item(
            "snapshot delivery cap",
            format!(
                "{:.1} Hz · worker {}",
                1.0 / d.snapshot_interval.as_secs_f64().max(f64::MIN_POSITIVE),
                fmt::uptime(d.worker_uptime)
            ),
        ));
        lines.push(item(
            "produced / delivered",
            format!("{} / {}", d.snapshots_produced, d.snapshots_delivered),
        ));
        lines.push(item(
            "coalesced / dropped",
            format!(
                "{} / {} notifications",
                d.snapshots_coalesced, d.notifications_dropped
            ),
        ));
        lines.push(item(
            "delivery queue",
            format!("{} / {}", d.queue_depth, d.queue_capacity),
        ));
    }
    lines.push(item(
        "query response / p95",
        format!(
            "{:6.2} / {:6.2} ms · {} obsolete results",
            app.query_response_ms.latest().unwrap_or(0.0),
            p95(&app.query_response_ms),
            app.discarded_particle_frames
        ),
    ));
    lines.push(item(
        "command batch / p95",
        format!(
            "{:6.2} / {:6.2} ms",
            app.command_response_ms.latest().unwrap_or(0.0),
            p95(&app.command_response_ms)
        ),
    ));
    if let Some(particles) = &app.explorer.frame {
        lines.push(item(
            "query execution cost",
            format!(
                "{} · {}",
                fmt::duration(particles.elapsed),
                widgets::evidence(&particles.meta, "elapsed")
            ),
        ));
    }
    lines.push(Line::styled(
        " Rates use local monotonic receipt times; UTC capture times do not imply network latency.",
        theme::dim(),
    ));
    lines.push(Line::raw(""));
    if let Some(snapshot) = &app.sim.snapshot {
        metadata(&mut lines, "SIMULATION", &snapshot.meta);
    }
    if let Some(particles) = &app.explorer.frame {
        metadata(&mut lines, "PARTICLE FRAME", &particles.meta);
    }
    if let Some(bench) = app.bench.selected() {
        metadata(&mut lines, "SELECTED BENCHMARK", &bench.meta);
        if let Some(run) = app.bench.records.get(&bench.id) {
            metadata(&mut lines, "BENCHMARK RUN", &run.meta);
        }
    }
    if let Some(process) = app.procs.list.get(app.procs.selected) {
        metadata(&mut lines, "SELECTED PROCESS", &process.meta);
    }
    lines.push(Line::styled(
        " Viewer observations are not restart checkpoints. UNKNOWN never implies LIVE.",
        theme::dim(),
    ));
    lines
}

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let block = widgets::panel(
        "Telemetry · source evidence & cadence · ↑↓ / PgUp PgDn",
        true,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = lines(app);
    app.telemetry_height.set(inner.height as usize);
    let offset = app
        .telemetry_scroll
        .min(lines.len().saturating_sub(inner.height as usize));
    widgets::scrollbar(frame, area, lines.len(), offset, inner.height as usize);
    widgets::scrollbar_hits(area, lines.len(), inner.height as usize, hits);
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .skip(offset)
                .take(inner.height as usize)
                .collect::<Vec<_>>(),
        ),
        inner,
    );
}
