//! Synthetic benchmark workloads for the bench runtime.
//!
//! Each mock bench really occupies the bench worker for a synthetic duration
//! (lognormal noise plus occasional outliers), so the runtime measures it like
//! any real workload.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use light_bench::{Bench, Registry};
use light_contracts::{BenchDescriptor, BenchId};

use crate::rng::Rng;

/// Below this remaining wait, spin instead of sleeping for accuracy.
const SPIN_THRESHOLD: Duration = Duration::from_micros(1_500);
const OUTLIER_CHANCE: f64 = 0.02;

struct Spec {
    id: &'static str,
    group: &'static str,
    description: &'static str,
    /// Typical per-iteration cost in nanoseconds.
    base_ns: f64,
    /// Relative standard deviation of a single iteration.
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

pub struct MockBench {
    spec: &'static Spec,
    rng: Rng,
}

impl Bench for MockBench {
    fn descriptor(&self) -> BenchDescriptor {
        BenchDescriptor {
            id: BenchId::from(self.spec.id),
            group: self.spec.group.into(),
            description: self.spec.description.into(),
        }
    }

    fn iter(&mut self) {
        let mut ns = self.spec.base_ns * (self.rng.normal() * self.spec.spread).exp();
        if self.rng.chance(OUTLIER_CHANCE) {
            ns *= self.rng.range(1.4, 2.2);
        }
        occupy(Duration::from_nanos(ns.max(1.0) as u64));
    }
}

/// Holds the current thread for `cost`: sleeps for the bulk, spins the tail.
fn occupy(cost: Duration) {
    let deadline = Instant::now() + cost;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        if remaining > SPIN_THRESHOLD {
            thread::sleep(remaining - SPIN_THRESHOLD);
        } else {
            std::hint::spin_loop();
        }
    }
}

/// Registry of every mock workload. Each run gets a distinct noise stream
/// derived from `seed`.
pub fn registry(seed: u64) -> Registry {
    let runs = Arc::new(AtomicU64::new(0));
    let mut registry = Registry::new();
    for (index, spec) in CATALOG.iter().enumerate() {
        let runs = Arc::clone(&runs);
        registry.register(move || {
            let run = runs.fetch_add(1, Ordering::Relaxed);
            MockBench {
                spec,
                rng: Rng::new(seed ^ ((index as u64) << 32) ^ run),
            }
        });
    }
    registry
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use light_bench::BenchRuntime;
    use light_contracts::{BenchClient, BenchEvent, BenchRunConfig};

    use super::*;

    #[test]
    fn registers_whole_catalog() {
        let r = registry(1);
        assert_eq!(r.len(), CATALOG.len());
        assert!(r.instantiate(&"layout/integrate_soa".into()).is_some());
    }

    #[test]
    fn occupies_roughly_the_requested_time() {
        let start = Instant::now();
        occupy(Duration::from_millis(3));
        let took = start.elapsed();
        assert!(took >= Duration::from_millis(3), "{took:?}");
    }

    #[test]
    fn runs_on_the_bench_runtime() {
        let mut rt = BenchRuntime::new(registry(7));
        let config = BenchRunConfig {
            warmup_iters: 1,
            samples: 8,
            iters_per_sample: 1,
        };
        rt.run(&"layout/integrate_aosoa8".into(), config).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut events = Vec::new();
        while rt.active().is_some() && Instant::now() < deadline {
            events.extend(rt.poll());
            thread::sleep(Duration::from_millis(1));
        }
        match events.last() {
            Some(BenchEvent::Finished { stats: Some(s), .. }) => {
                assert_eq!(s.count, 8);
                assert!(s.min >= Duration::from_micros(100), "{s:?}");
            }
            other => panic!("unexpected final event: {other:?}"),
        }
    }
}
