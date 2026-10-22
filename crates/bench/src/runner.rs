use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use light_contracts::{BenchEvent, BenchRunConfig, BenchStats};

use crate::bench::Bench;

/// Runs a benchmark on a worker thread and streams [`BenchEvent`]s back.
pub struct Runner {
    events: Receiver<BenchEvent>,
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn spawn(bench: Box<dyn Bench>, config: BenchRunConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let handle = thread::Builder::new()
            .name("light-bench".into())
            .spawn(move || run_blocking(bench, config, &flag, &tx))
            .expect("failed to spawn benchmark thread");
        Self {
            events: rx,
            cancel,
            handle: Some(handle),
        }
    }

    /// Drains events produced since the last call without blocking.
    pub fn poll(&self) -> Vec<BenchEvent> {
        self.events.try_iter().collect()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.cancel();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Runs a benchmark on the current thread. Sends are best-effort so a dropped
/// receiver never panics the worker.
pub fn run_blocking(
    mut bench: Box<dyn Bench>,
    config: BenchRunConfig,
    cancel: &AtomicBool,
    tx: &Sender<BenchEvent>,
) {
    let id = bench.descriptor().id;
    let _ = tx.send(BenchEvent::Started {
        id: id.clone(),
        config,
    });

    bench.setup();
    for _ in 0..config.warmup_iters {
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(BenchEvent::Cancelled { id });
            return;
        }
        bench.iter();
    }

    let per_sample = config.iters_per_sample.max(1);
    let mut samples = Vec::with_capacity(config.samples as usize);
    for index in 0..config.samples {
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(BenchEvent::Cancelled { id });
            return;
        }
        let start = Instant::now();
        for _ in 0..per_sample {
            bench.iter();
        }
        let elapsed = start.elapsed() / per_sample;
        samples.push(elapsed);
        let _ = tx.send(BenchEvent::Sample {
            id: id.clone(),
            index,
            elapsed,
        });
    }

    black_box(&mut bench);
    let _ = tx.send(BenchEvent::Finished {
        id,
        stats: BenchStats::from_samples(&samples),
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use light_contracts::BenchDescriptor;

    use super::*;

    struct Spin(u64);

    impl Bench for Spin {
        fn descriptor(&self) -> BenchDescriptor {
            BenchDescriptor {
                id: "test/spin".into(),
                group: "test".into(),
                description: "busy loop".into(),
            }
        }

        fn iter(&mut self) {
            self.0 = (0..1_000u64).fold(self.0, |a, b| black_box(a.wrapping_add(b)));
        }
    }

    #[test]
    fn streams_samples_then_finishes() {
        let config = BenchRunConfig {
            warmup_iters: 2,
            samples: 5,
            iters_per_sample: 3,
        };
        let (tx, rx) = mpsc::channel();
        run_blocking(Box::new(Spin(0)), config, &AtomicBool::new(false), &tx);
        let events: Vec<_> = rx.try_iter().collect();

        assert!(matches!(events.first(), Some(BenchEvent::Started { .. })));
        let samples = events
            .iter()
            .filter(|e| matches!(e, BenchEvent::Sample { .. }))
            .count();
        assert_eq!(samples, 5);
        match events.last() {
            Some(BenchEvent::Finished { stats: Some(s), .. }) => assert_eq!(s.count, 5),
            other => panic!("unexpected final event: {other:?}"),
        }
    }

    #[test]
    fn honors_cancellation() {
        let (tx, rx) = mpsc::channel();
        run_blocking(
            Box::new(Spin(0)),
            BenchRunConfig::default(),
            &AtomicBool::new(true),
            &tx,
        );
        let events: Vec<_> = rx.try_iter().collect();
        assert!(matches!(events.last(), Some(BenchEvent::Cancelled { .. })));
    }

    #[test]
    fn threaded_runner_completes() {
        let runner = Runner::spawn(Box::new(Spin(0)), BenchRunConfig::default());
        while !runner.is_finished() {
            thread::sleep(Duration::from_millis(1));
        }
        let events = runner.poll();
        assert!(matches!(events.last(), Some(BenchEvent::Finished { .. })));
    }

    #[test]
    fn builtin_registry_is_empty() {
        assert!(crate::Registry::builtin().is_empty());
    }
}
