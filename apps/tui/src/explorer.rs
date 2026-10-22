//! State behind the Particles tab: one shared [`ParticleQuery`] whose result
//! frame feeds both the heatmap and the table, plus the query editor.

use std::cell::Cell;
use std::time::Duration;

use light_contracts::{
    BinValue, Filter, HistogramSpec, KEYWORDS, ParticleField, ParticleFrame, ParticleQuery,
    ParticleRequest, QueryError, SortOrder,
};

use crate::colormap::Colormap;

const HISTORY_LEN: usize = 50;
const MIN_REFRESH: Duration = Duration::from_millis(100);

/// Aggregation shown by the heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeatMode {
    Count,
    Mean,
    Max,
    Sum,
}

impl HeatMode {
    pub const ALL: [Self; 4] = [Self::Count, Self::Mean, Self::Max, Self::Sum];

    fn bin_value(self, field: ParticleField) -> BinValue {
        match self {
            Self::Count => BinValue::Count,
            Self::Mean => BinValue::Mean(field),
            Self::Max => BinValue::Max(field),
            Self::Sum => BinValue::Sum(field),
        }
    }
}

/// Intents handled entirely by [`Explorer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExplorerAction {
    EditQuery,
    Insert(char),
    Backspace,
    Delete,
    Cursor(i32),
    CursorHome,
    CursorEnd,
    History(i32),
    Complete,
    Submit,
    CancelEdit,

    CycleX(i32),
    CycleY(i32),
    CycleMode,
    CycleField(i32),
    /// Aggregate this field in the heatmap (switching away from `count`).
    HeatField(ParticleField),
    CycleColormap,
    ToggleLog,
    ToggleLive,
    ToggleZoom,

    Refresh,
    ResetQuery,
    PopFilter,
    /// Sort by a column; repeating flips the order.
    SortBy(ParticleField),
    /// Narrow the shared query to a heatmap region (inclusive bounds).
    Drill {
        x: (f64, f64),
        y: (f64, f64),
    },
}

pub struct Explorer {
    /// Editor text; mirrors `query` whenever not editing.
    pub input: String,
    /// Cursor position in chars.
    pub cursor: usize,
    pub editing: bool,
    /// The applied query shared by every view.
    pub query: ParticleQuery,
    pub error: Option<QueryError>,
    history: Vec<String>,
    history_pos: Option<usize>,

    pub heat_x: ParticleField,
    pub heat_y: ParticleField,
    pub heat_mode: HeatMode,
    pub heat_field: ParticleField,
    pub colormap: Colormap,
    pub log_scale: bool,
    pub live: bool,
    pub zoom: bool,

    pub frame: Option<ParticleFrame>,
    pub selected: usize,
    /// Particle id of the selection, kept across refreshes.
    selected_id: Option<u64>,
    /// Plot size in cells, published by the renderer for bin sizing.
    pub viewport: Cell<Option<(u16, u16)>>,
    requested_viewport: Option<(u16, u16)>,
    pub last_run: Option<std::time::Instant>,
    dirty: bool,
}

impl Default for Explorer {
    fn default() -> Self {
        let query = ParticleQuery::default();
        Self {
            input: query.to_string(),
            cursor: 0,
            editing: false,
            query,
            error: None,
            history: Vec::new(),
            history_pos: None,
            heat_x: ParticleField::X,
            heat_y: ParticleField::Y,
            heat_mode: HeatMode::Mean,
            heat_field: ParticleField::Activity,
            colormap: Colormap::Turbo,
            log_scale: false,
            live: true,
            zoom: false,
            frame: None,
            selected: 0,
            selected_id: None,
            viewport: Cell::new(None),
            requested_viewport: None,
            last_run: None,
            dirty: true,
        }
    }
}

impl Explorer {
    pub fn bin_value(&self) -> BinValue {
        self.heat_mode.bin_value(self.heat_field)
    }

    pub fn request(&self) -> ParticleRequest {
        let (w, h) = self.viewport.get().unwrap_or((64, 16));
        ParticleRequest {
            query: self.query.clone(),
            histogram: Some(HistogramSpec {
                x: self.heat_x,
                y: self.heat_y,
                bins_x: w.clamp(1, HistogramSpec::MAX_BINS),
                bins_y: h.saturating_mul(2).clamp(1, HistogramSpec::MAX_BINS),
                value: self.bin_value(),
                x_range: None,
                y_range: None,
            }),
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Whether a new frame should be fetched for a state at `tick`.
    pub fn needs_refresh(&self, tick: u64, now: std::time::Instant) -> bool {
        if self.dirty || self.viewport.get() != self.requested_viewport {
            return true;
        }
        let stale = self.frame.as_ref().is_none_or(|f| f.tick != tick);
        let interval = self
            .frame
            .as_ref()
            .map_or(MIN_REFRESH, |f| (f.elapsed * 3).max(MIN_REFRESH));
        self.live && stale && self.last_run.is_none_or(|t| now - t >= interval)
    }

    /// Records a failed fetch; returns whether this is a new failure worth reporting.
    pub fn fail(&mut self, now: std::time::Instant) -> bool {
        let fresh = self.frame.take().is_some() || self.dirty;
        self.dirty = false;
        self.requested_viewport = self.viewport.get();
        self.last_run = Some(now);
        fresh
    }

    pub fn accept(&mut self, frame: ParticleFrame, now: std::time::Instant) {
        self.dirty = false;
        self.requested_viewport = self.viewport.get();
        self.last_run = Some(now);
        if let Some(pos) = self
            .selected_id
            .and_then(|id| frame.rows.iter().position(|r| r.index == id))
        {
            self.selected = pos;
        } else {
            self.selected = self.selected.min(frame.rows.len().saturating_sub(1));
        }
        self.selected_id = frame.rows.get(self.selected).map(|r| r.index);
        self.frame = Some(frame);
    }

    pub fn clear(&mut self) {
        self.frame = None;
        self.dirty = true;
    }

    pub fn select(&mut self, row: usize) {
        let Some(frame) = &self.frame else {
            return;
        };
        if let Some(r) = frame.rows.get(row) {
            self.selected = row;
            self.selected_id = Some(r.index);
        }
    }

    pub fn row_count(&self) -> usize {
        self.frame.as_ref().map_or(0, |f| f.rows.len())
    }

    pub fn apply(&mut self, action: ExplorerAction) {
        use ExplorerAction as A;
        match action {
            A::EditQuery => {
                self.editing = true;
                self.cursor = self.input.chars().count();
                self.history_pos = None;
            }
            A::Insert(c) => {
                let at = self.byte_at(self.cursor);
                self.input.insert(at, c);
                self.cursor += 1;
                self.error = None;
            }
            A::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    let at = self.byte_at(self.cursor);
                    self.input.remove(at);
                    self.error = None;
                }
            }
            A::Delete => {
                if self.cursor < self.input.chars().count() {
                    let at = self.byte_at(self.cursor);
                    self.input.remove(at);
                    self.error = None;
                }
            }
            A::Cursor(d) => {
                let len = self.input.chars().count() as i64;
                self.cursor = (self.cursor as i64 + i64::from(d)).clamp(0, len) as usize;
            }
            A::CursorHome => self.cursor = 0,
            A::CursorEnd => self.cursor = self.input.chars().count(),
            A::History(d) => self.recall(d),
            A::Complete => self.complete(),
            A::Submit => self.submit(),
            A::CancelEdit => {
                self.editing = false;
                self.error = None;
                self.sync_input();
            }

            A::CycleX(d) => {
                self.heat_x = cycle(&ParticleField::ALL, self.heat_x, d);
                self.dirty = true;
            }
            A::CycleY(d) => {
                self.heat_y = cycle(&ParticleField::ALL, self.heat_y, d);
                self.dirty = true;
            }
            A::CycleMode => {
                self.heat_mode = cycle(&HeatMode::ALL, self.heat_mode, 1);
                self.dirty = true;
            }
            A::CycleField(d) => {
                self.heat_field = cycle(&ParticleField::ALL, self.heat_field, d);
                if self.heat_mode == HeatMode::Count {
                    self.heat_mode = HeatMode::Mean;
                }
                self.dirty = true;
            }
            A::HeatField(field) => {
                self.heat_field = field;
                if self.heat_mode == HeatMode::Count {
                    self.heat_mode = HeatMode::Mean;
                }
                self.dirty = true;
            }
            A::CycleColormap => self.colormap = cycle(&Colormap::ALL, self.colormap, 1),
            A::ToggleLog => self.log_scale = !self.log_scale,
            A::ToggleLive => {
                self.live = !self.live;
                self.dirty |= self.live;
            }
            A::ToggleZoom => self.zoom = !self.zoom,

            A::Refresh => self.dirty = true,
            A::ResetQuery => self.set_query(ParticleQuery::default()),
            A::PopFilter => {
                let mut q = self.query.clone();
                if q.filters.pop().is_some() {
                    self.set_query(q);
                }
            }
            A::SortBy(field) => {
                let mut q = self.query.clone();
                q.sort = Some(match q.sort {
                    Some((f, order)) if f == field => (field, order.flip()),
                    _ => (field, SortOrder::Desc),
                });
                self.set_query(q);
            }
            A::Drill { x, y } => {
                let mut q = self.query.clone();
                for (field, (lo, hi)) in [(self.heat_x, x), (self.heat_y, y)] {
                    q.filters.push(Filter::Between {
                        field,
                        lo: round_sig(lo, 4, f64::floor),
                        hi: round_sig(hi, 4, f64::ceil),
                    });
                }
                self.set_query(q);
            }
        }
    }

    fn set_query(&mut self, query: ParticleQuery) {
        self.push_history(query.to_string());
        self.query = query;
        self.error = None;
        self.editing = false;
        self.sync_input();
        self.dirty = true;
    }

    fn sync_input(&mut self) {
        self.input = self.query.to_string();
        self.cursor = self.input.chars().count();
    }

    fn submit(&mut self) {
        match ParticleQuery::parse(&self.input) {
            Ok(query) => self.set_query(query),
            Err(err) => {
                self.cursor = self.input[..err.at.min(self.input.len())].chars().count();
                self.error = Some(err);
                self.editing = true;
            }
        }
    }

    fn push_history(&mut self, text: String) {
        self.history.retain(|h| *h != text);
        self.history.push(text);
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }
        self.history_pos = None;
    }

    fn recall(&mut self, d: i32) {
        if self.history.is_empty() {
            return;
        }
        let last = self.history.len() - 1;
        let pos = match (self.history_pos, d < 0) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(0), true) => Some(0),
            (Some(p), true) => Some(p - 1),
            (Some(p), false) if p >= last => None,
            (Some(p), false) => Some(p + 1),
        };
        self.history_pos = pos;
        match pos {
            Some(p) => self.input = self.history[p].clone(),
            None => self.input = self.query.to_string(),
        }
        self.cursor = self.input.chars().count();
        self.error = None;
    }

    /// Completes the word before the cursor to a field name or keyword.
    fn complete(&mut self) {
        let chars: Vec<char> = self.input.chars().collect();
        let end = self.cursor.min(chars.len());
        let mut start = end;
        while start > 0 && (chars[start - 1].is_ascii_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        let prefix: String = chars[start..end].iter().collect::<String>().to_lowercase();
        if prefix.is_empty() {
            return;
        }
        let mut candidates: Vec<&str> = ParticleField::ALL
            .iter()
            .map(|f| f.name())
            .chain(KEYWORDS.iter().copied().filter(|k| *k != "*"))
            .filter(|c| c.starts_with(&prefix))
            .collect();
        candidates.sort_unstable();
        candidates.dedup();
        let Some(first) = candidates.first() else {
            return;
        };
        let common = candidates.iter().fold(first.to_string(), |acc, c| {
            acc.chars()
                .zip(c.chars())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a)
                .collect()
        });
        let mut completion = common;
        if candidates.len() == 1 {
            completion.push(' ');
        }
        let (a, b) = (self.byte_at(start), self.byte_at(end));
        self.input.replace_range(a..b, &completion);
        self.cursor = start + completion.chars().count();
    }

    fn byte_at(&self, char_index: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_index)
            .map_or(self.input.len(), |(i, _)| i)
    }
}

fn cycle<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let i = values.iter().position(|v| *v == current).unwrap_or(0) as i64;
    values[(i + i64::from(delta)).rem_euclid(values.len() as i64) as usize]
}

/// Rounds to `digits` significant digits in the direction of `round`, so
/// drill-down bounds stay readable without shrinking the selected region.
fn round_sig(v: f64, digits: i32, round: fn(f64) -> f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    // Snap first so representation error (0.2 * 1e4 = 2000.0000000000002)
    // does not push `ceil` up a whole digit.
    let scaled = (v * scale * 1e6).round() / 1e6;
    let r = round(scaled) / scale;
    // Re-parse to drop float noise like 0.30000000000000004 from the display.
    format!("{r:.12e}").parse().unwrap_or(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ExplorerAction as A;

    fn typed(text: &str) -> Explorer {
        let mut e = Explorer::default();
        e.apply(A::EditQuery);
        while !e.input.is_empty() {
            e.apply(A::Backspace);
        }
        for c in text.chars() {
            e.apply(A::Insert(c));
        }
        e
    }

    #[test]
    fn submit_applies_or_reports() {
        let mut e = typed("where speed > 1 order by energy");
        e.apply(A::Submit);
        assert!(!e.editing && e.error.is_none());
        assert_eq!(e.query.filters.len(), 1);
        assert_eq!(e.input, e.query.to_string());

        let mut e = typed("where nope > 1");
        e.apply(A::Submit);
        assert!(e.editing);
        assert_eq!(e.error.as_ref().unwrap().at, 6);
        assert_eq!(e.cursor, 6);
        assert_eq!(e.query, ParticleQuery::default());
        e.apply(A::CancelEdit);
        assert_eq!(e.input, ParticleQuery::default().to_string());
    }

    #[test]
    fn editing_handles_cursor_and_unicode() {
        let mut e = typed("ab");
        e.apply(A::CursorHome);
        e.apply(A::Insert('é'));
        e.apply(A::Cursor(1));
        e.apply(A::Delete);
        assert_eq!(e.input, "éa");
        e.apply(A::CursorEnd);
        e.apply(A::Backspace);
        assert_eq!(e.input, "é");
    }

    #[test]
    fn completes_fields_and_keywords() {
        let mut e = typed("select act");
        e.apply(A::Complete);
        assert_eq!(e.input, "select activity ");
        let mut e = typed("where v");
        e.apply(A::Complete);
        assert_eq!(e.input, "where v", "vx/vy/vz share only `v`");
        let mut e = typed("ord");
        e.apply(A::Complete);
        assert_eq!(e.input, "order ");
    }

    #[test]
    fn history_recalls_applied_queries() {
        let mut e = typed("where x > 0");
        e.apply(A::Submit);
        e.apply(A::SortBy(ParticleField::Speed));
        e.apply(A::EditQuery);
        e.apply(A::History(-1));
        assert!(e.input.contains("order by speed desc"));
        e.apply(A::History(-1));
        assert!(e.input.ends_with("where x > 0"));
        e.apply(A::History(1));
        e.apply(A::History(1));
        assert_eq!(e.input, e.query.to_string());
    }

    #[test]
    fn sort_toggles_and_drill_narrows() {
        let mut e = Explorer::default();
        e.apply(A::SortBy(ParticleField::Energy));
        assert_eq!(e.query.sort, Some((ParticleField::Energy, SortOrder::Desc)));
        e.apply(A::SortBy(ParticleField::Energy));
        assert_eq!(e.query.sort, Some((ParticleField::Energy, SortOrder::Asc)));

        e.apply(A::Drill {
            x: (0.123456, 0.2),
            y: (-1.0, -0.987654),
        });
        assert_eq!(
            e.query.filters,
            vec![
                Filter::Between {
                    field: ParticleField::X,
                    lo: 0.1234,
                    hi: 0.2
                },
                Filter::Between {
                    field: ParticleField::Y,
                    lo: -1.0,
                    hi: -0.9876
                },
            ]
        );
        e.apply(A::PopFilter);
        e.apply(A::PopFilter);
        assert!(e.query.filters.is_empty());
    }

    #[test]
    fn request_matches_viewport() {
        let e = Explorer::default();
        e.viewport.set(Some((80, 20)));
        let spec = e.request().histogram.unwrap();
        assert_eq!((spec.bins_x, spec.bins_y), (80, 40));
        assert_eq!(spec.value, BinValue::Mean(ParticleField::Activity));
    }
}
