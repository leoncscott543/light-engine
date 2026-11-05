use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::time::{Duration, Instant};

use light_contracts::{
    BackendEvent, BackendInfo, BackendKind, BenchClient, BenchDescriptor, BenchEvent, BenchId,
    BenchRunConfig, BenchStats, Integrator, Layout, LogEntry, LogLevel, ParticleRequest, ProcessId,
    ProcessInfo, Series, SimCommand, SimConfig, SimConnection, SimSnapshot, SimStatus, SimUpdate,
};

use crate::action::Action;
use crate::explorer::Explorer;

const HISTORY: usize = 240;
const LOG_CAPACITY: usize = 5_000;
const PROCESS_REFRESH: Duration = Duration::from_millis(500);
const TOAST_TTL: Duration = Duration::from_secs(4);
const PAGE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Overview,
    Simulation,
    Particles,
    Benchmarks,
    Processes,
    Logs,
    Telemetry,
}

impl Tab {
    pub const ALL: [Self; 7] = [
        Self::Overview,
        Self::Simulation,
        Self::Particles,
        Self::Benchmarks,
        Self::Processes,
        Self::Logs,
        Self::Telemetry,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Simulation => "Simulation",
            Self::Particles => "Particles",
            Self::Benchmarks => "Benchmarks",
            Self::Processes => "Processes",
            Self::Logs => "Logs",
            Self::Telemetry => "Telemetry",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    fn offset(self, delta: isize) -> Self {
        let n = Self::ALL.len() as isize;
        Self::ALL[(self.index() as isize + delta).rem_euclid(n) as usize]
    }
}

/// Editable fields of the session configuration draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigField {
    Particles,
    Timestep,
    Integrator,
    Layout,
    Rate,
    Seed,
}

impl ConfigField {
    pub const ALL: [Self; 6] = [
        Self::Particles,
        Self::Timestep,
        Self::Integrator,
        Self::Layout,
        Self::Rate,
        Self::Seed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Particles => "particles",
            Self::Timestep => "timestep",
            Self::Integrator => "integrator",
            Self::Layout => "layout",
            Self::Rate => "target rate",
            Self::Seed => "seed",
        }
    }

    pub fn display(self, config: &SimConfig) -> String {
        match self {
            Self::Particles => crate::fmt::count(u64::from(config.particle_count)),
            Self::Timestep => format!("{:e}", config.dt),
            Self::Integrator => config.integrator.to_string(),
            Self::Layout => config.layout.to_string(),
            Self::Rate => match config.target_rate {
                0 => "unthrottled".into(),
                r => format!("{r} steps/s"),
            },
            Self::Seed => format!("{:#x}", config.seed),
        }
    }

    /// Whether a change can be applied to a live session without recreating it.
    pub fn is_live(self) -> bool {
        matches!(self, Self::Timestep)
    }

    fn adjust(self, config: &mut SimConfig, delta: i32) {
        const DT: [f64; 13] = [
            1e-5, 2e-5, 5e-5, 1e-4, 2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2, 5e-2, 1e-1,
        ];
        const RATES: [u32; 8] = [1, 10, 30, 60, 120, 240, 1_000, 0];
        match self {
            Self::Particles => {
                let n = if delta > 0 {
                    config.particle_count.saturating_mul(2)
                } else {
                    config.particle_count / 2
                };
                config.particle_count = n.clamp(64, 1 << 24);
            }
            Self::Timestep => {
                let i = nearest(&DT, config.dt);
                config.dt = DT[step_index(i, DT.len(), delta)];
            }
            Self::Integrator => {
                config.integrator = cycle(&Integrator::ALL, config.integrator, delta)
            }
            Self::Layout => config.layout = cycle(&Layout::ALL, config.layout, delta),
            Self::Rate => config.target_rate = cycle(&RATES, config.target_rate, delta),
            Self::Seed => config.seed = config.seed.wrapping_add_signed(i64::from(delta)),
        }
    }
}

fn nearest(values: &[f64], v: f64) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs()))
        .map_or(0, |(i, _)| i)
}

fn step_index(i: usize, len: usize, delta: i32) -> usize {
    (i as i64 + i64::from(delta)).clamp(0, len as i64 - 1) as usize
}

fn cycle<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let i = values.iter().position(|v| *v == current).unwrap_or(0) as i64;
    values[(i + i64::from(delta)).rem_euclid(values.len() as i64) as usize]
}

fn step_u32(values: &[u32], current: u32, delta: i32) -> u32 {
    let i = values
        .iter()
        .position(|v| *v >= current)
        .unwrap_or(values.len() - 1);
    values[step_index(i, values.len(), delta)]
}

pub struct SimView {
    pub snapshot: Option<SimSnapshot>,
    pub draft: SimConfig,
    pub field: ConfigField,
    /// `(tick, relative energy error)`.
    pub energy: Series<(f64, f64)>,
    pub step_us: Series<f64>,
    pub throughput: Series<f64>,
}

impl SimView {
    pub fn status(&self) -> SimStatus {
        self.snapshot
            .as_ref()
            .map_or(SimStatus::Inactive, |s| s.status.clone())
    }

    /// Draft fields that differ from the live session and need a recreate.
    pub fn pending_fields(&self) -> Vec<ConfigField> {
        let Some(live) = self.snapshot.as_ref().map(|s| &s.config) else {
            return Vec::new();
        };
        ConfigField::ALL
            .into_iter()
            .filter(|f| !f.is_live() && f.display(live) != f.display(&self.draft))
            .collect()
    }

    fn record(&mut self, snapshot: &SimSnapshot) {
        let tick = snapshot.tick as f64;
        if let Some(error) = snapshot.invariants.relative_energy_error_checked() {
            self.energy.push((tick, error));
        }
        self.step_us
            .push(snapshot.perf.step_time.as_secs_f64() * 1e6);
        if snapshot.perf.particles_per_sec.is_finite() && snapshot.perf.particles_per_sec >= 0.0 {
            self.throughput.push(snapshot.perf.particles_per_sec);
        }
    }

    fn clear_history(&mut self) {
        self.energy.clear();
        self.step_us.clear();
        self.throughput.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Finished,
    Cancelled,
    Failed,
}

pub struct BenchRecord {
    pub meta: light_contracts::ObservationMeta,
    pub config: BenchRunConfig,
    pub state: RunState,
    pub samples: Vec<Duration>,
    pub stats: Option<BenchStats>,
    pub error: Option<String>,
}

impl BenchRecord {
    pub fn progress(&self) -> f64 {
        if self.config.samples == 0 {
            return 0.0;
        }
        (self.samples.len() as f64 / f64::from(self.config.samples)).min(1.0)
    }
}

pub struct BenchView {
    pub viewport: crate::widgets::Viewport,
    pub list: Vec<BenchDescriptor>,
    pub selected: usize,
    pub config: BenchRunConfig,
    /// Mirrors [`BenchClient::active`]; the runtime is the source of truth.
    pub active: Option<BenchId>,
    pub records: HashMap<BenchId, BenchRecord>,
}

impl BenchView {
    pub fn selected(&self) -> Option<&BenchDescriptor> {
        self.list.get(self.selected)
    }

    fn apply(&mut self, event: BenchEvent) {
        match event {
            BenchEvent::Started { id, config, meta } => {
                self.records.insert(
                    id,
                    BenchRecord {
                        meta,
                        config,
                        state: RunState::Running,
                        samples: Vec::with_capacity(config.samples as usize),
                        stats: None,
                        error: None,
                    },
                );
            }
            BenchEvent::Sample { id, elapsed, .. } => {
                if let Some(r) = self.records.get_mut(&id) {
                    r.samples.push(elapsed);
                }
            }
            BenchEvent::Finished { id, stats } => {
                if let Some(r) = self.records.get_mut(&id) {
                    r.state = RunState::Finished;
                    r.stats = stats;
                }
            }
            BenchEvent::Cancelled { id } => {
                if let Some(r) = self.records.get_mut(&id) {
                    r.state = RunState::Cancelled;
                }
            }
            BenchEvent::Failed { id, reason } => {
                if let Some(r) = self.records.get_mut(&id) {
                    r.state = RunState::Failed;
                    r.error = Some(reason);
                }
            }
        }
    }
}

pub struct ProcessView {
    pub viewport: crate::widgets::Viewport,
    pub list: Vec<ProcessInfo>,
    pub selected: usize,
    pub cpu_history: HashMap<(String, ProcessId), Series<f64>>,
    last_refresh: Option<Instant>,
}

pub struct LogView {
    pub height: std::cell::Cell<usize>,
    pub entries: VecDeque<LogEntry>,
    pub min_level: LogLevel,
    pub follow: bool,
    /// Lines scrolled up from the newest entry when not following.
    pub scroll: usize,
}

impl LogView {
    pub fn visible(&self) -> impl DoubleEndedIterator<Item = &LogEntry> {
        self.entries.iter().filter(|e| e.level >= self.min_level)
    }

    fn push(&mut self, entry: LogEntry) {
        if self.entries.len() == LOG_CAPACITY {
            self.entries.pop_front();
        }
        if !self.follow && entry.level >= self.min_level {
            self.scroll += 1;
        }
        self.entries.push_back(entry);
        if !self.follow {
            self.scroll = self
                .scroll
                .min(self.visible().count().saturating_sub(self.height.get()));
        }
    }
}

/// The TUI's own frame timing, so the dashboard monitors itself too.
pub struct UiStats {
    pub target_fps: u32,
    pub over_budget: u64,
    pub render_ms: Series<f64>,
    pub frame_interval_ms: Series<f64>,
    last_frame: Option<Instant>,
}

/// Receipt timing uses the local monotonic clock, never unsynchronized UTC.
pub struct FeedStats {
    pub received: u64,
    pub intervals_ms: Series<f64>,
    pub last_received: Option<Instant>,
    last_sample: Option<Instant>,
}

impl Default for FeedStats {
    fn default() -> Self {
        Self {
            received: 0,
            intervals_ms: Series::new(120),
            last_received: None,
            last_sample: None,
        }
    }
}

impl FeedStats {
    fn receive(&mut self, now: Instant) {
        if let Some(last) = self.last_sample.replace(now) {
            let interval = now.duration_since(last).as_secs_f64() * 1e3;
            if interval > 0.0 {
                self.intervals_ms.push(interval);
            }
        }
        self.observe(now);
    }

    fn observe(&mut self, now: Instant) {
        self.last_received = Some(now);
        self.received += 1;
    }

    pub fn hz(&self) -> f64 {
        if self
            .last_sample
            .is_none_or(|last| last.elapsed() > Duration::from_secs(3))
        {
            return 0.0;
        }
        self.intervals_ms
            .mean()
            .filter(|ms| *ms > 0.0)
            .map_or(0.0, |ms| 1e3 / ms)
    }
}

pub fn p95(series: &Series<f64>) -> f64 {
    let mut values: Vec<_> = series.iter().filter(|v| v.is_finite()).collect();
    values.sort_by(f64::total_cmp);
    if values.is_empty() {
        0.0
    } else {
        values[((values.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)]
    }
}

impl UiStats {
    pub fn fps(&self) -> f64 {
        self.frame_interval_ms
            .mean()
            .filter(|ms| *ms > 0.0)
            .map_or(0.0, |ms| 1000.0 / ms)
    }
}

pub struct Toast {
    pub text: String,
    pub level: LogLevel,
    at: Instant,
}

pub struct App {
    pub inspector: crate::inspection::Inspector,
    pub theme: crate::theme::Theme,
    pub snapshot_feed: FeedStats,
    pub particle_feed: FeedStats,
    pub query_response_ms: Series<f64>,
    pub command_response_ms: Series<f64>,
    pub discarded_particle_frames: u64,
    pub delivery: Option<light_contracts::DeliveryStats>,
    pub connection_error: Option<String>,
    pub telemetry_scroll: usize,
    pub telemetry_height: std::cell::Cell<usize>,
    sim_client: Box<dyn SimConnection>,
    bench_client: Box<dyn BenchClient>,
    pub backend: BackendInfo,
    pub bench_backend: BackendInfo,
    pub running: bool,
    pub tab: Tab,
    pub show_help: bool,
    pub mouse: Option<(u16, u16)>,
    pub sim: SimView,
    pub explorer: Explorer,
    pub bench: BenchView,
    pub procs: ProcessView,
    pub logs: LogView,
    pub ui: UiStats,
    pub toast: Option<Toast>,
    pub build_state: light_tui::build_status::BuildState,
    build_monitor: Option<light_tui::build_status::BuildMonitor>,
    started: Instant,
    sim_ready: bool,
    sim_processes: Vec<ProcessInfo>,
    particle_request: Option<(u64, ParticleRequest, u64, Instant)>,
    command_submitted: Option<Instant>,
    session_epoch: u64,
    pending_commands: usize,
}

impl App {
    pub fn new(sim_client: Box<dyn SimConnection>, bench_client: Box<dyn BenchClient>) -> Self {
        let backend = BackendInfo {
            name: "connecting".into(),
            version: String::new(),
            kind: BackendKind::Local,
        };
        let bench_backend = bench_client.info();
        let benches = bench_client.benches();
        let active = bench_client.active();
        let mut app = Self {
            inspector: crate::inspection::Inspector::default(),
            theme: crate::theme::Theme::default(),
            snapshot_feed: FeedStats::default(),
            particle_feed: FeedStats::default(),
            query_response_ms: Series::new(HISTORY),
            command_response_ms: Series::new(HISTORY),
            discarded_particle_frames: 0,
            delivery: None,
            connection_error: None,
            telemetry_scroll: 0,
            telemetry_height: std::cell::Cell::new(PAGE),
            sim_client,
            bench_client,
            backend,
            bench_backend,
            running: true,
            tab: Tab::Overview,
            show_help: false,
            mouse: None,
            sim: SimView {
                snapshot: None,
                draft: SimConfig::default(),
                field: ConfigField::Particles,
                energy: Series::new(HISTORY),
                step_us: Series::new(HISTORY),
                throughput: Series::new(HISTORY),
            },
            explorer: Explorer::default(),
            bench: BenchView {
                viewport: Default::default(),
                list: benches,
                selected: 0,
                config: BenchRunConfig::default(),
                active,
                records: HashMap::new(),
            },
            procs: ProcessView {
                viewport: Default::default(),
                list: Vec::new(),
                selected: 0,
                cpu_history: HashMap::new(),
                last_refresh: None,
            },
            logs: LogView {
                height: std::cell::Cell::new(PAGE),
                entries: VecDeque::new(),
                min_level: LogLevel::Debug,
                follow: true,
                scroll: 0,
            },
            ui: UiStats {
                target_fps: 30,
                over_budget: 0,
                render_ms: Series::new(HISTORY),
                frame_interval_ms: Series::new(60),
                last_frame: None,
            },
            toast: None,
            build_state: light_tui::build_status::BuildState::Standalone,
            build_monitor: None,
            started: Instant::now(),
            sim_ready: false,
            sim_processes: Vec::new(),
            particle_request: None,
            command_submitted: None,
            session_epoch: 0,
            pending_commands: 0,
        };
        app.pump();
        app.refresh_processes(Instant::now());
        app
    }

    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn has_session(&self) -> bool {
        self.sim.snapshot.is_some()
    }

    pub fn snapshot_health(&self) -> &'static str {
        if self.connection_error.is_some() {
            return "FAULTED";
        }
        let Some(snapshot) = &self.sim.snapshot else {
            return if self.sim_ready {
                "NO SESSION"
            } else {
                "CONNECTING"
            };
        };
        if snapshot.status != SimStatus::Running {
            return "HELD";
        }
        let threshold = self.delivery.map_or(Duration::from_secs(1), |d| {
            (d.snapshot_interval * 3).max(Duration::from_secs(1))
        });
        if self
            .snapshot_feed
            .last_received
            .is_none_or(|t| t.elapsed() > threshold)
        {
            "STALE"
        } else {
            "FRESH"
        }
    }

    /// Pumps both backends once. Call every frame.
    pub fn tick(&mut self) {
        if let Some(update) = self
            .build_monitor
            .as_mut()
            .and_then(|monitor| monitor.poll())
        {
            match update {
                Ok(state) => self.build_state = state,
                Err(error) => {
                    self.build_state = light_tui::build_status::BuildState::Unavailable;
                    self.notify(LogLevel::Error, error);
                }
            }
        }
        self.pump();
        let now = Instant::now();
        self.refresh_particles(now);
        if self
            .procs
            .last_refresh
            .is_none_or(|t| now - t >= PROCESS_REFRESH)
        {
            self.refresh_processes(now);
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|t| t.at.elapsed() > TOAST_TTL)
        {
            self.toast = None;
        }
    }

    pub fn monitor_build(&mut self, monitor: light_tui::build_status::BuildMonitor) {
        self.build_monitor = Some(monitor);
        self.build_state = light_tui::build_status::BuildState::Pending;
    }

    pub fn record_frame(&mut self, render: Duration) {
        let now = Instant::now();
        self.ui.render_ms.push(render.as_secs_f64() * 1e3);
        if render.as_secs_f64() > 1.0 / f64::from(self.ui.target_fps) {
            self.ui.over_budget += 1;
        }
        if let Some(last) = self.ui.last_frame.replace(now) {
            self.ui
                .frame_interval_ms
                .push((now - last).as_secs_f64() * 1e3);
        }
    }

    /// Fetches one particle frame (table, stats, and heatmap from the same
    /// state) while the Particles tab is visible.
    fn refresh_particles(&mut self, now: Instant) {
        if self.tab != Tab::Particles || !self.sim_ready || self.pending_commands != 0 {
            return;
        }
        let Some(tick) = self.sim.snapshot.as_ref().map(|s| s.tick) else {
            self.explorer.frame = None;
            return;
        };
        if !self.explorer.needs_refresh(tick, now) {
            return;
        }
        if self.particle_request.is_some() {
            return;
        }
        let request = self.explorer.request();
        match self.sim_client.query_particles(request.clone()) {
            Ok(id) => {
                self.particle_request = Some((id, request, self.session_epoch, now));
                self.explorer.loading = true;
            }
            Err(err) => {
                if self.explorer.fail(now) {
                    self.notify(LogLevel::Error, format!("particle query failed: {err}"));
                }
            }
        }
    }

    fn pump(&mut self) {
        for update in self.sim_client.poll() {
            match update {
                SimUpdate::Telemetry(stats) => self.delivery = Some(stats),
                SimUpdate::Ready { info, snapshot } => {
                    self.backend = info;
                    self.sim_ready = true;
                    self.connection_error = None;
                    if let Some(snapshot) = &snapshot {
                        self.sim.draft = snapshot.config;
                    }
                    self.on_backend(BackendEvent::SessionChanged(snapshot));
                }
                SimUpdate::Backend(event) => self.on_backend(event),
                SimUpdate::Processes(processes) => self.sim_processes = processes,
                SimUpdate::Particles { id, result } => {
                    if self
                        .particle_request
                        .as_ref()
                        .is_some_and(|(pending, _, _, _)| *pending == id)
                    {
                        let (_, request, epoch, submitted) =
                            self.particle_request.take().expect("matched request");
                        self.query_response_ms
                            .push(submitted.elapsed().as_secs_f64() * 1e3);
                        self.explorer.loading = false;
                        // Query/viewport edits while work is in flight invalidate
                        // its result; leave dirty state for the next request.
                        if request != self.explorer.request()
                            || !self.has_session()
                            || epoch != self.session_epoch
                        {
                            self.discarded_particle_frames += 1;
                            continue;
                        }
                        let now = Instant::now();
                        match result {
                            Ok(frame) => {
                                self.particle_feed.receive(now);
                                self.explorer.accept(frame, now);
                            }
                            Err(error) => {
                                if self.explorer.fail(now) {
                                    self.notify(
                                        LogLevel::Error,
                                        format!("particle query failed: {error}"),
                                    );
                                }
                            }
                        }
                    }
                }
                SimUpdate::CommandCompleted(result) => {
                    self.pending_commands = self.pending_commands.saturating_sub(1);
                    if self.pending_commands == 0
                        && let Some(submitted) = self.command_submitted.take()
                    {
                        self.command_response_ms
                            .push(submitted.elapsed().as_secs_f64() * 1e3);
                    }
                    if let Err(error) = result {
                        self.notify(LogLevel::Error, error.to_string());
                    }
                }
                SimUpdate::Faulted(reason) => {
                    self.connection_error = Some(reason.clone());
                    self.sim_ready = false;
                    self.explorer.loading = false;
                    self.particle_request = None;
                    self.pending_commands = 0;
                    self.command_submitted = None;
                    if let Some(snapshot) = &mut self.sim.snapshot {
                        snapshot.status = SimStatus::Faulted(reason.clone());
                    }
                    self.notify(LogLevel::Error, reason);
                }
            }
        }
        for event in self.bench_client.poll() {
            self.on_bench(event);
        }
        self.bench.active = self.bench_client.active();
    }

    fn on_backend(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::Stepped(snapshot) => {
                self.snapshot_feed.receive(Instant::now());
                self.sim.record(&snapshot);
                self.sim.snapshot = Some(snapshot);
            }
            BackendEvent::SessionChanged(snapshot) => {
                if snapshot.is_some() {
                    self.snapshot_feed.observe(Instant::now());
                }
                match &snapshot {
                    None => {
                        self.session_epoch = self.session_epoch.wrapping_add(1);
                        self.sim.clear_history();
                        self.explorer.clear();
                    }
                    Some(s) if s.tick == 0 => {
                        self.session_epoch = self.session_epoch.wrapping_add(1);
                        self.sim.clear_history();
                        self.sim.record(s);
                    }
                    Some(_) => {}
                }
                self.sim.snapshot = snapshot;
            }
            BackendEvent::Log(entry) => self.logs.push(entry),
        }
    }

    fn on_bench(&mut self, event: BenchEvent) {
        let line = match &event {
            BenchEvent::Started { id, config, .. } => Some((
                LogLevel::Info,
                format!(
                    "{id} started: {} samples x {} iters, {} warmup",
                    config.samples, config.iters_per_sample, config.warmup_iters
                ),
            )),
            BenchEvent::Sample { .. } => None,
            BenchEvent::Finished { id, stats } => Some((
                LogLevel::Info,
                match stats {
                    Some(s) => format!(
                        "{id} finished: mean {:?} p99 {:?} ({} samples)",
                        s.mean, s.p99, s.count
                    ),
                    None => format!("{id} finished with no samples"),
                },
            )),
            BenchEvent::Cancelled { id } => Some((LogLevel::Warn, format!("{id} cancelled"))),
            BenchEvent::Failed { id, reason } => {
                Some((LogLevel::Error, format!("{id} failed: {reason}")))
            }
        };
        if let Some((level, message)) = line {
            self.logs.push(LogEntry {
                provenance: light_contracts::Provenance::live("light-tui"),
                at: self.uptime(),
                level,
                source: "bench".into(),
                message,
            });
        }
        self.bench.apply(event);
    }

    fn refresh_processes(&mut self, now: Instant) {
        let selected = self
            .procs
            .list
            .get(self.procs.selected)
            .map(|p| (p.meta.provenance.producer.clone(), p.id));
        self.procs.list = self.sim_processes.clone();
        self.procs.list.extend(self.bench_client.processes());
        self.procs.list.sort_by(|a, b| {
            (&a.meta.provenance.producer, a.id).cmp(&(&b.meta.provenance.producer, b.id))
        });
        if let Some((producer, id)) = selected
            && let Some(index) = self
                .procs
                .list
                .iter()
                .position(|p| p.id == id && p.meta.provenance.producer == producer)
        {
            self.procs.selected = index;
        }
        self.procs.cpu_history.retain(|(producer, id), _| {
            self.procs
                .list
                .iter()
                .any(|p| p.id == *id && p.meta.provenance.producer == *producer)
        });
        for p in &self.procs.list {
            self.procs
                .cpu_history
                .entry((p.meta.provenance.producer.clone(), p.id))
                .or_insert_with(|| Series::new(HISTORY))
                .push(p.cpu);
        }
        self.procs.selected = self
            .procs
            .selected
            .min(self.procs.list.len().saturating_sub(1));
        self.procs.last_refresh = Some(now);
    }

    fn notify(&mut self, level: LogLevel, text: impl Into<String>) {
        let text = text.into();
        self.logs.push(LogEntry {
            provenance: light_contracts::Provenance::live("light-tui"),
            at: self.uptime(),
            level,
            source: "tui".into(),
            message: text.clone(),
        });
        self.toast = Some(Toast {
            text,
            level,
            at: Instant::now(),
        });
    }

    fn report<E: Display>(&mut self, result: Result<(), E>) {
        if let Err(err) = result {
            self.notify(LogLevel::Error, err.to_string());
        }
        // Drain whatever the command produced right away.
        self.pump();
    }

    fn command(&mut self, command: SimCommand) {
        if !self.sim_ready {
            self.notify(LogLevel::Error, "simulation backend is not ready");
            return;
        }
        let invalidates_particles = matches!(
            &command,
            SimCommand::Create(_) | SimCommand::Destroy | SimCommand::Reset
        );
        let result = self.sim_client.command(command);
        if result.is_ok() {
            if self.pending_commands == 0 {
                self.command_submitted = Some(Instant::now());
            }
            self.pending_commands += 1;
            if invalidates_particles {
                self.session_epoch = self.session_epoch.wrapping_add(1);
            }
        }
        self.report(result);
    }

    pub fn update(&mut self, action: Action) {
        if self.inspector.open {
            let delta = match action {
                Action::Up => Some(-1),
                Action::Down => Some(1),
                Action::PageUp => Some(-(PAGE as isize)),
                Action::PageDown => Some(PAGE as isize),
                Action::Home => Some(isize::MIN / 2),
                Action::End => Some(isize::MAX / 2),
                _ => None,
            };
            if let Some(delta) = delta {
                self.inspector.navigate(delta);
                return;
            }
            if !matches!(
                action,
                Action::Noop
                    | Action::Quit
                    | Action::CloseOverlay
                    | Action::ToggleInspector
                    | Action::InspectRefresh
                    | Action::InspectSource
                    | Action::InspectSelect(_)
                    | Action::InspectScroll(_)
                    | Action::Hover(_, _)
            ) {
                return;
            }
        }
        if self.pending_commands != 0
            && matches!(
                action,
                Action::CreateSession
                    | Action::DestroySession
                    | Action::RecreateSession
                    | Action::ToggleRun
                    | Action::Step(_)
                    | Action::ResetSession
                    | Action::AdjustField(ConfigField::Timestep, _)
            )
        {
            self.notify(
                LogLevel::Warn,
                "previous simulation command is still pending",
            );
            return;
        }
        match action {
            Action::CycleTheme(delta) => {
                self.theme = self.theme.cycle(delta);
                self.explorer.colormap = crate::colormap::Colormap::Theme;
            }
            Action::Noop => {}
            Action::Quit => self.running = false,
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::CloseOverlay => {
                self.show_help = false;
                self.inspector.open = false;
            }
            Action::ToggleInspector => {
                self.inspector.open = !self.inspector.open;
                if self.inspector.open {
                    self.show_help = false;
                    self.inspector.particle = self.tab == Tab::Particles;
                    self.refresh_inspector();
                }
            }
            Action::InspectRefresh => self.refresh_inspector(),
            Action::InspectSource => {
                self.inspector.particle = !self.inspector.particle;
                self.inspector.selected = 0;
                self.refresh_inspector();
            }
            Action::InspectSelect(index) => {
                self.inspector.selected = index.min(self.inspector.len().saturating_sub(1));
                self.inspector.detail_scroll = 0;
            }
            Action::InspectScroll(delta) => {
                self.inspector.scroll_detail(delta);
            }
            Action::Hover(x, y) => self.mouse = Some((x, y)),
            Action::NextTab => self.tab = self.tab.offset(1),
            Action::PrevTab => self.tab = self.tab.offset(-1),
            Action::GoTo(tab) => {
                self.tab = tab;
                if tab == Tab::Particles {
                    self.explorer.mark_dirty();
                    self.refresh_particles(Instant::now());
                }
            }

            Action::Up => self.navigate(-1),
            Action::Down => self.navigate(1),
            Action::PageUp => self.navigate(-(self.page_size() as isize)),
            Action::PageDown => self.navigate(self.page_size() as isize),
            Action::Home => self.navigate(isize::MIN / 2),
            Action::End => self.navigate(isize::MAX / 2),
            Action::SelectRow(i) => self.select_row(i),
            Action::ScrollTo(i) => match self.tab {
                Tab::Telemetry => {
                    self.telemetry_scroll = i.min(
                        crate::views::telemetry::lines(self)
                            .len()
                            .saturating_sub(self.telemetry_height.get()),
                    )
                }
                Tab::Logs => {
                    let max = self
                        .logs
                        .visible()
                        .count()
                        .saturating_sub(self.logs.height.get());
                    self.logs.scroll = max.saturating_sub(i);
                    self.logs.follow = self.logs.scroll == 0;
                }
                _ => self.select_row(i),
            },

            Action::CreateSession => self.command(SimCommand::Create(self.sim.draft)),
            Action::DestroySession => self.command(SimCommand::Destroy),
            Action::RecreateSession => {
                if self.has_session() {
                    self.command(SimCommand::Destroy);
                }
                self.command(SimCommand::Create(self.sim.draft));
            }
            Action::ToggleRun => match self.sim.status() {
                SimStatus::Running => self.command(SimCommand::Pause),
                SimStatus::Inactive => {
                    self.command(SimCommand::Create(self.sim.draft));
                    self.command(SimCommand::Start);
                }
                _ => self.command(SimCommand::Start),
            },
            Action::Step(n) => self.command(SimCommand::Step(n)),
            Action::ResetSession => self.command(SimCommand::Reset),
            Action::FocusField(field) => self.sim.field = field,
            Action::AdjustField(field, delta) => {
                self.sim.field = field;
                field.adjust(&mut self.sim.draft, delta);
                if field.is_live() && self.has_session() {
                    self.command(SimCommand::SetTimestep(self.sim.draft.dt));
                }
            }

            Action::RunBench => {
                if let Some(id) = self.bench.selected().map(|d| d.id.clone()) {
                    let result = self.bench_client.run(&id, self.bench.config);
                    self.report(result);
                }
            }
            Action::CancelBench => {
                let result = self.bench_client.cancel();
                self.report(result);
            }
            Action::AdjustSamples(d) => {
                let c = &mut self.bench.config;
                c.samples = step_u32(&[10, 25, 50, 100, 200, 500, 1_000], c.samples, d);
            }
            Action::AdjustWarmup(d) => {
                let c = &mut self.bench.config;
                c.warmup_iters = step_u32(&[0, 1, 5, 10, 50, 100, 500], c.warmup_iters, d);
            }
            Action::AdjustItersPerSample(d) => {
                let c = &mut self.bench.config;
                c.iters_per_sample = step_u32(&[1, 2, 5, 10, 50, 100], c.iters_per_sample, d);
            }

            Action::SetLogLevel(level) => {
                self.logs.min_level = level;
                self.logs.scroll = 0;
            }
            Action::CycleLogLevel => {
                let all = LogLevel::ALL;
                let i = all
                    .iter()
                    .position(|l| *l == self.logs.min_level)
                    .unwrap_or(0);
                self.update(Action::SetLogLevel(all[(i + 1) % all.len()]));
            }
            Action::ToggleFollow => {
                self.logs.follow = !self.logs.follow;
                self.logs.scroll = 0;
            }
            Action::ClearLogs => {
                self.logs.entries.clear();
                self.logs.scroll = 0;
            }

            Action::Explore(action) => {
                self.explorer.apply(action);
                self.refresh_particles(Instant::now());
            }
        }
    }

    fn refresh_inspector(&mut self) {
        self.inspector.refresh(
            self.sim.snapshot.as_ref(),
            self.explorer.frame.as_ref(),
            self.explorer.selected,
        );
    }

    fn navigate(&mut self, delta: isize) {
        let move_in = |i: usize, len: usize| -> usize {
            if len == 0 {
                0
            } else {
                (i as isize)
                    .saturating_add(delta)
                    .clamp(0, len as isize - 1) as usize
            }
        };
        match self.tab {
            Tab::Telemetry => {
                self.telemetry_scroll = (self.telemetry_scroll as isize)
                    .saturating_add(delta)
                    .clamp(
                        0,
                        crate::views::telemetry::lines(self)
                            .len()
                            .saturating_sub(self.telemetry_height.get())
                            as isize,
                    ) as usize;
            }
            Tab::Overview => {}
            Tab::Simulation => {
                let i = self.sim.field_index();
                self.sim.field = ConfigField::ALL[move_in(i, ConfigField::ALL.len())];
            }
            Tab::Particles => {
                let i = move_in(self.explorer.selected, self.explorer.row_count());
                self.explorer.select(i);
            }
            Tab::Benchmarks => {
                self.bench.selected = move_in(self.bench.selected, self.bench.list.len());
            }
            Tab::Processes => {
                self.procs.selected = move_in(self.procs.selected, self.procs.list.len());
            }
            Tab::Logs => {
                let total = self.logs.visible().count();
                // Up (negative) scrolls toward older entries.
                let scroll = (self.logs.scroll as isize)
                    .saturating_sub(delta)
                    .clamp(0, total.saturating_sub(self.logs.height.get()) as isize)
                    as usize;
                self.logs.scroll = scroll;
                self.logs.follow = scroll == 0;
            }
        }
    }

    fn page_size(&self) -> usize {
        match self.tab {
            Tab::Particles => self.explorer.table_viewport.height.get(),
            Tab::Benchmarks => self.bench.viewport.height.get(),
            Tab::Processes => self.procs.viewport.height.get(),
            Tab::Telemetry => self.telemetry_height.get(),
            Tab::Logs => self.logs.height.get(),
            _ => PAGE,
        }
        .max(1)
    }

    fn select_row(&mut self, i: usize) {
        match self.tab {
            Tab::Simulation => {
                if let Some(f) = ConfigField::ALL.get(i) {
                    self.sim.field = *f;
                }
            }
            Tab::Particles => self.explorer.select(i),
            Tab::Benchmarks if i < self.bench.list.len() => self.bench.selected = i,
            Tab::Processes if i < self.procs.list.len() => self.procs.selected = i,
            _ => {}
        }
    }
}

impl SimView {
    pub fn field_index(&self) -> usize {
        ConfigField::ALL
            .iter()
            .position(|f| *f == self.field)
            .unwrap_or(0)
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use light_bench::BenchRuntime;
    use light_contracts::{
        ClientError, ParticleRequest, SimClient, SimCommand, SimConnection, SimUpdate,
    };
    use light_mock::MockClient;

    use super::App;

    /// Deterministic transport fixture. Production always uses the worker.
    pub struct ImmediateConnection {
        backend: MockClient,
        updates: Vec<SimUpdate>,
        next_query: u64,
    }

    impl ImmediateConnection {
        pub fn new(seed: u64) -> Self {
            let backend = MockClient::new(seed);
            let updates = vec![SimUpdate::Ready {
                info: backend.info(),
                snapshot: backend.snapshot(),
            }];
            Self {
                backend,
                updates,
                next_query: 0,
            }
        }
    }

    impl SimConnection for ImmediateConnection {
        fn command(&mut self, command: SimCommand) -> Result<(), ClientError> {
            self.backend.command(command)?;
            self.updates
                .extend(self.backend.poll().into_iter().map(SimUpdate::Backend));
            self.updates.push(SimUpdate::CommandCompleted(Ok(())));
            Ok(())
        }

        fn query_particles(&mut self, request: ParticleRequest) -> Result<u64, ClientError> {
            let id = self.next_query;
            self.next_query += 1;
            self.updates.push(SimUpdate::Particles {
                id,
                result: self.backend.particles(&request),
            });
            Ok(id)
        }

        fn poll(&mut self) -> Vec<SimUpdate> {
            self.updates
                .extend(self.backend.poll().into_iter().map(SimUpdate::Backend));
            self.updates
                .push(SimUpdate::Processes(self.backend.processes()));
            std::mem::take(&mut self.updates)
        }
    }

    /// An app wired to the mock sim and a bench runtime over mock workloads.
    pub fn mock_app(seed: u64) -> App {
        App::new(
            Box::new(ImmediateConnection::new(seed)),
            Box::new(BenchRuntime::new(light_mock::bench_registry(seed))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use light_contracts::{ClientError, SimClient};
    use std::sync::mpsc::{self, Receiver, Sender};

    struct ControlledConnection {
        updates: Receiver<SimUpdate>,
        queries: Sender<(u64, ParticleRequest)>,
        commands: Sender<SimCommand>,
        next_query: u64,
    }

    impl SimConnection for ControlledConnection {
        fn command(&mut self, command: SimCommand) -> Result<(), ClientError> {
            self.commands.send(command).unwrap();
            Ok(())
        }

        fn query_particles(&mut self, request: ParticleRequest) -> Result<u64, ClientError> {
            let id = self.next_query;
            self.next_query += 1;
            self.queries.send((id, request)).unwrap();
            Ok(id)
        }

        fn poll(&mut self) -> Vec<SimUpdate> {
            self.updates.try_iter().collect()
        }
    }

    type ControlledApp = (
        App,
        Sender<SimUpdate>,
        Receiver<(u64, ParticleRequest)>,
        Receiver<SimCommand>,
    );

    fn controlled_app() -> ControlledApp {
        let (updates_tx, updates_rx) = mpsc::channel();
        let (queries_tx, queries_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let backend = light_mock::MockClient::with_session(1, SimConfig::default());
        updates_tx
            .send(SimUpdate::Ready {
                info: backend.info(),
                snapshot: backend.snapshot(),
            })
            .unwrap();
        let app = App::new(
            Box::new(ControlledConnection {
                updates: updates_rx,
                queries: queries_tx,
                commands: commands_tx,
                next_query: 0,
            }),
            Box::new(light_bench::BenchRuntime::builtin()),
        );
        (app, updates_tx, queries_rx, commands_rx)
    }

    fn query_result(request: &ParticleRequest) -> light_contracts::ParticleFrame {
        light_mock::MockClient::with_session(1, SimConfig::default())
            .particles(request)
            .unwrap()
    }

    fn app() -> App {
        testing::mock_app(1)
    }

    #[test]
    fn boots_without_session_and_navigates_everything() {
        let mut app = app();
        assert!(!app.has_session());
        for _ in 0..Tab::ALL.len() {
            app.update(Action::NextTab);
            for a in [
                Action::Up,
                Action::Down,
                Action::PageDown,
                Action::Home,
                Action::End,
            ] {
                app.update(a);
            }
            app.tick();
        }
        assert_eq!(app.tab, Tab::Overview);
        app.update(Action::Step(1));
        assert!(
            app.toast.is_some(),
            "stepping without a session reports an error"
        );
    }

    #[test]
    fn toggle_run_creates_session_when_inactive() {
        let mut app = app();
        app.update(Action::ToggleRun);
        assert_eq!(app.sim.status(), SimStatus::Running);
        app.update(Action::ToggleRun);
        assert_eq!(app.sim.status(), SimStatus::Paused);
        app.update(Action::Step(10));
        assert_eq!(app.sim.snapshot.as_ref().unwrap().tick, 10);
        app.update(Action::DestroySession);
        assert!(!app.has_session());
    }

    #[test]
    fn non_live_edits_are_pending_until_recreate() {
        let mut app = app();
        app.update(Action::CreateSession);
        app.update(Action::AdjustField(ConfigField::Particles, 1));
        assert_eq!(app.sim.pending_fields(), vec![ConfigField::Particles]);
        app.update(Action::AdjustField(ConfigField::Timestep, 1));
        assert_eq!(
            app.sim.snapshot.as_ref().unwrap().config.dt,
            app.sim.draft.dt
        );
        app.update(Action::RecreateSession);
        assert!(app.sim.pending_fields().is_empty());
    }

    #[test]
    fn bench_runs_on_its_own_runtime() {
        let mut app = app();
        app.update(Action::GoTo(Tab::Benchmarks));
        app.update(Action::RunBench);
        let id = app.bench.selected().unwrap().id.clone();
        assert_eq!(app.bench.active.as_ref(), Some(&id));
        // Benchmarks run without any simulation session.
        assert!(!app.has_session());

        app.update(Action::RunBench);
        assert!(app.toast.is_some(), "a second run is rejected while busy");

        app.update(Action::CancelBench);
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.bench.active.is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
            app.tick();
        }
        assert!(app.bench.active.is_none());
        assert_eq!(app.bench.records[&id].state, RunState::Cancelled);
        assert!(
            app.procs
                .list
                .iter()
                .any(|p| p.kind == light_contracts::ProcessKind::Bench),
            "bench runtime reports its worker"
        );
    }

    #[test]
    fn field_adjustments_stay_in_range() {
        let mut c = SimConfig::default();
        for _ in 0..40 {
            ConfigField::Particles.adjust(&mut c, 1);
            ConfigField::Timestep.adjust(&mut c, 1);
        }
        assert_eq!(c.particle_count, 1 << 24);
        assert_eq!(c.dt, 1e-1);
        assert_eq!(step_u32(&[10, 25, 50], 30, -1), 25);
    }

    #[test]
    fn stale_running_data_is_distinct_from_intentionally_held_data() {
        let mut app = app();
        assert_eq!(app.snapshot_health(), "NO SESSION");
        app.update(Action::CreateSession);
        app.snapshot_feed.last_received = Some(Instant::now() - Duration::from_secs(5));
        assert_eq!(app.snapshot_health(), "HELD");
        app.sim.snapshot.as_mut().unwrap().status = SimStatus::Running;
        assert_eq!(app.snapshot_health(), "STALE");
        app.snapshot_feed.last_received = Some(Instant::now());
        assert_eq!(app.snapshot_health(), "FRESH");
        app.connection_error = Some("disconnected".into());
        assert_eq!(app.snapshot_health(), "FAULTED");
    }

    #[test]
    fn log_scroll_keeps_a_full_page_and_survives_ring_eviction() {
        let mut app = app();
        app.tab = Tab::Logs;
        app.logs.height.set(5);
        for i in 0..LOG_CAPACITY + 10 {
            app.logs.push(LogEntry {
                provenance: Default::default(),
                at: Duration::ZERO,
                level: LogLevel::Info,
                source: "test".into(),
                message: i.to_string(),
            });
        }
        app.update(Action::Home);
        assert_eq!(app.logs.scroll, LOG_CAPACITY - 5);
        app.logs.push(LogEntry {
            provenance: Default::default(),
            at: Duration::ZERO,
            level: LogLevel::Info,
            source: "test".into(),
            message: "new".into(),
        });
        assert_eq!(app.logs.scroll, LOG_CAPACITY - 5);
        app.update(Action::End);
        assert_eq!(app.logs.scroll, 0);
        assert!(app.logs.follow);
    }

    #[test]
    fn in_flight_query_edits_discard_old_results_and_schedule_only_the_latest() {
        let (mut app, updates, queries, _commands) = controlled_app();
        app.update(Action::GoTo(Tab::Particles));
        app.tick();
        let (id, request) = queries.try_recv().unwrap();
        assert!(app.explorer.loading);
        app.update(Action::Explore(crate::explorer::ExplorerAction::CycleX(1)));
        app.update(Action::Explore(crate::explorer::ExplorerAction::CycleX(1)));
        assert!(queries.try_recv().is_err(), "do not queue obsolete queries");
        updates
            .send(SimUpdate::Particles {
                id,
                result: Ok(query_result(&request)),
            })
            .unwrap();
        app.tick();
        assert!(
            app.explorer.frame.is_none(),
            "old query must not overwrite the new one"
        );
        let (id, latest) = queries.try_recv().unwrap();
        assert_eq!(latest, app.explorer.request());
        updates
            .send(SimUpdate::Particles {
                id,
                result: Ok(query_result(&latest)),
            })
            .unwrap();
        app.tick();
        assert!(!app.explorer.loading);
        assert_eq!(
            app.explorer
                .frame
                .as_ref()
                .unwrap()
                .histogram
                .as_ref()
                .unwrap()
                .x,
            app.explorer.heat_x
        );
    }

    #[test]
    fn recreation_invalidates_particle_results_from_the_previous_session() {
        let (mut app, updates, queries, _commands) = controlled_app();
        app.update(Action::GoTo(Tab::Particles));
        app.tick();
        let (id, request) = queries.try_recv().unwrap();
        let snapshot = app.sim.snapshot.clone();
        updates
            .send(SimUpdate::Backend(BackendEvent::SessionChanged(None)))
            .unwrap();
        updates
            .send(SimUpdate::Backend(BackendEvent::SessionChanged(snapshot)))
            .unwrap();
        updates
            .send(SimUpdate::Particles {
                id,
                result: Ok(query_result(&request)),
            })
            .unwrap();
        app.tick();
        assert!(app.explorer.frame.is_none());
        assert!(
            queries.try_recv().is_ok(),
            "new session needs a fresh query"
        );
    }

    #[test]
    fn command_acknowledgments_prevent_duplicate_input_and_surface_failures() {
        let (mut app, updates, _queries, commands) = controlled_app();
        app.update(Action::Step(1));
        assert_eq!(commands.try_recv().unwrap(), SimCommand::Step(1));
        app.update(Action::Step(1));
        assert!(commands.try_recv().is_err());
        assert!(app.toast.as_ref().unwrap().text.contains("pending"));
        updates
            .send(SimUpdate::CommandCompleted(Err(ClientError::Backend(
                "test failure".into(),
            ))))
            .unwrap();
        app.tick();
        assert!(app.toast.as_ref().unwrap().text.contains("test failure"));
        app.update(Action::Step(1));
        assert_eq!(commands.try_recv().unwrap(), SimCommand::Step(1));
    }

    #[test]
    fn worker_faults_clear_pending_work_and_remain_visible() {
        let (mut app, updates, queries, _commands) = controlled_app();
        app.update(Action::GoTo(Tab::Particles));
        app.tick();
        queries.try_recv().unwrap();
        updates
            .send(SimUpdate::Faulted("backend disconnected".into()))
            .unwrap();
        app.tick();
        assert!(!app.explorer.loading);
        assert!(app.particle_request.is_none());
        assert!(matches!(app.sim.status(), SimStatus::Faulted(_)));
        assert!(
            app.toast
                .as_ref()
                .unwrap()
                .text
                .contains("backend disconnected")
        );
        assert!(queries.try_recv().is_err());
    }

    #[test]
    fn stalled_backend_initialization_does_not_block_input_or_rendering() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let runtime = light_runtime::SimRuntime::spawn(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            Box::new(light_mock::MockClient::new(1))
        })
        .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let mut app = App::new(
            Box::new(runtime),
            Box::new(light_bench::BenchRuntime::builtin()),
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 35)).unwrap();
        let mut hits = crate::hit::HitMap::default();
        let started = Instant::now();
        for _ in 0..12 {
            app.tick();
            app.update(Action::NextTab);
            terminal
                .draw(|frame| {
                    hits.clear();
                    crate::views::render(&app, frame, &mut hits);
                })
                .unwrap();
        }
        let elapsed = started.elapsed();
        release_tx.send(()).unwrap();
        assert!(
            elapsed < Duration::from_millis(500),
            "UI waited for blocked backend: {elapsed:?}"
        );
    }
}
