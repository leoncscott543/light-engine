use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::charts;
use crate::action::Action;
use crate::app::App;
use crate::fmt;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, hovered, panel};

const COLUMNS: [(&str, usize); 8] = [
    ("PID", 5),
    ("NAME", 16),
    ("KIND", 11),
    ("SOURCE", 8),
    ("STATE", 10),
    ("CPU", 18),
    ("MEMORY", 11),
    ("UPTIME", 8),
];

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [table, detail] = Layout::vertical([Constraint::Min(5), Constraint::Length(8)]).areas(area);
    process_table(app, frame, table, hits);

    let Some(p) = app.procs.list.get(app.procs.selected) else {
        let block = panel("Process", false);
        let inner = block.inner(detail);
        frame.render_widget(block, detail);
        widgets::empty_state(frame, inner, "no processes", "the backend reports none");
        return;
    };
    let history: Vec<f64> = app
        .procs
        .cpu_history
        .get(&(p.meta.provenance.producer.clone(), p.id))
        .map(|s| s.iter().map(|c| c * 100.0).collect())
        .unwrap_or_default();
    let title = format!(
        "{} · CPU history · {}",
        p.name,
        widgets::evidence(&p.meta, "cpu")
    );
    charts::sparkline(
        frame,
        detail,
        &title,
        &history,
        theme::palette().honey,
        |v| format!("{v:.0}%"),
        false,
    );
}

fn row_text(cells: [String; 8]) -> String {
    cells
        .iter()
        .zip(COLUMNS)
        .map(|(c, (_, w))| format!("{} ", widgets::fixed(c, w, false)))
        .collect()
}

fn process_table(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let title = format!("Processes ({})", app.procs.list.len());
    let block = panel(&title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let header = row_text(COLUMNS.map(|(h, _)| h.to_string()));
    frame.render_widget(
        Paragraph::new(Span::styled(header, theme::title())),
        Rect { height: 1, ..inner },
    );

    let body = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
    let height = body.height as usize;
    let offset = app
        .procs
        .viewport
        .range(app.procs.selected, app.procs.list.len(), height)
        .start;
    widgets::scrollbar(frame, area, app.procs.list.len(), offset, height);
    widgets::scrollbar_hits(area, app.procs.list.len(), height, hits);
    for (row, (i, p)) in app
        .procs
        .list
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .enumerate()
    {
        let rect = Rect::new(body.x, body.y + row as u16, body.width, 1);
        let style = if i == app.procs.selected {
            theme::selected()
        } else if hovered(app.mouse, rect) {
            Style::new()
                .bg(theme::palette().hover)
                .fg(theme::palette().text)
        } else {
            theme::base()
        };
        let cpu = if p.meta.field("cpu").quality == light_contracts::DataQuality::Unavailable {
            "unavailable".into()
        } else {
            format!(
                "{} {:>4.0}%",
                widgets::bar(p.cpu.min(1.0), 10),
                p.cpu * 100.0
            )
        };
        let pre = [
            p.id.to_string(),
            p.name.clone(),
            p.kind.to_string(),
            p.meta.label().into(),
        ]
        .iter()
        .zip(COLUMNS)
        .map(|(c, (_, w))| format!("{} ", widgets::fixed(c, w, false)))
        .collect::<String>();
        // Split around the state column so it can carry its own color.
        let (w_state, w_cpu, w_mem) = (COLUMNS[4].1, COLUMNS[5].1, COLUMNS[6].1);
        let line = Line::from(vec![
            Span::styled(pre, style),
            Span::styled(
                format!("{} ", widgets::fixed(&p.state.to_string(), w_state, false)),
                theme::process_state(&p.state).bg(style.bg.unwrap_or(theme::palette().bg)),
            ),
            Span::styled(
                format!("{} ", widgets::fixed(&cpu, w_cpu, false)),
                style.fg(theme::palette().honey),
            ),
            Span::styled(
                format!(
                    "{} ",
                    widgets::fixed(
                        &if p.meta.field("memory_bytes").quality
                            == light_contracts::DataQuality::Unavailable
                        {
                            "—".into()
                        } else {
                            fmt::bytes(p.memory_bytes)
                        },
                        w_mem,
                        true
                    )
                ),
                style,
            ),
            Span::styled(fmt::uptime(p.uptime), style.fg(theme::palette().text_dim)),
        ]);
        frame.render_widget(Paragraph::new(line).style(style), rect);
        hits.push(rect, Action::SelectRow(i));
    }
}
