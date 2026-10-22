//! The benchmark runtime: a dedicated worker thread that owns execution.

use std::any::Any;
use std::hint::black_box;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use light_contracts::{
    BackendInfo, BackendKind, BenchClient, BenchDescriptor, BenchError, BenchEvent, BenchId,
    BenchRunConfig, BenchStats, ProcessId, ProcessInfo, ProcessKind, ProcessState,
};

use crate::bench::Bench;
use crate::registry::Registry;

/// Process id the runtime reports for its worker thread.
pub const WORKER_PROCESS_ID: ProcessId = ProcessId(30);

struct Job {
    bench: Box<dyn Bench>,
    config: BenchRunConfig,
    cancel: Arc<AtomicBool>,
}

struct ActiveRun {
    id: BenchId,
    cancel: Arc<AtomicBool>,
}

/// Owns the benchmark registry and a persistent worker thread.
///
/// Runs are queued to the worker and stream [`BenchEvent`]s back over a
/// channel, so benchmarking never blocks or shares a thread with the
/// simulation or the front end. Dropping the runtime cancels any run and
/// joins the worker.
pub struct BenchRuntime {
    name: String,
    registry: Registry,
    jobs: Option<Sender<Job>>,
    events: Receiver<BenchEvent>,
    worker: Option<JoinHandle<()>>,
    active: Option<ActiveRun>,
    started: Instant,
}

impl BenchRuntime {
    pub fn new(registry: Registry) -> Self {
        Self::named("light-bench", registry)
    }

    /// A runtime that reports `name` in [`BenchClient::info`].
    pub fn named(name: impl Into<String>, registry: Registry) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (event_tx, event_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("light-bench".into())
            .spawn(move || {
                for job in job_rx {
                    execute(job, &event_tx);
                }
            })
            .expect("failed to spawn benchmark worker");
        Self {
            name: name.into(),
            registry,
            jobs: Some(job_tx),
            events: event_rx,
            worker: Some(worker),
            active: None,
            started: Instant::now(),
        }
    }

    /// Runtime over [`Registry::builtin`].
    pub fn builtin() -> Self {
        Self::new(Registry::builtin())
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl BenchClient for BenchRuntime {
    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: self.name.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
            kind: BackendKind::Local,
        }
    }

    fn benches(&self) -> Vec<BenchDescriptor> {
        self.registry.descriptors().cloned().collect()
    }

    fn run(&mut self, id: &BenchId, config: BenchRunConfig) -> Result<(), BenchError> {
        if let Some(active) = &self.active {
            return Err(BenchError::Busy(active.id.clone()));
        }
        config.validate()?;
        let bench = self
            .registry
            .instantiate(id)
            .ok_or_else(|| BenchError::Unknown(id.clone()))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            bench,
            config,
            cancel: Arc::clone(&cancel),
        };
        self.jobs
            .as_ref()
            .and_then(|jobs| jobs.send(job).ok())
            .ok_or_else(|| BenchError::Runtime("worker has shut down".into()))?;
        self.active = Some(ActiveRun {
            id: id.clone(),
            cancel,
        });
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), BenchError> {
        let active = self.active.as_ref().ok_or(BenchError::NotRunning)?;
        active.cancel.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn active(&self) -> Option<BenchId> {
        self.active.as_ref().map(|a| a.id.clone())
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        let alive = self.worker.as_ref().is_some_and(|w| !w.is_finished());
        let busy = alive && self.active.is_some();
        vec![ProcessInfo {
            id: WORKER_PROCESS_ID,
            name: "bench-worker".into(),
            kind: ProcessKind::Bench,
            state: match (alive, busy) {
                (false, _) => ProcessState::Failed("worker exited".into()),
                (true, true) => ProcessState::Running,
                (true, false) => ProcessState::Idle,
            },
            // The worker spins a full core while a run is in flight.
            cpu: if busy { 1.0 } else { 0.0 },
            memory_bytes: 0,
            uptime: self.started.elapsed(),
        }]
    }

    fn poll(&mut self) -> Vec<BenchEvent> {
        let events: Vec<_> = self.events.try_iter().collect();
        if let Some(active) = &self.active
            && events
                .iter()
                .any(|e| e.is_terminal() && e.id() == &active.id)
        {
            self.active = None;
        }
        events
    }
}

impl Drop for BenchRuntime {
    fn drop(&mut self) {
        if let Some(active) = &self.active {
            active.cancel.store(true, Ordering::Relaxed);
        }
        // Closing the job channel ends the worker loop.
        self.jobs.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Runs one job, converting a panicking workload into [`BenchEvent::Failed`].
fn execute(job: Job, tx: &Sender<BenchEvent>) {
    let id = job.bench.descriptor().id;
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        run_blocking(job.bench, job.config, &job.cancel, tx);
    }));
    if let Err(payload) = result {
        let _ = tx.send(BenchEvent::Failed {
            id,
            reason: panic_message(payload.as_ref()),
        });
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "benchmark panicked".into())
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

    struct Sleepy;

    impl Bench for Sleepy {
        fn descriptor(&self) -> BenchDescriptor {
            BenchDescriptor {
                id: "test/sleepy".into(),
                group: "test".into(),
                description: "slow iterations".into(),
            }
        }

        fn iter(&mut self) {
            thread::sleep(Duration::from_millis(2));
        }
    }

    struct Boom;

    impl Bench for Boom {
        fn descriptor(&self) -> BenchDescriptor {
            BenchDescriptor {
                id: "test/boom".into(),
                group: "test".into(),
                description: "always panics".into(),
            }
        }

        fn iter(&mut self) {
            panic!("boom");
        }
    }

    fn runtime() -> BenchRuntime {
        let mut registry = Registry::new();
        registry
            .register(|| Spin(0))
            .register(|| Sleepy)
            .register(|| Boom);
        BenchRuntime::new(registry)
    }

    /// Polls until a terminal event arrives, returning everything seen.
    fn drain(rt: &mut BenchRuntime) -> Vec<BenchEvent> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut seen = Vec::new();
        while Instant::now() < deadline {
            seen.extend(rt.poll());
            if seen.last().is_some_and(BenchEvent::is_terminal) {
                return seen;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("run did not finish: {seen:?}");
    }

    #[test]
    fn run_blocking_streams_samples_then_finishes() {
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
    fn run_blocking_honors_cancellation() {
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
    fn runtime_runs_exclusively_and_is_reusable() {
        let mut rt = runtime();
        assert_eq!(rt.benches().len(), 3);
        let spin = BenchId::from("test/spin");
        rt.run(&spin, BenchRunConfig::default()).unwrap();
        assert_eq!(rt.active(), Some(spin.clone()));
        assert_eq!(
            rt.run(&spin, BenchRunConfig::default()),
            Err(BenchError::Busy(spin.clone()))
        );
        assert!(matches!(
            drain(&mut rt).last(),
            Some(BenchEvent::Finished { .. })
        ));
        assert_eq!(rt.active(), None);
        assert_eq!(rt.cancel(), Err(BenchError::NotRunning));

        // The same worker accepts further runs.
        rt.run(&spin, BenchRunConfig::default()).unwrap();
        assert!(matches!(
            drain(&mut rt).last(),
            Some(BenchEvent::Finished { .. })
        ));
    }

    #[test]
    fn runtime_cancels() {
        let mut rt = runtime();
        let config = BenchRunConfig {
            warmup_iters: 0,
            samples: 1_000,
            iters_per_sample: 1,
        };
        rt.run(&"test/sleepy".into(), config).unwrap();
        rt.cancel().unwrap();
        assert!(matches!(
            drain(&mut rt).last(),
            Some(BenchEvent::Cancelled { .. })
        ));
        assert_eq!(rt.active(), None);
    }

    #[test]
    fn runtime_survives_panicking_bench() {
        let mut rt = runtime();
        rt.run(&"test/boom".into(), BenchRunConfig::default())
            .unwrap();
        match drain(&mut rt).last() {
            Some(BenchEvent::Failed { reason, .. }) => assert_eq!(reason, "boom"),
            other => panic!("expected failure, got {other:?}"),
        }
        assert_eq!(rt.processes()[0].state, ProcessState::Idle);
        rt.run(&"test/spin".into(), BenchRunConfig::default())
            .unwrap();
        assert!(matches!(
            drain(&mut rt).last(),
            Some(BenchEvent::Finished { .. })
        ));
    }

    #[test]
    fn runtime_rejects_bad_requests() {
        let mut rt = runtime();
        assert_eq!(
            rt.run(&"nope".into(), BenchRunConfig::default()),
            Err(BenchError::Unknown("nope".into()))
        );
        let zero = BenchRunConfig {
            samples: 0,
            ..BenchRunConfig::default()
        };
        assert!(matches!(
            rt.run(&"test/spin".into(), zero),
            Err(BenchError::InvalidConfig(_))
        ));
        assert_eq!(rt.active(), None);
    }

    #[test]
    fn builtin_registry_is_empty() {
        assert!(BenchRuntime::builtin().benches().is_empty());
    }
}
