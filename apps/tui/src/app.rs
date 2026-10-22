use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use light_contracts::{
    BackendEvent, BackendInfo, BenchDescriptor, BenchEvent, BenchId, BenchRunConfig, BenchStats,
    ClientError, Integrator, Layout, LogEntry, LogLevel, ProcessId, ProcessInfo, Series,
    SimClient, SimCommand, SimConfig, SimSnapshot, SimStatus,
};

use crate::action::Action;

const HISTORY: usize = 240;
const LOG_CAPACITY: usize = 5_000;
const PROCESS_REFRESH: Duration = Duration::from_millis(500);
const TOAST_TTL: Duration = Duration::from_secs(4);
const PAGE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Overview,
    Simulation,
    Benchmarks,
    Processes,
    Logs,
}

impl Tab {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Simulation,
        Self::Benchmarks,
        Self::Processes,
        Self::Logs,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Simulation => "Simulation",
            Self::Benchmarks => "Benchmarks",
            Self::Processes => "Processes",
            Self::Logs => "Logs",
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
            Self::Integrator => config.integrator = cycle(&Integrator::ALL, config.integrator, delta),
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
        self.energy
            .push((tick, snapshot.invariants.relative_energy_error()));
        self.step_us
            .push(snapshot.perf.step_time.as_secs_f64() * 1e6);
        self.throughput.push(snapshot.perf.particles_per_sec);
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
}

pub struct BenchRecord {
    pub config: BenchRunConfig,
    pub state: RunState,
    pub samples: Vec<Duration>,
    pub stats: Option<BenchStats>,
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
    pub list: Vec<BenchDescriptor>,
    pub selected: usize,
    pub config: BenchRunConfig,
    pub active: Option<BenchId>,
    pub records: HashMap<BenchId, BenchRecord>,
}

impl BenchView {
    pub fn selected(&self) -> Option<&BenchDescriptor> {
        self.list.get(self.selected)
    }

    fn apply(&mut self, event: BenchEvent) {
        match event {
            BenchEvent::Started { id, config } => {
                self.active = Some(id.clone());
                self.records.insert(
                    id,
                    BenchRecord {
                        config,
                        state: RunState::Running,
                        samples: Vec::with_capacity(config.samples as usize),
                        stats: None,
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
                self.active = None;
            }
            BenchEvent::Cancelled { id } => {
                if let Some(r) = self.records.get_mut(&id) {
                    r.state = RunState::Cancelled;
                }
                self.active = None;
            }
        }
    }
}

pub struct ProcessView {
    pub list: Vec<ProcessInfo>,
    pub selected: usize,
    pub cpu_history: HashMap<ProcessId, Series<f64>>,
    last_refresh: Option<Instant>,
}

pub struct LogView {
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
    }
}

/// The TUI's own frame timing, so the dashboard monitors itself too.
pub struct UiStats {
    pub render_ms: Series<f64>,
    pub frame_interval_ms: Series<f64>,
    last_frame: Option<Instant>,
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
    client: Box<dyn SimClient>,
    pub backend: BackendInfo,
    pub running: bool,
    pub tab: Tab,
    pub show_help: bool,
    pub mouse: Option<(u16, u16)>,
    pub sim: SimView,
    pub bench: BenchView,
    pub procs: ProcessView,
    pub logs: LogView,
    pub ui: UiStats,
    pub toast: Option<Toast>,
    started: Instant,
}

impl App {
    pub fn new(client: Box<dyn SimClient>) -> Self {
        let backend = client.info();
        let snapshot = client.snapshot();
        let draft = snapshot.as_ref().map_or_else(SimConfig::default, |s| s.config);
        let benches = client.benches();
        let active = client.active_bench();
        let mut app = Self {
            client,
            backend,
            running: true,
            tab: Tab::Overview,
            show_help: false,
            mouse: None,
            sim: SimView {
                snapshot,
                draft,
                field: ConfigField::Particles,
                energy: Series::new(HISTORY),
                step_us: Series::new(HISTORY),
                throughput: Series::new(HISTORY),
            },
            bench: BenchView {
                list: benches,
                selected: 0,
                config: BenchRunConfig::default(),
                active,
                records: HashMap::new(),
            },
            procs: ProcessView {
                list: Vec::new(),
                selected: 0,
                cpu_history: HashMap::new(),
                last_refresh: None,
            },
            logs: LogView {
                entries: VecDeque::new(),
                min_level: LogLevel::Debug,
                follow: true,
                scroll: 0,
            },
            ui: UiStats {
                render_ms: Series::new(HISTORY),
                frame_interval_ms: Series::new(60),
                last_frame: None,
            },
            toast: None,
            started: Instant::now(),
        };
        if let Some(s) = app.sim.snapshot.clone() {
            app.sim.record(&s);
        }
        app.refresh_processes(Instant::now());
        app
    }

    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn has_session(&self) -> bool {
        self.sim.snapshot.is_some()
    }

    /// Pumps the backend once. Call every frame.
    pub fn tick(&mut self) {
        for event in self.client.poll() {
            self.on_backend(event);
        }
        let now = Instant::now();
        if self
            .procs
            .last_refresh
            .is_none_or(|t| now - t >= PROCESS_REFRESH)
        {
            self.refresh_processes(now);
        }
        if self.toast.as_ref().is_some_and(|t| t.at.elapsed() > TOAST_TTL) {
            self.toast = None;
        }
    }

    pub fn record_frame(&mut self, render: Duration) {
        let now = Instant::now();
        self.ui.render_ms.push(render.as_secs_f64() * 1e3);
        if let Some(last) = self.ui.last_frame.replace(now) {
            self.ui
                .frame_interval_ms
                .push((now - last).as_secs_f64() * 1e3);
        }
    }

    fn on_backend(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::Stepped(snapshot) => {
                self.sim.record(&snapshot);
                self.sim.snapshot = Some(snapshot);
            }
            BackendEvent::SessionChanged(snapshot) => {
                match &snapshot {
                    None => self.sim.clear_history(),
                    Some(s) if s.tick == 0 => {
                        self.sim.clear_history();
                        self.sim.record(s);
                    }
                    Some(_) => {}
                }
                self.sim.snapshot = snapshot;
            }
            BackendEvent::Bench(event) => self.bench.apply(event),
            BackendEvent::Log(entry) => self.logs.push(entry),
        }
    }

    fn refresh_processes(&mut self, now: Instant) {
        self.procs.list = self.client.processes();
        for p in &self.procs.list {
            self.procs
                .cpu_history
                .entry(p.id)
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

    fn report(&mut self, result: Result<(), ClientError>) {
        if let Err(err) = result {
            self.notify(LogLevel::Error, err.to_string());
        }
        // Commands mutate synchronously; drain their events right away.
        for event in self.client.poll() {
            self.on_backend(event);
        }
    }

    fn command(&mut self, command: SimCommand) {
        let result = self.client.command(command);
        self.report(result);
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Noop => {}
            Action::Quit => self.running = false,
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::CloseOverlay => self.show_help = false,
            Action::Hover(x, y) => self.mouse = Some((x, y)),
            Action::NextTab => self.tab = self.tab.offset(1),
            Action::PrevTab => self.tab = self.tab.offset(-1),
            Action::GoTo(tab) => self.tab = tab,

            Action::Up => self.navigate(-1),
            Action::Down => self.navigate(1),
            Action::PageUp => self.navigate(-(PAGE as isize)),
            Action::PageDown => self.navigate(PAGE as isize),
            Action::Home => self.navigate(isize::MIN / 2),
            Action::End => self.navigate(isize::MAX / 2),
            Action::SelectRow(i) => self.select_row(i),

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
                    let result = self.client.run_bench(&id, self.bench.config);
                    self.report(result);
                }
            }
            Action::CancelBench => {
                let result = self.client.cancel_bench();
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
                let i = all.iter().position(|l| *l == self.logs.min_level).unwrap_or(0);
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
        }
    }

    fn navigate(&mut self, delta: isize) {
        let move_in = |i: usize, len: usize| -> usize {
            if len == 0 {
                0
            } else {
                (i as isize).saturating_add(delta).clamp(0, len as isize - 1) as usize
            }
        };
        match self.tab {
            Tab::Overview => {}
            Tab::Simulation => {
                let i = self.sim.field_index();
                self.sim.field = ConfigField::ALL[move_in(i, ConfigField::ALL.len())];
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
                    .clamp(0, total.saturating_sub(1) as isize) as usize;
                self.logs.scroll = scroll;
                self.logs.follow = scroll == 0;
            }
        }
    }

    fn select_row(&mut self, i: usize) {
        match self.tab {
            Tab::Simulation => {
                if let Some(f) = ConfigField::ALL.get(i) {
                    self.sim.field = *f;
                }
            }
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
mod tests {
    use light_mock::MockClient;

    use super::*;

    fn app() -> App {
        App::new(Box::new(MockClient::new(1)))
    }

    #[test]
    fn boots_without_session_and_navigates_everything() {
        let mut app = app();
        assert!(!app.has_session());
        for _ in 0..Tab::ALL.len() {
            app.update(Action::NextTab);
            for a in [Action::Up, Action::Down, Action::PageDown, Action::Home, Action::End] {
                app.update(a);
            }
            app.tick();
        }
        assert_eq!(app.tab, Tab::Overview);
        app.update(Action::Step(1));
        assert!(app.toast.is_some(), "stepping without a session reports an error");
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
        assert_eq!(app.sim.snapshot.as_ref().unwrap().config.dt, app.sim.draft.dt);
        app.update(Action::RecreateSession);
        assert!(app.sim.pending_fields().is_empty());
    }

    #[test]
    fn bench_run_is_tracked() {
        let mut app = app();
        app.update(Action::GoTo(Tab::Benchmarks));
        app.update(Action::RunBench);
        let id = app.bench.selected().unwrap().id.clone();
        assert_eq!(app.bench.active.as_ref(), Some(&id));
        app.update(Action::CancelBench);
        assert_eq!(app.bench.records[&id].state, RunState::Cancelled);
        assert!(app.bench.active.is_none());
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
}
