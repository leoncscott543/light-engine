//! A dedicated worker owns the backend; front ends only submit and drain data.
//!
//! No backend method or factory is invoked on the front-end thread. Commands
//! are bounded, snapshots coalesce, and notification overflow is reported.
//! No shared lock is held while executing backend code.

use std::collections::VecDeque;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use light_contracts::{
    BackendEvent, ClientError, LogEntry, LogLevel, ParticleRequest, SimClient, SimCommand,
    SimConnection, SimUpdate,
};

/// Worker identity used by the dashboard's terminal-safe panic hook.
pub const WORKER_THREAD: &str = "light-sim";
const COMMAND_CAPACITY: usize = 64;
const UPDATE_CAPACITY: usize = 128;
const COMMAND_BATCH: usize = 16;
const TICK: Duration = Duration::from_millis(10);
const PROCESS_REFRESH: Duration = Duration::from_millis(500);

enum Request {
    Command(SimCommand),
    Particles { id: u64, request: ParticleRequest },
}

struct Outbox {
    events: VecDeque<SimUpdate>,
    dropped: usize,
    pending_step: Option<light_contracts::SimSnapshot>,
    last_snapshot: Option<Instant>,
    stats: light_contracts::DeliveryStats,
    started: Instant,
}

impl Default for Outbox {
    fn default() -> Self {
        Self::new(Duration::from_millis(50))
    }
}

impl Outbox {
    fn new(interval: Duration) -> Self {
        Self {
            events: VecDeque::new(),
            dropped: 0,
            pending_step: None,
            last_snapshot: None,
            started: Instant::now(),
            stats: light_contracts::DeliveryStats {
                queue_capacity: UPDATE_CAPACITY,
                snapshot_interval: interval,
                ..Default::default()
            },
        }
    }

    fn push(&mut self, event: SimUpdate) {
        if let SimUpdate::Backend(BackendEvent::Stepped(snapshot)) = event {
            self.stats.snapshots_produced += 1;
            if self.pending_step.replace(snapshot).is_some() {
                self.stats.snapshots_coalesced += 1;
            }
            return;
        }
        match &event {
            SimUpdate::Ready { snapshot, .. }
            | SimUpdate::Backend(BackendEvent::SessionChanged(snapshot)) => {
                self.stats.snapshots_produced += u64::from(snapshot.is_some());
                if self.pending_step.take().is_some() {
                    self.stats.snapshots_coalesced += 1;
                }
            }
            SimUpdate::Faulted(_) => {
                self.pending_step = None;
            }
            SimUpdate::Processes(_) => {
                self.events
                    .retain(|event| !matches!(event, SimUpdate::Processes(_)));
            }
            _ => {}
        }
        if self.events.len() == UPDATE_CAPACITY {
            let latest_state = self.events.iter().rposition(|event| {
                matches!(
                    event,
                    SimUpdate::Backend(BackendEvent::Stepped(_) | BackendEvent::SessionChanged(_))
                )
            });
            let latest_session = self.events.iter().rposition(|event| {
                matches!(event, SimUpdate::Backend(BackendEvent::SessionChanged(_)))
            });
            // Preserve current truth and request completions under log floods.
            let expendable = self.events.iter().enumerate().position(|(index, event)| {
                Some(index) != latest_state
                    && Some(index) != latest_session
                    && !matches!(
                        event,
                        SimUpdate::Ready { .. }
                            | SimUpdate::Particles { .. }
                            | SimUpdate::CommandCompleted(_)
                            | SimUpdate::Faulted(_)
                    )
            });
            if let Some(index) = expendable {
                self.events.remove(index);
            } else {
                self.dropped += 1;
                return;
            }
            self.dropped += 1;
        }
        self.events.push_back(event);
    }

    fn drain(&mut self) -> Vec<SimUpdate> {
        self.drain_at(Instant::now())
    }

    fn drain_at(&mut self, now: Instant) -> Vec<SimUpdate> {
        let mut events = Vec::with_capacity(self.events.len() + 1);
        if self.dropped != 0 {
            self.stats.notifications_dropped += self.dropped as u64;
            events.push(SimUpdate::Backend(BackendEvent::Log(LogEntry {
                provenance: light_contracts::Provenance::live("light-runtime"),
                at: Duration::ZERO,
                level: LogLevel::Warn,
                source: "sim-runtime".into(),
                message: format!(
                    "frontend fell behind; dropped {} backend notifications",
                    std::mem::take(&mut self.dropped)
                ),
            })));
        }
        events.extend(self.events.drain(..));
        if self
            .last_snapshot
            .is_none_or(|last| now.duration_since(last) >= self.stats.snapshot_interval)
            && let Some(snapshot) = self.pending_step.take()
        {
            events.push(SimUpdate::Backend(BackendEvent::Stepped(snapshot)));
            self.last_snapshot = Some(now);
        }
        self.stats.snapshots_delivered += events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    SimUpdate::Ready {
                        snapshot: Some(_),
                        ..
                    } | SimUpdate::Backend(
                        BackendEvent::Stepped(_) | BackendEvent::SessionChanged(Some(_))
                    )
                )
            })
            .count() as u64;
        self.stats.queue_depth = self.events.len() + usize::from(self.pending_step.is_some());
        self.stats.worker_uptime = now.duration_since(self.started);
        if !events.is_empty() || self.pending_step.is_some() {
            events.push(SimUpdate::Telemetry(self.stats));
        }
        events
    }
}

/// Backend-independent asynchronous endpoint for mock, local, or remote clients.
pub struct SimRuntime {
    requests: SyncSender<Request>,
    outbox: Arc<Mutex<Outbox>>,
    stop: Arc<AtomicBool>,
    worker: JoinHandle<()>,
    next_query: u64,
    pending_query: Option<u64>,
    pending_commands: usize,
    failed: bool,
}

impl SimRuntime {
    /// Construct and initialize the backend entirely on its worker thread.
    pub fn spawn(create: impl FnOnce() -> Box<dyn SimClient> + Send + 'static) -> io::Result<Self> {
        Self::spawn_with_snapshot_rate(20, create)
    }

    /// Cap stepped-snapshot delivery independently of worker execution/UI FPS.
    /// Lifecycle and command completion notifications bypass this cadence.
    pub fn spawn_with_snapshot_rate(
        snapshot_hz: u32,
        create: impl FnOnce() -> Box<dyn SimClient> + Send + 'static,
    ) -> io::Result<Self> {
        if !(1..=120).contains(&snapshot_hz) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "snapshot rate must be 1-120 Hz",
            ));
        }
        let (requests, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        let outbox = Arc::new(Mutex::new(Outbox::new(Duration::from_secs_f64(
            1.0 / f64::from(snapshot_hz),
        ))));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_outbox = Arc::clone(&outbox);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name(WORKER_THREAD.into())
            .spawn(move || {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    drive(create(), receiver, &worker_outbox, &worker_stop);
                }));
                if let Err(payload) = result {
                    let reason = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&str>().copied())
                        .unwrap_or("non-string panic");
                    publish(
                        &worker_outbox,
                        SimUpdate::Faulted(format!("simulation worker panicked: {reason}")),
                    );
                }
            })?;
        Ok(Self {
            requests,
            outbox,
            stop,
            worker,
            next_query: 0,
            pending_query: None,
            pending_commands: 0,
            failed: false,
        })
    }

    fn submit(&self, request: Request) -> Result<(), ClientError> {
        if self.failed {
            return Err(ClientError::Backend("simulation worker has stopped".into()));
        }
        self.requests.try_send(request).map_err(|error| {
            ClientError::Backend(match error {
                TrySendError::Full(_) => "simulation command queue is full; retry later".into(),
                TrySendError::Disconnected(_) => "simulation worker has stopped".into(),
            })
        })?;
        self.worker.thread().unpark();
        Ok(())
    }
}

impl SimConnection for SimRuntime {
    fn command(&mut self, command: SimCommand) -> Result<(), ClientError> {
        if self.pending_commands == COMMAND_CAPACITY {
            return Err(ClientError::Backend(
                "simulation command queue is full; retry later".into(),
            ));
        }
        self.submit(Request::Command(command))?;
        self.pending_commands += 1;
        Ok(())
    }

    fn query_particles(&mut self, request: ParticleRequest) -> Result<u64, ClientError> {
        if self.pending_query.is_some() {
            return Err(ClientError::Backend(
                "particle query already in flight".into(),
            ));
        }
        let id = self.next_query;
        self.submit(Request::Particles { id, request })?;
        self.next_query = self.next_query.wrapping_add(1);
        self.pending_query = Some(id);
        Ok(id)
    }

    fn poll(&mut self) -> Vec<SimUpdate> {
        let mut updates = match self.outbox.try_lock() {
            Ok(mut outbox) => outbox.drain(),
            Err(TryLockError::WouldBlock) => return Vec::new(),
            Err(TryLockError::Poisoned(_)) if !self.failed => {
                self.failed = true;
                return vec![SimUpdate::Faulted(
                    "simulation notification lock poisoned".into(),
                )];
            }
            Err(TryLockError::Poisoned(_)) => return Vec::new(),
        };
        if updates.iter().any(|event| {
            matches!(event, SimUpdate::Particles { id, .. } if Some(*id) == self.pending_query)
        }) {
            self.pending_query = None;
        }
        self.pending_commands -= updates
            .iter()
            .filter(|event| matches!(event, SimUpdate::CommandCompleted(_)))
            .count();
        if updates
            .iter()
            .any(|event| matches!(event, SimUpdate::Faulted(_)))
        {
            self.failed = true;
        }
        if self.worker.is_finished() && !self.failed {
            self.failed = true;
            updates.push(SimUpdate::Faulted(
                "simulation worker exited unexpectedly".into(),
            ));
        }
        updates
    }
}

impl Drop for SimRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.thread().unpark();
        // SimClient calls cannot be forcibly cancelled. Do not hold up UI
        // shutdown/hot reload waiting for an unresponsive backend call.
    }
}

fn publish(outbox: &Mutex<Outbox>, event: SimUpdate) {
    outbox
        .lock()
        .expect("simulation outbox poisoned")
        .push(event);
}

fn drive(
    mut backend: Box<dyn SimClient>,
    requests: Receiver<Request>,
    outbox: &Mutex<Outbox>,
    stop: &AtomicBool,
) {
    publish(
        outbox,
        SimUpdate::Ready {
            info: backend.info(),
            snapshot: backend.snapshot(),
        },
    );
    let mut last_processes = None;
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        for request in requests.try_iter().take(COMMAND_BATCH) {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            match request {
                Request::Command(command) => {
                    let result = backend.command(command);
                    // Preserve lifecycle ordering relative to following queries.
                    for event in backend.poll() {
                        publish(outbox, SimUpdate::Backend(event));
                    }
                    publish(outbox, SimUpdate::CommandCompleted(result));
                }
                Request::Particles { id, request } => {
                    let result = backend.particles(&request);
                    publish(outbox, SimUpdate::Particles { id, result });
                }
            }
        }
        for event in backend.poll() {
            publish(outbox, SimUpdate::Backend(event));
        }
        if last_processes.is_none_or(|last| started.duration_since(last) >= PROCESS_REFRESH) {
            publish(outbox, SimUpdate::Processes(backend.processes()));
            last_processes = Some(started);
        }
        thread::park_timeout(TICK.saturating_sub(started.elapsed()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use light_contracts::{BackendInfo, ParticleFrame, ProcessInfo, SimConfig, SimSnapshot};
    use light_mock::MockClient;
    use std::sync::mpsc::Sender;

    const TEST_TIMEOUT: Duration = Duration::from_secs(3);

    #[derive(Clone, Copy, PartialEq)]
    enum Stage {
        Initialize,
        Poll,
        Command,
        Query,
        Processes,
    }

    struct BlockingClient {
        inner: MockClient,
        stage: Stage,
        entered: Sender<()>,
        release: Mutex<Receiver<()>>,
        used: AtomicBool,
    }

    impl BlockingClient {
        fn block(&self, stage: Stage) {
            if stage == self.stage && !self.used.swap(true, Ordering::Relaxed) {
                self.entered.send(()).unwrap();
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(TEST_TIMEOUT)
                    .unwrap();
            }
        }
    }

    impl SimClient for BlockingClient {
        fn info(&self) -> BackendInfo {
            self.block(Stage::Initialize);
            self.inner.info()
        }

        fn snapshot(&self) -> Option<SimSnapshot> {
            self.inner.snapshot()
        }

        fn command(&mut self, command: SimCommand) -> Result<(), ClientError> {
            self.block(Stage::Command);
            self.inner.command(command)
        }

        fn particles(&self, request: &ParticleRequest) -> Result<ParticleFrame, ClientError> {
            self.block(Stage::Query);
            self.inner.particles(request)
        }

        fn processes(&self) -> Vec<ProcessInfo> {
            self.block(Stage::Processes);
            self.inner.processes()
        }

        fn poll(&mut self) -> Vec<BackendEvent> {
            self.block(Stage::Poll);
            self.inner.poll()
        }
    }

    fn blocked(stage: Stage) -> (SimRuntime, Receiver<()>, Sender<()>) {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let runtime = SimRuntime::spawn(move || {
            Box::new(BlockingClient {
                inner: MockClient::with_session(
                    1,
                    SimConfig {
                        particle_count: 16,
                        ..SimConfig::default()
                    },
                ),
                stage,
                entered: entered_tx,
                release: Mutex::new(release_rx),
                used: AtomicBool::new(false),
            })
        })
        .unwrap();
        (runtime, entered_rx, release_tx)
    }

    fn wait_for(
        runtime: &mut SimRuntime,
        predicate: impl Fn(&SimUpdate) -> bool,
    ) -> Vec<SimUpdate> {
        let deadline = Instant::now() + TEST_TIMEOUT;
        let mut received = Vec::new();
        loop {
            received.extend(runtime.poll());
            if received.iter().any(&predicate) {
                return received;
            }
            assert!(
                Instant::now() < deadline,
                "worker did not publish expected update: {received:?}"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn every_backend_execution_path_is_off_the_frontend_thread() {
        for stage in [
            Stage::Initialize,
            Stage::Poll,
            Stage::Command,
            Stage::Query,
            Stage::Processes,
        ] {
            let (mut runtime, entered, release) = blocked(stage);
            if stage == Stage::Command {
                runtime.command(SimCommand::Step(1)).unwrap();
            } else if stage == Stage::Query {
                runtime.query_particles(ParticleRequest::default()).unwrap();
            }
            entered.recv_timeout(TEST_TIMEOUT).unwrap();
            let started = Instant::now();
            for _ in 0..100 {
                runtime.poll();
            }
            runtime.command(SimCommand::Pause).unwrap();
            assert!(
                started.elapsed() < Duration::from_millis(250),
                "frontend waited for backend"
            );
            let started = Instant::now();
            drop(runtime);
            assert!(
                started.elapsed() < Duration::from_millis(250),
                "shutdown waited for backend"
            );
            release.send(()).unwrap();
        }
    }

    #[test]
    fn command_backpressure_is_bounded_and_explicit() {
        let (mut runtime, entered, release) = blocked(Stage::Initialize);
        entered.recv_timeout(TEST_TIMEOUT).unwrap();
        for _ in 0..COMMAND_CAPACITY {
            runtime.command(SimCommand::Pause).unwrap();
        }
        let error = runtime.command(SimCommand::Pause).unwrap_err();
        assert!(error.to_string().contains("queue is full"));
        drop(runtime);
        release.send(()).unwrap();
    }

    #[test]
    fn initialization_and_commands_publish_asynchronous_results_in_order() {
        let mut runtime = SimRuntime::spawn(|| Box::new(MockClient::new(1))).unwrap();
        runtime
            .command(SimCommand::Create(SimConfig::default()))
            .unwrap();
        runtime.command(SimCommand::Step(7)).unwrap();
        runtime.command(SimCommand::Destroy).unwrap();
        runtime.command(SimCommand::Step(1)).unwrap();
        let updates = wait_for(&mut runtime, |event| {
            matches!(event, SimUpdate::CommandCompleted(Err(_)))
        });
        assert!(matches!(updates.first(), Some(SimUpdate::Ready { .. })));
        let ticks: Vec<_> = updates
            .iter()
            .filter_map(|update| match update {
                SimUpdate::Backend(BackendEvent::SessionChanged(snapshot)) => {
                    Some(snapshot.as_ref().map(|s| s.tick))
                }
                _ => None,
            })
            .collect();
        assert_eq!(ticks, vec![Some(0), Some(7), None]);
        assert!(updates.iter().any(|event| matches!(
            event,
            SimUpdate::CommandCompleted(Err(ClientError::NoSession))
        )));
    }

    #[test]
    fn backend_progresses_without_ui_polling_and_coalesces_snapshots() {
        let mut runtime = SimRuntime::spawn(|| {
            let mut client = MockClient::with_session(
                1,
                SimConfig {
                    target_rate: 1000,
                    ..SimConfig::default()
                },
            );
            client.command(SimCommand::Start).unwrap();
            Box::new(client)
        })
        .unwrap();
        wait_for(&mut runtime, |event| {
            matches!(event, SimUpdate::Ready { .. })
        });
        thread::sleep(Duration::from_millis(60));
        let updates = runtime.poll();
        let ticks: Vec<_> = updates
            .iter()
            .filter_map(|update| match update {
                SimUpdate::Backend(BackendEvent::Stepped(snapshot)) => Some(snapshot.tick),
                _ => None,
            })
            .collect();
        assert_eq!(ticks.len(), 1);
        assert!(ticks[0] > 0);
    }

    #[test]
    fn query_results_are_correlated_and_only_one_can_be_in_flight() {
        let (mut runtime, entered, release) = blocked(Stage::Query);
        let id = runtime.query_particles(ParticleRequest::default()).unwrap();
        entered.recv_timeout(TEST_TIMEOUT).unwrap();
        assert!(runtime.query_particles(ParticleRequest::default()).is_err());
        release.send(()).unwrap();
        let updates = wait_for(
            &mut runtime,
            |event| matches!(event, SimUpdate::Particles { id: result_id, .. } if *result_id == id),
        );
        assert!(updates.iter().any(|event| matches!(event, SimUpdate::Particles { result: Ok(frame), .. } if frame.scanned == 16)));
        let next = runtime.query_particles(ParticleRequest::default()).unwrap();
        assert_ne!(id, next);
    }

    #[test]
    fn query_execution_errors_are_returned_as_data() {
        let mut runtime = SimRuntime::spawn(|| Box::new(MockClient::new(1))).unwrap();
        runtime.query_particles(ParticleRequest::default()).unwrap();
        let updates = wait_for(&mut runtime, |event| {
            matches!(event, SimUpdate::Particles { .. })
        });
        assert!(updates.iter().any(|event| matches!(
            event,
            SimUpdate::Particles {
                result: Err(ClientError::NoSession),
                ..
            }
        )));
    }

    #[test]
    fn backend_panics_become_visible_faults_and_reject_future_commands() {
        let mut runtime = SimRuntime::spawn(|| panic!("test backend fault")).unwrap();
        let updates = wait_for(&mut runtime, |event| matches!(event, SimUpdate::Faulted(_)));
        assert!(updates.iter().any(|event| matches!(event, SimUpdate::Faulted(reason) if reason.contains("test backend fault"))));
        assert!(runtime.command(SimCommand::Pause).is_err());
    }

    #[test]
    fn frontend_never_waits_for_notification_lock() {
        let (mut runtime, entered, release) = blocked(Stage::Initialize);
        entered.recv_timeout(TEST_TIMEOUT).unwrap();
        let outbox = Arc::clone(&runtime.outbox);
        let guard = outbox.lock().unwrap();
        assert!(runtime.poll().is_empty());
        drop(guard);
        drop(runtime);
        release.send(()).unwrap();
    }

    #[test]
    fn notification_overflow_is_bounded_reported_and_preserves_query_completion() {
        let mut outbox = Outbox::default();
        outbox.push(SimUpdate::Particles {
            id: 10,
            result: Err(ClientError::NoSession),
        });
        for _ in 0..COMMAND_CAPACITY {
            outbox.push(SimUpdate::CommandCompleted(Ok(())));
        }
        outbox.push(SimUpdate::Backend(BackendEvent::SessionChanged(None)));
        for _ in 0..UPDATE_CAPACITY * 3 {
            outbox.push(SimUpdate::Backend(BackendEvent::Log(LogEntry {
                provenance: light_contracts::Provenance::live("light-runtime"),
                at: Duration::ZERO,
                level: LogLevel::Debug,
                source: "test".into(),
                message: "noise".into(),
            })));
        }
        assert_eq!(outbox.events.len(), UPDATE_CAPACITY);
        let updates = outbox.drain();
        assert!(updates.len() <= UPDATE_CAPACITY + 2);
        assert!(
            matches!(updates.first(), Some(SimUpdate::Backend(BackendEvent::Log(entry))) if entry.level == LogLevel::Warn && entry.message.contains("dropped"))
        );
        assert!(
            updates
                .iter()
                .any(|event| matches!(event, SimUpdate::Particles { id: 10, .. }))
        );
        assert_eq!(
            updates
                .iter()
                .filter(|event| matches!(event, SimUpdate::CommandCompleted(_)))
                .count(),
            COMMAND_CAPACITY
        );
        assert!(updates.iter().any(|event| matches!(
            event,
            SimUpdate::Backend(BackendEvent::SessionChanged(None))
        )));
        assert!(outbox.drain().is_empty());
    }

    #[test]
    fn delivery_cadence_coalesces_without_delaying_lifecycle_or_completions() {
        let start = Instant::now();
        let mut outbox = Outbox::new(Duration::from_millis(50));
        let backend = MockClient::with_session(7, SimConfig::default());
        let mut snapshot = backend.snapshot().unwrap();
        snapshot.tick = 1;
        outbox.push(SimUpdate::Backend(BackendEvent::Stepped(snapshot.clone())));
        let first = outbox.drain_at(start);
        assert!(
            first
                .iter()
                .any(|u| matches!(u, SimUpdate::Backend(BackendEvent::Stepped(s)) if s.tick == 1))
        );
        for tick in 2..=10 {
            snapshot.tick = tick;
            outbox.push(SimUpdate::Backend(BackendEvent::Stepped(snapshot.clone())));
        }
        outbox.push(SimUpdate::CommandCompleted(Ok(())));
        let early = outbox.drain_at(start + Duration::from_millis(49));
        assert!(
            early
                .iter()
                .any(|u| matches!(u, SimUpdate::CommandCompleted(Ok(()))))
        );
        assert!(
            !early
                .iter()
                .any(|u| matches!(u, SimUpdate::Backend(BackendEvent::Stepped(_))))
        );
        let due = outbox.drain_at(start + Duration::from_millis(50));
        assert!(
            due.iter()
                .any(|u| matches!(u, SimUpdate::Backend(BackendEvent::Stepped(s)) if s.tick == 10))
        );
        assert_eq!(outbox.stats.snapshots_produced, 10);
        assert_eq!(outbox.stats.snapshots_delivered, 2);
        assert_eq!(outbox.stats.snapshots_coalesced, 8);
        snapshot.tick = 11;
        outbox.push(SimUpdate::Backend(BackendEvent::Stepped(snapshot)));
        outbox.push(SimUpdate::Backend(BackendEvent::SessionChanged(None)));
        let destroyed = outbox.drain_at(start + Duration::from_millis(51));
        assert!(
            destroyed
                .iter()
                .any(|u| matches!(u, SimUpdate::Backend(BackendEvent::SessionChanged(None))))
        );
        assert!(
            outbox.pending_step.is_none(),
            "previous session cannot reappear after destruction"
        );
    }
}
