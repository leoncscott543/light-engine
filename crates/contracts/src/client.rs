use std::fmt;

use crate::bench::{BenchDescriptor, BenchEvent, BenchId, BenchRunConfig};
use crate::error::ClientError;
use crate::log::LogEntry;
use crate::process::ProcessInfo;
use crate::sim::{SimCommand, SimSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Synthetic data stand-in.
    Mock,
    /// In-process kernel.
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

/// Asynchronous notifications produced by a backend.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    /// One or more steps completed; carries the latest state.
    Stepped(SimSnapshot),
    /// Session created, destroyed, started, paused, reset, etc.
    SessionChanged(Option<SimSnapshot>),
    Bench(BenchEvent),
    Log(LogEntry),
}

/// The single interface front ends use to drive any backend.
///
/// The model is poll-based: commands are applied immediately, and
/// [`SimClient::poll`] advances the backend and drains pending events.
/// Front ends call `poll` once per frame.
pub trait SimClient: Send {
    fn info(&self) -> BackendInfo;

    // --- simulation ---

    /// Current session state, or `None` when no session exists.
    fn snapshot(&self) -> Option<SimSnapshot>;

    fn command(&mut self, command: SimCommand) -> Result<(), ClientError>;

    // --- benchmarks ---

    fn benches(&self) -> Vec<BenchDescriptor>;

    fn run_bench(&mut self, id: &BenchId, config: BenchRunConfig) -> Result<(), ClientError>;

    fn cancel_bench(&mut self) -> Result<(), ClientError>;

    /// The currently running benchmark, if any.
    fn active_bench(&self) -> Option<BenchId>;

    // --- backend processes ---

    fn processes(&self) -> Vec<ProcessInfo>;

    // --- event pump ---

    fn poll(&mut self) -> Vec<BackendEvent>;
}
