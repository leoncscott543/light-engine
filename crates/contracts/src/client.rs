use std::fmt;

use crate::bench::{BenchDescriptor, BenchError, BenchEvent, BenchId, BenchRunConfig};
use crate::error::ClientError;
use crate::log::LogEntry;
use crate::process::ProcessInfo;
use crate::sim::{SimCommand, SimSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Synthetic data stand-in.
    Mock,
    /// In-process kernel or runtime.
    Local,
    /// Out-of-process or networked kernel.
    Remote,
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mock => "mock",
            Self::Local => "local",
            Self::Remote => "remote",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendInfo {
    pub name: String,
    pub version: String,
    pub kind: BackendKind,
}

/// Asynchronous notifications produced by a simulation backend.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    /// One or more steps completed; carries the latest state.
    Stepped(SimSnapshot),
    /// Session created, destroyed, started, paused, reset, etc.
    SessionChanged(Option<SimSnapshot>),
    Log(LogEntry),
}

/// Drives a simulation backend.
///
/// The model is poll-based: commands are applied immediately, and
/// [`SimClient::poll`] advances the backend and drains pending events.
/// Front ends call `poll` once per frame.
pub trait SimClient: Send {
    fn info(&self) -> BackendInfo;

    /// Current session state, or `None` when no session exists.
    fn snapshot(&self) -> Option<SimSnapshot>;

    fn command(&mut self, command: SimCommand) -> Result<(), ClientError>;

    /// Processes owned by the simulation backend.
    fn processes(&self) -> Vec<ProcessInfo>;

    fn poll(&mut self) -> Vec<BackendEvent>;
}

/// Drives a benchmark runtime.
///
/// Benchmarks execute independently of any simulation session, on the
/// runtime's own threads. Runs are exclusive: at most one is active.
/// [`BenchClient::poll`] drains progress without blocking.
pub trait BenchClient: Send {
    fn info(&self) -> BackendInfo;

    fn benches(&self) -> Vec<BenchDescriptor>;

    fn run(&mut self, id: &BenchId, config: BenchRunConfig) -> Result<(), BenchError>;

    /// Requests cancellation; the run ends with [`BenchEvent::Cancelled`].
    fn cancel(&mut self) -> Result<(), BenchError>;

    /// The run in flight, from acceptance until its terminal event is polled.
    fn active(&self) -> Option<BenchId>;

    /// Processes owned by the bench runtime.
    fn processes(&self) -> Vec<ProcessInfo>;

    fn poll(&mut self) -> Vec<BenchEvent>;
}
