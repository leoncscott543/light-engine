//! Particles tab: a query bar over two views of the same particle frame — an
//! RGB heatmap of the matched particles and a sortable table with an inspector.

use light_contracts::{BinValue, Histogram2d, ParticleField, ParticleFrame, SortOrder};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::app::App;
use crate::colormap::Colormap;
use crate::explorer::{Explorer, ExplorerAction as A};
use crate::fmt;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, Button, hovered, panel};

const Y_LABEL_W: u16 = 9;
const INSPECTOR_H: u16 = 20;

pub fn render(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let [bar, body] = Layout::vertical([Constraint::Length(4), Constraint::Min(0)]).areas(area);
    query_bar(app, frame, bar, hits);

    if !app.has_session() {
        let block = panel("Particles", false);
        let inner = block.inner(body);
        frame.render_widget(block, body);
        widgets::empty_state(
            frame,
            inner,
            "no active session",
            "queries run against live state · create a session to explore it",
        );
        let buttons = [
            Button::new("Create session", Action::CreateSession)
                .key("n")
                .primary(),
            Button::new("Create & run", Action::ToggleRun).key("space"),
        ];
        let w: u16 = buttons.iter().map(|b| b.width() + 1).sum();
        let row = widgets::centered(inner, w, 1);
        let row = Rect {
            y: (row.y + 2).min(inner.bottom().saturating_sub(1)),
            ..row
        };
        widgets::button_row(frame, row, &buttons, app.mouse, hits);
        return;
    }

    if app.explorer.zoom {
        heatmap(app, frame, body, hits);
        return;
    }
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Min(0)]).areas(body);
    heatmap(app, frame, left, hits);
    let [table_area, inspector_area] =
        Layout::vertical([Constraint::Min(6), Constraint::Length(INSPECTOR_H)]).areas(right);
    table(app, frame, table_area, hits);
    inspector(app, frame, inspector_area, hits);
}

fn query_bar(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let ex = &app.explorer;
    let title = if ex.editing {
        "Query · editing (enter apply · esc cancel · tab complete)"
    } else {
        "Query · shared by heatmap and table"
    };
    let block = panel(title, ex.editing || ex.error.is_some());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width < 4 {
        return;
    }

    let run = if ex.editing { A::Submit } else { A::Refresh };
    let mut live = Button::new("Live", Action::Explore(A::ToggleLive)).key("a");
    if ex.live {
        live = live.primary();
    }
    let buttons = [
        Button::new("Run", Action::Explore(run)).key("⏎").primary(),
        live,
        Button::new("Reset", Action::Explore(A::ResetQuery)).key("r"),
    ];
    let buttons_w: u16 = buttons.iter().map(|b| b.width() + 1).sum();
    let show_buttons = inner.width > buttons_w + 20;
    let text_w = if show_buttons {
        inner.width - buttons_w - 1
    } else {
        inner.width
    };
    let text_area = Rect::new(inner.x, inner.y, text_w, 1);

    let prompt = "› ";
    let avail = (text_w as usize).saturating_sub(prompt.len() + 1).max(1);
    let chars: Vec<char> = ex.input.chars().collect();
    let start = if ex.editing {
        ex.cursor.saturating_sub(avail - 1)
    } else {
        0
    };
    let slice = |from: usize, to: usize| -> String {
        chars
            .iter()
            .skip(from)
            .take(to.saturating_sub(from))
            .collect()
    };
    let end = (start + avail).min(chars.len());
    let mut spans = vec![Span::styled(prompt, theme::title())];
    if ex.editing {
        let cursor = ex.cursor.clamp(start, end);
        spans.push(Span::styled(slice(start, cursor), theme::value()));
        let under = chars.get(cursor).map_or(" ".into(), |c| c.to_string());
        spans.push(Span::styled(
            under,
            Style::new().bg(theme::GOLD).fg(theme::BG),
        ));
        spans.push(Span::styled(slice(cursor + 1, end), theme::value()));
    } else {
        let style = if hovered(app.mouse, text_area) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            Style::new().fg(theme::TEXT)
        };
        let mut text = slice(0, end);
        if end < chars.len() && avail > 1 {
            text.pop();
            text.push('…');
        }
        spans.push(Span::styled(text, style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), text_area);
    hits.push(text_area, Action::Explore(A::EditQuery));

    if show_buttons {
        let mut x = text_area.right() + 1;
        for b in &buttons {
            let w = b.width();
            b.render(frame, Rect::new(x, inner.y, w, 1), app.mouse, hits);
            x += w + 1;
        }
    }

    if inner.height < 2 {
        return;
    }
    let line = if let Some(err) = &ex.error {
        let at = ex
            .input
            .get(..err.at)
            .map_or(chars.len(), |s| s.chars().count());
        let pad = (at.saturating_sub(start) + prompt.chars().count()).min(text_w as usize);
        Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(
                format!("▲ {}", err.message),
                Style::new().fg(theme::ERR).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        status_line(ex)
    };
    frame.render_widget(
        Paragraph::new(line),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
}

fn status_line(ex: &Explorer) -> Line<'static> {
    let sep = || Span::styled("  ·  ", theme::faint());
    let Some(pf) = &ex.frame else {
        return Line::styled("  no data yet", theme::dim());
    };
    let pct = if pf.scanned == 0 {
        0.0
    } else {
        pf.matched as f64 / pf.scanned as f64 * 100.0
    };
    let live = if ex.live {
        Span::styled(
            "● live",
            Style::new().fg(theme::OK).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "❚❚ frozen",
            Style::new().fg(theme::HONEY).add_modifier(Modifier::BOLD),
        )
    };
    Line::from(vec![
        Span::raw("  "),
        live,
        sep(),
        Span::styled("tick ", theme::label()),
        Span::styled(fmt::count(pf.tick), theme::value()),
        Span::styled(format!(" (t={:.3})", pf.sim_time), theme::dim()),
        sep(),
        Span::styled("matched ", theme::label()),
        Span::styled(fmt::count(pf.matched), theme::value()),
        Span::styled(
            format!(" / {} ({pct:.1}%)", fmt::count(pf.scanned)),
            theme::dim(),
        ),
        sep(),
        Span::styled("filters ", theme::label()),
        Span::styled(ex.query.filters.len().to_string(), theme::value()),
        sep(),
        Span::styled("scan ", theme::label()),
        Span::styled(fmt::duration(pf.elapsed), theme::value()),
    ])
}

/// Maps bin values onto `0..=1` for the colormap.
struct Scale {
    lo: f64,
    hi: f64,
    log: bool,
    count: bool,
}

impl Scale {
    fn new(h: &Histogram2d, log: bool) -> Self {
        let count = h.value == BinValue::Count;
        let (lo, hi) = if count {
            (0.0, f64::from(h.max_count()))
        } else {
            h.value_range
        };
        Self {
            lo,
            hi,
            log: log && (count || lo > 0.0),
            count,
        }
    }

    fn t(&self, v: f64) -> f64 {
        if self.hi <= self.lo {
            return 1.0;
        }
        if self.log && self.count {
            (1.0 + v).ln() / (1.0 + self.hi).ln()
        } else if self.log {
            (v / self.lo).ln() / (self.hi / self.lo).ln()
        } else {
            (v - self.lo) / (self.hi - self.lo)
        }
    }

    /// Inverse of [`Scale::t`], for legend labels.
    fn at(&self, t: f64) -> f64 {
        if self.log && self.count {
            (1.0 + self.hi).powf(t) - 1.0
        } else if self.log {
            self.lo * (self.hi / self.lo).powf(t)
        } else {
            self.lo + (self.hi - self.lo) * t
        }
    }
}

/// Combined count and value for a terminal cell spanning bins `bys` in column `bx`.
fn merged(h: &Histogram2d, bx: u16, bys: [u16; 2]) -> (u32, f64) {
    let mut n = 0u32;
    let mut acc = f64::NAN;
    for by in bys {
        let c = h.count(bx, by);
        if c == 0 {
            continue;
        }
        let v = h.value(bx, by);
        acc = match h.value {
            BinValue::Mean(_) => {
                let prev = if acc.is_nan() {
                    0.0
                } else {
                    acc * f64::from(n)
                };
                (prev + v * f64::from(c)) / f64::from(n + c)
            }
            BinValue::Max(_) if !acc.is_nan() => acc.max(v),
            BinValue::Count | BinValue::Sum(_) if !acc.is_nan() => acc + v,
            _ => v,
        };
        n += c;
    }
    (n, acc)
}

struct Plot {
    area: Rect,
}

impl Plot {
    /// Bin column for terminal column `cx` (histogram may lag a resize by a frame).
    fn bx(&self, h: &Histogram2d, cx: u16) -> u16 {
        ((u32::from(cx) * u32::from(h.bins_x)) / u32::from(self.area.width)) as u16
    }

    /// Bin row for half-row `sy` counted from the top.
    fn by(&self, h: &Histogram2d, sy: u16) -> u16 {
        let rows = u32::from(self.area.height) * 2;
        let from_bottom = rows - 1 - u32::from(sy);
        ((from_bottom * u32::from(h.bins_y)) / rows) as u16
    }

    /// Data-space bounds of terminal cell `(cx, cy)`.
    fn cell_bounds(&self, h: &Histogram2d, cx: u16, cy: u16) -> ((f64, f64), (f64, f64)) {
        let (w, ht) = (f64::from(self.area.width), f64::from(self.area.height));
        let lerp = |(lo, hi): (f64, f64), t: f64| lo + (hi - lo) * t;
        let x = (
            lerp(h.x_range, f64::from(cx) / w),
            lerp(h.x_range, f64::from(cx + 1) / w),
        );
        let y = (
            lerp(h.y_range, (ht - f64::from(cy) - 1.0) / ht),
            lerp(h.y_range, (ht - f64::from(cy)) / ht),
        );
        (x, y)
    }
}

fn heatmap(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let ex = &app.explorer;
    let title = format!(
        "Heatmap · {} over {} × {}",
        ex.bin_value(),
        ex.heat_x,
        ex.heat_y
    );
    let block = panel(&title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 6 || inner.width < Y_LABEL_W + 8 {
        ex.viewport.set(None);
        return;
    }

    heat_controls(app, frame, Rect { height: 1, ..inner }, hits);

    let [_, plot_row, x_axis, legend, info] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let [y_axis, plot_area] =
        Layout::horizontal([Constraint::Length(Y_LABEL_W), Constraint::Min(1)]).areas(plot_row);
    ex.viewport.set(Some((plot_area.width, plot_area.height)));

    let Some(h) = ex.frame.as_ref().and_then(|f| f.histogram.as_ref()) else {
        widgets::empty_state(frame, plot_area, "querying…", "");
        return;
    };
    let plot = Plot { area: plot_area };
    let scale = Scale::new(h, ex.log_scale);
    let cmap = ex.colormap;
    let color_of =
        |bx: u16, by: u16| (h.count(bx, by) > 0).then(|| cmap.color(scale.t(h.value(bx, by))));

    let buf = frame.buffer_mut();
    for cy in 0..plot_area.height {
        for cx in 0..plot_area.width {
            let bx = plot.bx(h, cx);
            let top = color_of(bx, plot.by(h, cy * 2));
            let bottom = color_of(bx, plot.by(h, cy * 2 + 1));
            if let Some(cell) = buf.cell_mut((plot_area.x + cx, plot_area.y + cy)) {
                cell.set_char('▀')
                    .set_fg(top.unwrap_or(theme::SURFACE))
                    .set_bg(bottom.unwrap_or(theme::SURFACE));
            }
        }
    }

    // Selected particle from the table, located in the same frame.
    if let Some(row) = ex.frame.as_ref().and_then(|f| f.rows.get(ex.selected))
        && let Some((bx, by)) = h.locate(row.get(ex.heat_x), row.get(ex.heat_y))
    {
        let cx = (u32::from(bx) * u32::from(plot_area.width) / u32::from(h.bins_x)) as u16;
        let rows = u32::from(plot_area.height) * 2;
        let sy = rows - 1 - (u32::from(by) * rows / u32::from(h.bins_y)).min(rows - 1);
        let cy = (sy / 2) as u16;
        let t = if h.count(bx, by) > 0 {
            scale.t(h.value(bx, by))
        } else {
            0.0
        };
        if let Some(cell) = buf.cell_mut((plot_area.x + cx, plot_area.y + cy)) {
            cell.set_char('◆')
                .set_fg(cmap.contrast(t))
                .set_bg(cmap.color(t))
                .modifier
                .insert(Modifier::BOLD);
        }
    }

    for cy in 0..plot_area.height {
        for cx in 0..plot_area.width {
            let (x, y) = plot.cell_bounds(h, cx, cy);
            hits.push(
                Rect::new(plot_area.x + cx, plot_area.y + cy, 1, 1),
                Action::Explore(A::Drill { x, y }),
            );
        }
    }

    // Y axis: max at top, min at bottom, field name in the middle.
    let label = |v: f64| format!("{:>w$}", fmt::value(v, false), w = Y_LABEL_W as usize - 1);
    let y_lines: Vec<Line> = (0..y_axis.height)
        .map(|r| {
            if r == 0 {
                Line::styled(label(h.y_range.1), theme::dim())
            } else if r + 1 == y_axis.height {
                Line::styled(label(h.y_range.0), theme::dim())
            } else if r == y_axis.height / 2 {
                Line::styled(
                    format!("{:>w$}", ex.heat_y.name(), w = Y_LABEL_W as usize - 1),
                    theme::title(),
                )
            } else {
                Line::raw("")
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(y_lines), y_axis);

    let x_axis = Rect {
        x: plot_area.x,
        width: plot_area.width,
        ..x_axis
    };
    let lo = fmt::value(h.x_range.0, false);
    let hi = fmt::value(h.x_range.1, false);
    let name = ex.heat_x.name();
    let gap = (x_axis.width as usize).saturating_sub(lo.len() + hi.len() + name.len());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(lo, theme::dim()),
            Span::raw(" ".repeat(gap / 2)),
            Span::styled(name, theme::title()),
            Span::raw(" ".repeat(gap - gap / 2)),
            Span::styled(hi, theme::dim()),
        ])),
        x_axis,
    );

    color_legend(frame, legend, &scale, cmap, h);
    hover_info(app, frame, info, &plot, h);
}

fn heat_controls(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let ex = &app.explorer;
    let x_label = format!("x: {}", ex.heat_x);
    let y_label = format!("y: {}", ex.heat_y);
    let mode = format!("{}", ex.bin_value());
    let mut log = Button::new("log", Action::Explore(A::ToggleLog)).key("o");
    if ex.log_scale {
        log = log.primary();
    }
    let mut zoom = Button::new("zoom", Action::Explore(A::ToggleZoom)).key("z");
    if ex.zoom {
        zoom = zoom.primary();
    }
    let buttons = [
        Button::new(&x_label, Action::Explore(A::CycleX(1))).key("x"),
        Button::new(&y_label, Action::Explore(A::CycleY(1))).key("y"),
        Button::new(&mode, Action::Explore(A::CycleMode)).key("m"),
        Button::new(ex.colormap.name(), Action::Explore(A::CycleColormap)).key("c"),
        log,
        zoom,
    ];
    let mut x = area.x;
    for b in &buttons {
        let w = b.width();
        if x + w > area.right() {
            break;
        }
        b.render(frame, Rect::new(x, area.y, w, 1), app.mouse, hits);
        x += w + 1;
    }
}

fn color_legend(frame: &mut Frame, area: Rect, scale: &Scale, cmap: Colormap, h: &Histogram2d) {
    if h.max_count() == 0 {
        frame.render_widget(
            Paragraph::new(Line::styled("no particles in range", theme::dim())),
            area,
        );
        return;
    }
    let lo = fmt::value(scale.at(0.0), scale.count);
    let hi = fmt::value(scale.at(1.0), scale.count);
    let suffix = if scale.log { " log" } else { "" };
    let fixed = Y_LABEL_W as usize + hi.len() + 2 + suffix.len();
    let bar_w = (area.width as usize).saturating_sub(fixed).max(1);
    let mut spans = vec![Span::styled(
        format!("{:>w$} ", lo, w = Y_LABEL_W as usize - 1),
        theme::dim(),
    )];
    for i in 0..bar_w {
        let t = i as f64 / (bar_w.max(2) - 1) as f64;
        spans.push(Span::styled("█", Style::new().fg(cmap.color(t))));
    }
    spans.push(Span::styled(format!(" {hi}"), theme::dim()));
    spans.push(Span::styled(suffix, theme::title()));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn hover_info(app: &App, frame: &mut Frame, area: Rect, plot: &Plot, h: &Histogram2d) {
    let ex = &app.explorer;
    let hover = app
        .mouse
        .filter(|(x, y)| plot.area.contains(Position::new(*x, *y)));
    let line = match hover {
        Some((mx, my)) => {
            let (cx, cy) = (mx - plot.area.x, my - plot.area.y);
            let (xb, yb) = plot.cell_bounds(h, cx, cy);
            let bx = plot.bx(h, cx);
            let (n, v) = merged(h, bx, [plot.by(h, cy * 2), plot.by(h, cy * 2 + 1)]);
            let mut spans = vec![
                Span::styled(format!(" {} ", ex.heat_x), theme::label()),
                Span::styled(
                    format!("{}‥{}", fmt::value(xb.0, false), fmt::value(xb.1, false)),
                    theme::value(),
                ),
                Span::styled(format!("  {} ", ex.heat_y), theme::label()),
                Span::styled(
                    format!("{}‥{}", fmt::value(yb.0, false), fmt::value(yb.1, false)),
                    theme::value(),
                ),
                Span::styled("  n ", theme::label()),
                Span::styled(n.to_string(), theme::value()),
            ];
            if h.value != BinValue::Count && n > 0 {
                spans.push(Span::styled(format!("  {} ", h.value), theme::label()));
                spans.push(Span::styled(fmt::value(v, false), theme::value()));
            }
            spans.push(Span::styled("  click to drill in", theme::faint()));
            Line::from(spans)
        }
        None => Line::styled(
            " hover a bin for details · click to drill in · ⌫ pops a filter · s sorts by heat field",
            theme::faint(),
        ),
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn col_width(field: ParticleField) -> usize {
    (field.name().len() + 1).max(8)
}

fn table(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let ex = &app.explorer;
    let Some(pf) = &ex.frame else {
        let block = panel("Rows", false);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        widgets::empty_state(frame, inner, "querying…", "");
        return;
    };
    let sort = match ex.query.sort {
        Some((f, SortOrder::Desc)) => format!(" · by {f} ▼"),
        Some((f, SortOrder::Asc)) => format!(" · by {f} ▲"),
        None => String::new(),
    };
    let title = format!(
        "Rows {}–{} of {} matched{sort}",
        ex.query.offset + u64::from(!pf.rows.is_empty()),
        ex.query.offset + pf.rows.len() as u64,
        fmt::count(pf.matched),
    );
    let block = panel(&title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 2 {
        return;
    }
    if pf.rows.is_empty() {
        widgets::empty_state(
            frame,
            inner,
            "no particles match",
            "⌫ pops the last filter · r resets the query",
        );
        return;
    }

    // Columns that fit, after a colormap swatch tying rows to the heatmap.
    let swatch_w = 3u16;
    let mut cols = Vec::new();
    let mut used = swatch_w as usize;
    for &field in &pf.columns {
        let w = col_width(field) + 1;
        if used + w > inner.width as usize {
            break;
        }
        cols.push((field, used as u16, w as u16));
        used += w;
    }

    let header_y = inner.y;
    for &(field, x, w) in &cols {
        let rect = Rect::new(inner.x + x, header_y, w - 1, 1);
        let arrow = match ex.query.sort {
            Some((f, SortOrder::Desc)) if f == field => "▼",
            Some((f, SortOrder::Asc)) if f == field => "▲",
            _ => "",
        };
        let style = if !arrow.is_empty() {
            theme::selected()
        } else if hovered(app.mouse, rect) {
            Style::new()
                .bg(theme::HOVER)
                .fg(theme::GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::title()
        };
        let text = format!("{arrow}{}", field.name());
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{text:>w$}", w = (w - 1) as usize),
                style,
            )),
            rect,
        );
        hits.push(rect, Action::Explore(A::SortBy(field)));
    }

    let swatch_scale = pf
        .stat(ex.heat_field)
        .map(|s| (s.min, s.max))
        .unwrap_or((0.0, 1.0));
    let body = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
    let height = body.height as usize;
    let offset = widgets::scroll_offset(ex.selected, height);
    for (line, (i, row)) in pf
        .rows
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .enumerate()
    {
        let rect = Rect::new(body.x, body.y + line as u16, body.width, 1);
        let style = if i == ex.selected {
            theme::selected()
        } else if hovered(app.mouse, rect) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            theme::base()
        };
        let v = row.get(ex.heat_field);
        let (lo, hi) = swatch_scale;
        let t = if hi > lo { (v - lo) / (hi - lo) } else { 0.5 };
        let mut spans = vec![
            Span::styled("██", Style::new().fg(ex.colormap.color(t))),
            Span::styled(" ", style),
        ];
        for &(field, _, w) in &cols {
            let text = fmt::value(row.get(field), field.is_integer());
            spans.push(Span::styled(
                format!("{text:>w$} ", w = (w - 1) as usize),
                style,
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)).style(style), rect);
        hits.push(rect, Action::SelectRow(i));
    }
}

fn inspector(app: &App, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    let ex = &app.explorer;
    let pf: Option<&ParticleFrame> = ex.frame.as_ref();
    let row = pf.and_then(|f| f.rows.get(ex.selected));
    let title = match row {
        Some(r) => format!("Particle #{} · vs all matched", r.index),
        None => "Particle".into(),
    };
    let block = panel(&title, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (Some(pf), Some(row)) = (pf, row) else {
        widgets::empty_state(
            frame,
            inner,
            "nothing selected",
            "select a row in the table",
        );
        return;
    };
    if inner.height < 2 {
        return;
    }

    // tag, field, value, position-in-range bar, then stats while they fit.
    const NUM: usize = 10;
    let fixed = 2 + 9 + NUM + 12;
    let stat_cols = ((inner.width as usize).saturating_sub(fixed) / NUM).min(4);
    let stat_names = ["min", "mean", "max", "σ"];
    let mut header = format!("  {:<9}{:>NUM$} {:<11}", "field", "value", " range");
    for name in &stat_names[..stat_cols] {
        header.push_str(&format!("{name:>NUM$}"));
    }
    frame.render_widget(
        Paragraph::new(Span::styled(header, theme::title())),
        Rect { height: 1, ..inner },
    );

    for (i, field) in ParticleField::ALL.iter().enumerate() {
        let y = inner.y + 1 + i as u16;
        if y >= inner.bottom() {
            break;
        }
        let rect = Rect::new(inner.x, y, inner.width, 1);
        let is_heat = *field == ex.heat_field && ex.bin_value() != BinValue::Count;
        let tag = if is_heat {
            "◆ "
        } else if *field == ex.heat_x {
            "x "
        } else if *field == ex.heat_y {
            "y "
        } else {
            "  "
        };
        let style = if is_heat {
            theme::selected()
        } else if hovered(app.mouse, rect) {
            Style::new().bg(theme::HOVER).fg(theme::TEXT)
        } else {
            theme::base()
        };
        let v = row.get(*field);
        let stat = pf.stat(*field);
        let t = stat.map_or(0.5, |s| {
            if s.max > s.min {
                (v - s.min) / (s.max - s.min)
            } else {
                0.5
            }
        });
        let mut spans = vec![
            Span::styled(tag, style.fg(theme::GOLD)),
            Span::styled(format!("{:<9}", field.name()), style.fg(theme::TEXT_DIM)),
            Span::styled(
                format!("{:>NUM$} ", fmt::value(v, field.is_integer())),
                style.add_modifier(Modifier::BOLD),
            ),
            Span::styled(widgets::bar(t, 10), style.fg(ex.colormap.color(t))),
            Span::styled(" ", style),
        ];
        if let Some(s) = stat {
            let values = [s.min, s.mean, s.max, s.stddev];
            for value in &values[..stat_cols] {
                spans.push(Span::styled(
                    format!("{:>NUM$}", fmt::value(*value, false)),
                    style.fg(theme::TEXT_DIM),
                ));
            }
        }
        frame.render_widget(Paragraph::new(Line::from(spans)).style(style), rect);
        hits.push(rect, Action::Explore(A::HeatField(*field)));
    }
}
