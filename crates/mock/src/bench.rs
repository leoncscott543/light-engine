//! Synthetic benchmark catalog and timed sample generation.

use std::time::Duration;

use light_contracts::{BenchDescriptor, BenchEvent, BenchId, BenchRunConfig, BenchStats};

use crate::rng::Rng;

/// Wall-clock pacing so front ends can watch runs progress.
const WARMUP_ITERS_PER_SEC: f64 = 400.0;
const SAMPLES_PER_SEC: f64 = 45.0;

struct Spec {
    id: &'static str,
    group: &'static str,
    description: &'static str,
    /// Typical per-iteration cost in nanoseconds.
    base_ns: f64,
    /// Relative standard deviation.
    spread: f64,
}

const CATALOG: &[Spec] = &[
    Spec {
        id: "kernel/geometric_product",
        group: "kernel",
        description: "Grade 0-2 multivector geometric product, 1M pairs (mock)",
        base_ns: 2_400_000.0,
        spread: 0.04,
    },
    Spec {
        id: "kernel/hamiltonian_eval",
        group: "kernel",
        description: "Evaluate H(q, p) over 64k particles (mock)",
        base_ns: 410_000.0,
        spread: 0.06,
    },
    Spec {
        id: "layout/integrate_aos",
        group: "layout",
        description: "Leapfrog step, 64k particles, AoS layout (mock)",
        base_ns: 760_000.0,
        spread: 0.08,
    },
    Spec {
        id: "layout/integrate_soa",
        group: "layout",
        description: "Leapfrog step, 64k particles, SoA layout (mock)",
        base_ns: 380_000.0,
        spread: 0.05,
    },
    Spec {
        id: "layout/integrate_aosoa8",
        group: "layout",
        description: "Leapfrog step, 64k particles, AoSoA x8 tiles (mock)",
        base_ns: 178_000.0,
        spread: 0.04,
    },
    Spec {
        id: "spatial/morton_reorder",
        group: "spatial",
        description: "Morton-key sort and tile rebuild, 64k particles (mock)",
        base_ns: 1_150_000.0,
        spread: 0.10,
    },
    Spec {
        id: "sensor/accumulate",
        group: "sensor",
        description: "Accumulate 100k measurement events into a 256x256 image (mock)",
        base_ns: 920_000.0,
        spread: 0.12,
    },
];

pub fn catalog() -> Vec<BenchDescriptor> {
    CATALOG
        .iter()
        .map(|s| BenchDescriptor {
            id: BenchId::from(s.id),
            group: s.group.into(),
            description: s.description.into(),
        })
        .collect()
}

pub struct BenchRun {
    pub id: BenchId,
    config: BenchRunConfig,
    base_ns: f64,
    spread: f64,
    warmup_left: f64,
    sample_budget: f64,
    samples: Vec<Duration>,
}

impl BenchRun {
    pub fn start(id: &BenchId, config: BenchRunConfig) -> Option<(Self, BenchEvent)> {
        let spec = CATALOG.iter().find(|s| s.id == id.0)?;
        let run = Self {
            id: id.clone(),
            config,
            base_ns: spec.base_ns,
            spread: spec.spread,
            warmup_left: f64::from(config.warmup_iters),
            sample_budget: 0.0,
            samples: Vec::with_capacity(config.samples as usize),
        };
        let started = BenchEvent::Started {
            id: id.clone(),
            config,
        };
        Some((run, started))
    }

    /// Advances wall time; returns emitted events and whether the run finished.
    pub fn advance(&mut self, dt: Duration, rng: &mut Rng) -> (Vec<BenchEvent>, bool) {
        let mut events = Vec::new();
        let mut secs = dt.as_secs_f64();

        if self.warmup_left > 0.0 {
            let needed = self.warmup_left / WARMUP_ITERS_PER_SEC;
            if secs < needed {
                self.warmup_left -= secs * WARMUP_ITERS_PER_SEC;
                return (events, false);
            }
            self.warmup_left = 0.0;
            secs -= needed;
        }

        self.sample_budget += secs * SAMPLES_PER_SEC;
        while self.sample_budget >= 1.0 && (self.samples.len() as u32) < self.config.samples {
            self.sample_budget -= 1.0;
            let elapsed = self.sample(rng);
            events.push(BenchEvent::Sample {
                id: self.id.clone(),
                index: self.samples.len() as u32,
                elapsed,
            });
            self.samples.push(elapsed);
        }

        let done = self.samples.len() as u32 >= self.config.samples;
        if done {
            events.push(BenchEvent::Finished {
                id: self.id.clone(),
                stats: BenchStats::from_samples(&self.samples),
            });
        }
        (events, done)
    }

    fn sample(&self, rng: &mut Rng) -> Duration {
        let per_sample = f64::from(self.config.iters_per_sample.max(1));
        // Averaging over more iterations narrows the spread.
        let sigma = self.spread / per_sample.sqrt();
        let mut ns = self.base_ns * (rng.normal() * sigma).exp();
        if rng.chance(0.02) {
            ns *= rng.range(1.4, 2.2);
        }
        Duration::from_nanos(ns.max(1.0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_produces_requested_samples() {
        let id = BenchId::from("layout/integrate_soa");
        let config = BenchRunConfig {
            warmup_iters: 4,
            samples: 20,
            iters_per_sample: 1,
        };
        let (mut run, _) = BenchRun::start(&id, config).unwrap();
        let mut rng = Rng::new(1);
        let (events, done) = run.advance(Duration::from_secs(10), &mut rng);
        assert!(done);
        let samples = events
            .iter()
            .filter(|e| matches!(e, BenchEvent::Sample { .. }))
            .count();
        assert_eq!(samples, 20);
        assert!(matches!(events.last(), Some(BenchEvent::Finished { stats: Some(_), .. })));
    }

    #[test]
    fn unknown_bench_is_rejected() {
        assert!(BenchRun::start(&BenchId::from("nope"), BenchRunConfig::default()).is_none());
    }
}
