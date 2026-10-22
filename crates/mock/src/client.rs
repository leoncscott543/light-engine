use std::time::{Duration, Instant};

use light_contracts::{
    BackendEvent, BackendInfo, BackendKind, ClientError, LogEntry, LogLevel, ParticleFrame,
    ParticleRequest, ProcessId, ProcessInfo, ProcessKind, ProcessState, SimClient, SimCommand,
    SimConfig, SimSnapshot, SimStatus, execute,
};

use crate::model::Session;
use crate::rng::Rng;

/// Steps per second used when `SimConfig::target_rate` is `0` (unthrottled).
const UNTHROTTLED_RATE: f64 = 2_000.0;
/// Upper bound on wall time consumed per poll, so a stalled UI doesn't jump.
const MAX_POLL_DT: Duration = Duration::from_millis(250);
const INTEGRATOR_WORKERS: u32 = 4;
const ENERGY_WARN_THRESHOLD: f64 = 1e-3;
const CHECKPOINT_EVERY: u64 = 5_000;

/// Synthetic backend. Time advances with wall clock in [`SimClient::poll`],
/// or explicitly via [`MockClient::advance`] for deterministic use.
pub struct MockClient {
    seed: u64,
    clock: Duration,
    last_poll: Option<Instant>,
    session: Option<Session>,
    step_budget: f64,
    energy_warned: bool,
    events: Vec<BackendEvent>,
}

impl MockClient {
    pub fn new(seed: u64) -> Self {
        let mut client = Self {
            seed,
            clock: Duration::ZERO,
            last_poll: None,
            session: None,
            step_budget: 0.0,
            energy_warned: false,
            events: Vec::new(),
        };
        client.log(
            LogLevel::Info,
            "mock",
            "mock backend online; all data is synthetic",
        );
        client
    }

    /// A client that boots with an idle session already created.
    pub fn with_session(seed: u64, config: SimConfig) -> Self {
        let mut client = Self::new(seed);
        client
            .command(SimCommand::Create(config))
            .expect("fresh client has no session");
        client
    }

    /// Advances virtual time by `dt` and returns drained events.
    pub fn advance(&mut self, dt: Duration) -> Vec<BackendEvent> {
        self.clock += dt;
        self.advance_session(dt);
        std::mem::take(&mut self.events)
    }

    fn advance_session(&mut self, dt: Duration) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if session.status != SimStatus::Running {
            return;
        }
        let rate = match session.config.target_rate {
            0 => UNTHROTTLED_RATE,
            r => f64::from(r),
        };
        self.step_budget += dt.as_secs_f64() * rate;
        let steps = self.step_budget.floor() as u64;
        if steps == 0 {
            return;
        }
        self.step_budget -= steps as f64;

        let before = session.tick;
        session.advance(steps);
        let snapshot = session.snapshot();
        self.events.push(BackendEvent::Stepped(snapshot.clone()));

        if before / CHECKPOINT_EVERY != snapshot.tick / CHECKPOINT_EVERY {
            let msg = format!(
                "checkpoint tick={} t={:.3} dE/E={:.2e}",
                snapshot.tick,
                snapshot.sim_time,
                snapshot.invariants.relative_energy_error()
            );
            self.log(LogLevel::Debug, "sim", msg);
        }
        if !self.energy_warned
            && snapshot.invariants.relative_energy_error() > ENERGY_WARN_THRESHOLD
        {
            self.energy_warned = true;
            let msg = format!(
                "relative energy error exceeded {ENERGY_WARN_THRESHOLD:.0e} at tick {} ({})",
                snapshot.tick, snapshot.config.integrator
            );
            self.log(LogLevel::Warn, "sim", msg);
        }
    }

    fn log(&mut self, level: LogLevel, source: &str, message: impl Into<String>) {
        self.events.push(BackendEvent::Log(LogEntry {
            at: self.clock,
            level,
            source: source.into(),
            message: message.into(),
        }));
    }

    fn session_changed(&mut self) {
        let snapshot = self.snapshot();
        self.events.push(BackendEvent::SessionChanged(snapshot));
    }

    fn session_mut(&mut self) -> Result<&mut Session, ClientError> {
        self.session.as_mut().ok_or(ClientError::NoSession)
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new(0x11_6487)
    }
}

impl SimClient for MockClient {
    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "mock-data".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            kind: BackendKind::Mock,
        }
    }

    fn snapshot(&self) -> Option<SimSnapshot> {
        self.session.as_ref().map(Session::snapshot)
    }

    fn command(&mut self, command: SimCommand) -> Result<(), ClientError> {
        let message = match command {
            SimCommand::Create(config) => {
                if self.session.is_some() {
                    return Err(ClientError::SessionExists);
                }
                validate(&config)?;
                self.session = Some(Session::new(config));
                self.step_budget = 0.0;
                self.energy_warned = false;
                format!(
                    "session created: {} particles, {}, {}, dt={:e}",
                    config.particle_count, config.integrator, config.layout, config.dt
                )
            }
            SimCommand::Destroy => {
                self.session.take().ok_or(ClientError::NoSession)?;
                "session destroyed".into()
            }
            SimCommand::Start => {
                let s = self.session_mut()?;
                if s.status == SimStatus::Running {
                    return Err(ClientError::InvalidCommand("already running".into()));
                }
                s.status = SimStatus::Running;
                "running".into()
            }
            SimCommand::Pause => {
                let s = self.session_mut()?;
                if s.status != SimStatus::Running {
                    return Err(ClientError::InvalidCommand("not running".into()));
                }
                s.status = SimStatus::Paused;
                format!("paused at tick {}", s.tick)
            }
            SimCommand::Step(n) => {
                let s = self.session_mut()?;
                if s.status == SimStatus::Running {
                    return Err(ClientError::InvalidCommand("pause before stepping".into()));
                }
                s.advance(u64::from(n));
                if s.status == SimStatus::Idle {
                    s.status = SimStatus::Paused;
                }
                let snapshot = s.snapshot();
                let msg = format!("stepped {n} -> tick {}", snapshot.tick);
                self.events.push(BackendEvent::Stepped(snapshot));
                msg
            }
            SimCommand::Reset => {
                self.session_mut()?.reset();
                self.step_budget = 0.0;
                self.energy_warned = false;
                "reset to initial state".into()
            }
            SimCommand::SetTimestep(dt) => {
                if !(dt.is_finite() && dt > 0.0) {
                    return Err(ClientError::InvalidCommand(format!(
                        "timestep must be positive and finite, got {dt}"
                    )));
                }
                self.session_mut()?.config.dt = dt;
                format!("timestep set to {dt:e}")
            }
        };
        self.log(LogLevel::Info, "sim", message);
        self.session_changed();
        Ok(())
    }

    fn particles(&self, request: &ParticleRequest) -> Result<ParticleFrame, ClientError> {
        let session = self.session.as_ref().ok_or(ClientError::NoSession)?;
        Ok(execute(&session.particles(), request))
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        let mut rng = Rng::new(self.seed ^ self.clock.as_millis() as u64);
        let uptime = self.clock;
        let sim_state = match self.session.as_ref().map(|s| &s.status) {
            None => ProcessState::Stopped,
            Some(SimStatus::Running) => ProcessState::Running,
            Some(SimStatus::Faulted(why)) => ProcessState::Failed(why.clone()),
            Some(_) => ProcessState::Idle,
        };
        let sim_running = sim_state == ProcessState::Running;
        let sim_mem = self
            .session
            .as_ref()
            .map_or(0, |s| s.snapshot().perf.memory_bytes);
        let mut busy = |active: bool, hi: f64| {
            if active {
                rng.range(hi * 0.7, hi)
            } else {
                rng.range(0.0, 0.01)
            }
        };

        let mut out = vec![ProcessInfo {
            id: ProcessId(1),
            name: "kernel".into(),
            kind: ProcessKind::Kernel,
            state: sim_state.clone(),
            cpu: busy(sim_running, 0.15),
            memory_bytes: sim_mem / 8 + (4 << 20),
            uptime,
        }];
        for w in 0..INTEGRATOR_WORKERS {
            out.push(ProcessInfo {
                id: ProcessId(10 + w),
                name: format!("integrator-{w}"),
                kind: ProcessKind::Integrator,
                state: sim_state.clone(),
                cpu: busy(sim_running, 0.98),
                memory_bytes: sim_mem / u64::from(INTEGRATOR_WORKERS),
                uptime,
            });
        }
        out.push(ProcessInfo {
            id: ProcessId(40),
            name: "sensor-0".into(),
            kind: ProcessKind::Sensor,
            state: ProcessState::Stopped,
            cpu: 0.0,
            memory_bytes: 0,
            uptime: Duration::ZERO,
        });
        out.push(ProcessInfo {
            id: ProcessId(50),
            name: "snapshot-io".into(),
            kind: ProcessKind::Io,
            state: ProcessState::Idle,
            cpu: busy(false, 0.0),
            memory_bytes: 1 << 20,
            uptime,
        });
        out
    }

    fn poll(&mut self) -> Vec<BackendEvent> {
        let now = Instant::now();
        let dt = self
            .last_poll
            .map_or(Duration::ZERO, |last| now - last)
            .min(MAX_POLL_DT);
        self.last_poll = Some(now);
        self.advance(dt)
    }
}

fn validate(config: &SimConfig) -> Result<(), ClientError> {
    if config.particle_count == 0 {
        return Err(ClientError::InvalidCommand(
            "particle_count must be > 0".into(),
        ));
    }
    if !(config.dt.is_finite() && config.dt > 0.0) {
        return Err(ClientError::InvalidCommand(
            "dt must be positive and finite".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boots_without_session() {
        let mut c = MockClient::new(1);
        assert!(c.snapshot().is_none());
        assert_eq!(c.command(SimCommand::Start), Err(ClientError::NoSession));
        assert!(
            c.advance(Duration::from_secs(1))
                .iter()
                .all(|e| matches!(e, BackendEvent::Log(_)))
        );
    }

    #[test]
    fn session_lifecycle() {
        let mut c = MockClient::new(1);
        c.command(SimCommand::Create(SimConfig::default())).unwrap();
        assert_eq!(
            c.command(SimCommand::Create(SimConfig::default())),
            Err(ClientError::SessionExists)
        );
        c.command(SimCommand::Start).unwrap();
        c.advance(Duration::from_secs(1));
        let tick = c.snapshot().unwrap().tick;
        assert_eq!(tick, u64::from(SimConfig::default().target_rate));

        assert!(c.command(SimCommand::Step(1)).is_err());
        c.command(SimCommand::Pause).unwrap();
        c.command(SimCommand::Step(5)).unwrap();
        assert_eq!(c.snapshot().unwrap().tick, tick + 5);

        c.command(SimCommand::Reset).unwrap();
        assert_eq!(c.snapshot().unwrap().tick, 0);
        c.command(SimCommand::Destroy).unwrap();
        assert!(c.snapshot().is_none());
    }

    #[test]
    fn rejects_bad_timestep() {
        let mut c = MockClient::with_session(1, SimConfig::default());
        assert!(c.command(SimCommand::SetTimestep(0.0)).is_err());
        assert!(c.command(SimCommand::SetTimestep(f64::NAN)).is_err());
        c.command(SimCommand::SetTimestep(5e-4)).unwrap();
        assert_eq!(c.snapshot().unwrap().config.dt, 5e-4);
    }

    #[test]
    fn processes_reflect_session() {
        let mut c = MockClient::new(1);
        assert_eq!(c.processes()[0].state, ProcessState::Stopped);
        c.command(SimCommand::Create(SimConfig::default())).unwrap();
        c.command(SimCommand::Start).unwrap();
        assert_eq!(c.processes()[0].state, ProcessState::Running);
    }

    #[test]
    fn particles_require_a_session_and_share_one_tick() {
        use light_contracts::{BinValue, HistogramSpec, ParticleQuery};

        let mut c = MockClient::new(1);
        let request = ParticleRequest {
            query: ParticleQuery::parse("where activity > 0.5 order by energy desc limit 5")
                .unwrap(),
            histogram: Some(HistogramSpec {
                value: BinValue::Count,
                ..HistogramSpec::default()
            }),
        };
        assert_eq!(c.particles(&request), Err(ClientError::NoSession));

        c.command(SimCommand::Create(SimConfig::default())).unwrap();
        c.command(SimCommand::Step(10)).unwrap();
        let frame = c.particles(&request).unwrap();
        assert_eq!(frame.tick, 10);
        assert_eq!(
            frame.scanned,
            u64::from(SimConfig::default().particle_count)
        );
        assert!(frame.matched > 0 && frame.matched < frame.scanned);
        assert_eq!(frame.rows.len(), 5);
        let energies: Vec<f64> = frame
            .rows
            .iter()
            .map(|r| r.get(light_contracts::ParticleField::Energy))
            .collect();
        assert!(energies.windows(2).all(|w| w[0] >= w[1]));
        let h = frame.histogram.unwrap();
        assert_eq!(u64::from(h.counts.iter().sum::<u32>()), frame.matched);
    }
}
