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

const COLUMNS: [(&str, usize); 7] = [
    ("PID", 5),
    ("NAME", 16),
    ("KIND", 11),
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
        .get(&p.id)
        .map(|s| s.iter().map(|c| c * 100.0).collect())
        .unwrap_or_default();
    let title = format!("{} · CPU history", p.name);
    charts::sparkline(
        frame,
        detail,
        &title,
        &history,
        theme::HONEY,
        |v| format!("{v:.0}%"),
        false,
    );
}

fn row_text(cells: [String; 7]) -> String {
    cells
        .iter()
        .zip(COLUMNS)
        .map(|(c, (_, w))| format!("{c:<w$.w$} "))
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
    let offset = widgets::scroll_offset(app.procs.selected, height);
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
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            theme::base()
        };
        let cpu = format!(
            "{} {:>4.0}%",
            widgets::bar(p.cpu.min(1.0), 10),
            p.cpu * 100.0
        );
        let pre = row_text([
            p.id.to_string(),
            p.name.clone(),
            p.kind.to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]);
        // Split around the state column so it can carry its own color.
        let state_start: usize = COLUMNS[..3].iter().map(|(_, w)| w + 1).sum();
        let (w_state, w_cpu, w_mem) = (COLUMNS[3].1, COLUMNS[4].1, COLUMNS[5].1);
        let line = Line::from(vec![
            Span::styled(pre[..state_start].to_string(), style),
            Span::styled(
                format!("{:<w_state$.w_state$} ", p.state.to_string()),
                theme::process_state(&p.state).bg(style.bg.unwrap_or(theme::BG)),
            ),
            Span::styled(format!("{cpu:<w_cpu$} "), style.fg(theme::HONEY)),
            Span::styled(format!("{:<w_mem$} ", fmt::bytes(p.memory_bytes)), style),
            Span::styled(fmt::uptime(p.uptime), style.fg(theme::TEXT_DIM)),
        ]);
        frame.render_widget(Paragraph::new(line).style(style), rect);
        hits.push(rect, Action::SelectRow(i));
    }
}
