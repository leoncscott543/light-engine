//! Query execution over a [`ParticleSource`]: one consistent state produces
//! the result table, per-field statistics, and a 2D histogram together, so
//! every view of a [`ParticleFrame`] describes the same particles at the same
//! tick.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;
use std::time::{Duration, Instant};

use crate::particles::{FIELD_COUNT, ParticleField, ParticleRow, ParticleSource};
use crate::query::{ParticleQuery, SortOrder};

/// What each histogram bin reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinValue {
    /// Number of particles in the bin.
    Count,
    Mean(ParticleField),
    Max(ParticleField),
    Sum(ParticleField),
}

impl BinValue {
    pub fn field(self) -> Option<ParticleField> {
        match self {
            Self::Count => None,
            Self::Mean(f) | Self::Max(f) | Self::Sum(f) => Some(f),
        }
    }

    pub fn mode_name(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Mean(_) => "mean",
            Self::Max(_) => "max",
            Self::Sum(_) => "sum",
        }
    }

    /// Same mode applied to a different field (`Count` is unchanged).
    pub fn with_field(self, field: ParticleField) -> Self {
        match self {
            Self::Count => Self::Count,
            Self::Mean(_) => Self::Mean(field),
            Self::Max(_) => Self::Max(field),
            Self::Sum(_) => Self::Sum(field),
        }
    }
}

impl fmt::Display for BinValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.field() {
            None => f.write_str("count"),
            Some(field) => write!(f, "{}({field})", self.mode_name()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistogramSpec {
    pub x: ParticleField,
    pub y: ParticleField,
    pub bins_x: u16,
    pub bins_y: u16,
    pub value: BinValue,
    /// Fixed axis range; `None` fits the matched particles.
    pub x_range: Option<(f64, f64)>,
    pub y_range: Option<(f64, f64)>,
}

impl HistogramSpec {
    pub const MAX_BINS: u16 = 1024;
}

impl Default for HistogramSpec {
    fn default() -> Self {
        Self {
            x: ParticleField::X,
            y: ParticleField::Y,
            bins_x: 64,
            bins_y: 32,
            value: BinValue::Mean(ParticleField::Activity),
            x_range: None,
            y_range: None,
        }
    }
}

/// Everything a front end asks of one particle state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParticleRequest {
    pub query: ParticleQuery,
    /// Histogram over the particles matched by `query.filters`.
    pub histogram: Option<HistogramSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnStats {
    pub field: ParticleField,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub stddev: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryRow {
    pub index: u64,
    /// All fields, regardless of the selected columns.
    pub values: ParticleRow,
}

impl QueryRow {
    pub fn get(&self, field: ParticleField) -> f64 {
        self.values[field.index()]
    }
}

/// Binned aggregate over two fields. Bin `(bx, by)` is at
/// `by * bins_x + bx`, with `by = 0` at the low end of the y range.
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram2d {
    pub x: ParticleField,
    pub y: ParticleField,
    pub value: BinValue,
    pub bins_x: u16,
    pub bins_y: u16,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub counts: Vec<u32>,
    /// Aggregated value per bin; `NaN` for empty bins.
    pub values: Vec<f64>,
    /// Min and max over non-empty bins.
    pub value_range: (f64, f64),
}

impl Histogram2d {
    fn bin_index(&self, bx: u16, by: u16) -> usize {
        usize::from(by) * usize::from(self.bins_x) + usize::from(bx)
    }

    pub fn count(&self, bx: u16, by: u16) -> u32 {
        self.counts[self.bin_index(bx, by)]
    }

    pub fn value(&self, bx: u16, by: u16) -> f64 {
        self.values[self.bin_index(bx, by)]
    }

    /// Half-open `[lo, hi)` x interval covered by column `bx`.
    pub fn x_bounds(&self, bx: u16) -> (f64, f64) {
        bounds(self.x_range, self.bins_x, bx)
    }

    pub fn y_bounds(&self, by: u16) -> (f64, f64) {
        bounds(self.y_range, self.bins_y, by)
    }

    /// Bin containing the point, if it lies inside the ranges.
    pub fn locate(&self, x: f64, y: f64) -> Option<(u16, u16)> {
        Some((
            locate(self.x_range, self.bins_x, x)?,
            locate(self.y_range, self.bins_y, y)?,
        ))
    }

    pub fn max_count(&self) -> u32 {
        self.counts.iter().copied().max().unwrap_or(0)
    }
}

fn bounds((lo, hi): (f64, f64), bins: u16, b: u16) -> (f64, f64) {
    let w = (hi - lo) / f64::from(bins);
    (lo + w * f64::from(b), lo + w * f64::from(b + 1))
}

fn locate((lo, hi): (f64, f64), bins: u16, v: f64) -> Option<u16> {
    if !(lo..=hi).contains(&v) {
        return None;
    }
    let t = (v - lo) / (hi - lo) * f64::from(bins);
    Some((t as u16).min(bins - 1))
}

/// Result of one [`ParticleRequest`] against one consistent state.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleFrame {
    pub tick: u64,
    pub sim_time: f64,
    pub scanned: u64,
    pub matched: u64,
    pub columns: Vec<ParticleField>,
    pub rows: Vec<QueryRow>,
    /// Statistics for every field over all matched particles, in
    /// [`ParticleField::ALL`] order; empty when nothing matched.
    pub stats: Vec<ColumnStats>,
    pub histogram: Option<Histogram2d>,
    pub elapsed: Duration,
}

impl ParticleFrame {
    pub fn stat(&self, field: ParticleField) -> Option<&ColumnStats> {
        self.stats.get(field.index())
    }
}

#[derive(Clone, Copy)]
struct Welford {
    n: u64,
    mean: f64,
    m2: f64,
    min: f64,
    max: f64,
}

impl Welford {
    const EMPTY: Self = Self {
        n: 0,
        mean: 0.0,
        m2: 0.0,
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
    };

    fn push(&mut self, v: f64) {
        self.n += 1;
        let d = v - self.mean;
        self.mean += d / self.n as f64;
        self.m2 += d * (v - self.mean);
        self.min = self.min.min(v);
        self.max = self.max.max(v);
    }

    fn stats(&self, field: ParticleField) -> ColumnStats {
        ColumnStats {
            field,
            min: self.min,
            max: self.max,
            mean: self.mean,
            stddev: (self.m2 / self.n.max(1) as f64).sqrt(),
        }
    }
}

/// Max-heap entry ordered by `(key, index)` so the heap root is the worst kept row.
struct Ranked {
    key: f64,
    index: u64,
}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key
            .total_cmp(&other.key)
            .then(self.index.cmp(&other.index))
    }
}

impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Ranked {}

/// Runs `request` over every particle of `source`.
pub fn execute(source: &dyn ParticleSource, request: &ParticleRequest) -> ParticleFrame {
    let started = Instant::now();
    let query = &request.query;
    let count = source.particle_count();
    let window = query.offset.saturating_add(u64::from(query.limit));

    let mut row = [0.0; FIELD_COUNT];
    let mut matched = 0u64;
    let mut stats = [Welford::EMPTY; FIELD_COUNT];
    let mut rows = Vec::new();
    let mut heap = BinaryHeap::new();
    let mut points: Vec<(f64, f64, f64)> = Vec::new();

    for index in 0..count {
        source.read(index, &mut row);
        if !query.matches(&row) {
            continue;
        }
        for (w, v) in stats.iter_mut().zip(row) {
            w.push(v);
        }
        match query.sort {
            None => {
                if matched >= query.offset && matched < window {
                    rows.push(QueryRow { index, values: row });
                }
            }
            Some((field, order)) => {
                let v = row[field.index()];
                let key = if order == SortOrder::Desc { -v } else { v };
                heap.push(Ranked { key, index });
                if heap.len() as u64 > window {
                    heap.pop();
                }
            }
        }
        if let Some(spec) = &request.histogram {
            let value = spec.value.field().map_or(1.0, |f| row[f.index()]);
            points.push((row[spec.x.index()], row[spec.y.index()], value));
        }
        matched += 1;
    }

    if query.sort.is_some() {
        rows = heap
            .into_sorted_vec()
            .into_iter()
            .skip(usize::try_from(query.offset).unwrap_or(usize::MAX))
            .map(|r| {
                source.read(r.index, &mut row);
                QueryRow {
                    index: r.index,
                    values: row,
                }
            })
            .collect();
    }

    let histogram = request.histogram.as_ref().map(|spec| {
        let x_range = spec.x_range.unwrap_or_else(|| fit(&stats[spec.x.index()]));
        let y_range = spec.y_range.unwrap_or_else(|| fit(&stats[spec.y.index()]));
        bin(spec, x_range, y_range, &points)
    });

    ParticleFrame {
        tick: source.tick(),
        sim_time: source.sim_time(),
        scanned: count,
        matched,
        columns: query.columns.clone(),
        rows,
        stats: if matched == 0 {
            Vec::new()
        } else {
            ParticleField::ALL
                .iter()
                .map(|&f| stats[f.index()].stats(f))
                .collect()
        },
        histogram,
        elapsed: started.elapsed(),
    }
}

fn fit(w: &Welford) -> (f64, f64) {
    if w.n == 0 || !w.min.is_finite() || !w.max.is_finite() {
        return (0.0, 1.0);
    }
    if w.max - w.min <= f64::EPSILON * w.max.abs().max(1.0) {
        return (w.min - 0.5, w.max + 0.5);
    }
    (w.min, w.max)
}

fn bin(
    spec: &HistogramSpec,
    x_range: (f64, f64),
    y_range: (f64, f64),
    points: &[(f64, f64, f64)],
) -> Histogram2d {
    let bins_x = spec.bins_x.clamp(1, HistogramSpec::MAX_BINS);
    let bins_y = spec.bins_y.clamp(1, HistogramSpec::MAX_BINS);
    let len = usize::from(bins_x) * usize::from(bins_y);
    let mut h = Histogram2d {
        x: spec.x,
        y: spec.y,
        value: spec.value,
        bins_x,
        bins_y,
        x_range,
        y_range,
        counts: vec![0; len],
        values: vec![f64::NAN; len],
        value_range: (0.0, 0.0),
    };
    let valid = |(lo, hi): (f64, f64)| lo.is_finite() && hi.is_finite() && lo < hi;
    if !valid(x_range) || !valid(y_range) {
        return h;
    }
    for &(x, y, v) in points {
        let Some((bx, by)) = h.locate(x, y) else {
            continue;
        };
        let i = h.bin_index(bx, by);
        h.counts[i] += 1;
        let slot = &mut h.values[i];
        *slot = match spec.value {
            BinValue::Count => f64::from(h.counts[i]),
            BinValue::Max(_) if !slot.is_nan() => slot.max(v),
            BinValue::Mean(_) | BinValue::Sum(_) if !slot.is_nan() => *slot + v,
            _ => v,
        };
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for (v, &c) in h.values.iter_mut().zip(&h.counts) {
        if c == 0 {
            continue;
        }
        if matches!(spec.value, BinValue::Mean(_)) {
            *v /= f64::from(c);
        }
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if lo <= hi {
        h.value_range = (lo, hi);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use ParticleField as F;

    /// Particle `i` sits at `x = i % 10`, `y = i / 10` with `activity = i`.
    struct Grid(u64);

    impl ParticleSource for Grid {
        fn particle_count(&self) -> u64 {
            self.0
        }
        fn tick(&self) -> u64 {
            42
        }
        fn sim_time(&self) -> f64 {
            0.5
        }
        fn read(&self, index: u64, row: &mut ParticleRow) {
            *row = [0.0; FIELD_COUNT];
            row[F::Id.index()] = index as f64;
            row[F::X.index()] = (index % 10) as f64;
            row[F::Y.index()] = (index / 10) as f64;
            row[F::Activity.index()] = index as f64;
        }
    }

    fn run(text: &str, histogram: Option<HistogramSpec>) -> ParticleFrame {
        let request = ParticleRequest {
            query: ParticleQuery::parse(text).unwrap(),
            histogram,
        };
        execute(&Grid(100), &request)
    }

    fn ids(frame: &ParticleFrame) -> Vec<u64> {
        frame.rows.iter().map(|r| r.index).collect()
    }

    #[test]
    fn filters_and_stats_cover_all_matches() {
        let f = run("where x >= 5 and y < 2 limit 3", None);
        assert_eq!((f.tick, f.scanned, f.matched), (42, 100, 10));
        assert_eq!(ids(&f), vec![5, 6, 7]);
        let s = f.stat(F::X).unwrap();
        assert_eq!((s.min, s.max), (5.0, 9.0));
        assert!((s.mean - 7.0).abs() < 1e-12);
        assert!((s.stddev - 2f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn sorted_window_uses_offset() {
        let f = run("where x = 3 order by activity desc limit 3 offset 1", None);
        assert_eq!(ids(&f), vec![83, 73, 63]);
        let f = run("order by y asc limit 4", None);
        assert_eq!(ids(&f), vec![0, 1, 2, 3]);
    }

    #[test]
    fn unsorted_offset_skips_matches() {
        let f = run("where y = 1 offset 8", None);
        assert_eq!(ids(&f), vec![18, 19]);
    }

    #[test]
    fn empty_match_has_no_stats() {
        let f = run("where x > 100", Some(HistogramSpec::default()));
        assert_eq!(f.matched, 0);
        assert!(f.stats.is_empty() && f.rows.is_empty());
        assert_eq!(f.histogram.unwrap().max_count(), 0);
    }

    #[test]
    fn histogram_bins_the_same_matches() {
        let spec = HistogramSpec {
            bins_x: 10,
            bins_y: 10,
            value: BinValue::Count,
            ..HistogramSpec::default()
        };
        let f = run("where y < 5", Some(spec));
        let h = f.histogram.unwrap();
        assert_eq!(h.counts.iter().sum::<u32>() as u64, f.matched);
        assert_eq!((h.x_range, h.y_range), ((0.0, 9.0), (0.0, 4.0)));
        assert_eq!(h.count(9, 9), 1);
        assert_eq!(h.count(0, 0), 1);
        assert_eq!(h.value_range, (1.0, 1.0));
    }

    #[test]
    fn histogram_aggregates_values() {
        let spec = HistogramSpec {
            bins_x: 1,
            bins_y: 2,
            x_range: Some((0.0, 10.0)),
            y_range: Some((0.0, 10.0)),
            value: BinValue::Mean(F::Activity),
            ..HistogramSpec::default()
        };
        let h = run("", Some(spec)).histogram.unwrap();
        assert_eq!(h.value(0, 0), 24.5);
        assert_eq!(h.value(0, 1), 74.5);
        let max = HistogramSpec {
            value: BinValue::Max(F::Activity),
            ..spec
        };
        assert_eq!(run("", Some(max)).histogram.unwrap().value(0, 1), 99.0);
    }

    #[test]
    fn degenerate_ranges_widen() {
        let spec = HistogramSpec {
            value: BinValue::Count,
            ..HistogramSpec::default()
        };
        let h = run("where id = 11", Some(spec)).histogram.unwrap();
        assert_eq!(h.x_range, (0.5, 1.5));
        assert_eq!(h.counts.iter().sum::<u32>(), 1);
    }
}
