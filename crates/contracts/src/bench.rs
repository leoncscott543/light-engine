use std::fmt;
use std::time::Duration;

/// Stable benchmark identifier, e.g. `"kernel/integrate_aosoa"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BenchId(pub String);

impl From<&str> for BenchId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Display for BenchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchDescriptor {
    pub id: BenchId,
    /// Grouping used by front ends, e.g. `"kernel"`, `"layout"`.
    pub group: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchRunConfig {
    pub warmup_iters: u32,
    pub samples: u32,
    /// Iterations timed together per sample; a sample is the per-iteration mean.
    pub iters_per_sample: u32,
}

impl Default for BenchRunConfig {
    fn default() -> Self {
        Self {
            warmup_iters: 10,
            samples: 100,
            iters_per_sample: 1,
        }
    }
}

/// Summary statistics over a set of timing samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchStats {
    pub count: usize,
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub stddev: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl BenchStats {
    /// Returns `None` when `samples` is empty.
    pub fn from_samples(samples: &[Duration]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();

        let secs: Vec<f64> = sorted.iter().map(Duration::as_secs_f64).collect();
        let n = secs.len() as f64;
        let mean = secs.iter().sum::<f64>() / n;
        let variance = secs.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n;

        Some(Self {
            count: sorted.len(),
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            mean: Duration::from_secs_f64(mean),
            stddev: Duration::from_secs_f64(variance.sqrt()),
            p50: percentile(&sorted, 0.50),
            p95: percentile(&sorted, 0.95),
            p99: percentile(&sorted, 0.99),
        })
    }
}

/// Nearest-rank percentile over pre-sorted samples.
fn percentile(sorted: &[Duration], q: f64) -> Duration {
    let rank = (q * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Progress of a benchmark run.
#[derive(Debug, Clone, PartialEq)]
pub enum BenchEvent {
    Started {
        id: BenchId,
        config: BenchRunConfig,
    },
    Sample {
        id: BenchId,
        index: u32,
        elapsed: Duration,
    },
    Finished {
        id: BenchId,
        stats: Option<BenchStats>,
    },
    Cancelled {
        id: BenchId,
    },
}

impl BenchEvent {
    pub fn id(&self) -> &BenchId {
        match self {
            Self::Started { id, .. }
            | Self::Sample { id, .. }
            | Self::Finished { id, .. }
            | Self::Cancelled { id } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn empty_has_no_stats() {
        assert!(BenchStats::from_samples(&[]).is_none());
    }

    #[test]
    fn summarizes_samples() {
        let samples: Vec<_> = (1..=100).rev().map(ms).collect();
        let s = BenchStats::from_samples(&samples).unwrap();
        assert_eq!(s.count, 100);
        assert_eq!(s.min, ms(1));
        assert_eq!(s.max, ms(100));
        assert_eq!(s.p50, ms(50));
        assert_eq!(s.p95, ms(95));
        assert_eq!(s.p99, ms(99));
        assert!((s.mean.as_secs_f64() - 0.0505).abs() < 1e-9);
    }

    #[test]
    fn single_sample() {
        let s = BenchStats::from_samples(&[ms(7)]).unwrap();
        assert_eq!(s.p50, ms(7));
        assert_eq!(s.p99, ms(7));
        assert_eq!(s.stddev, Duration::ZERO);
    }
}
