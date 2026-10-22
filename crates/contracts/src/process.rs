use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId(pub u32);

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Role of a backend process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessKind {
    Kernel,
    Integrator,
    Bench,
    Sensor,
    Io,
}

impl fmt::Display for ProcessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Kernel => "kernel",
            Self::Integrator => "integrator",
            Self::Bench => "bench",
            Self::Sensor => "sensor",
            Self::Io => "io",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessState {
    Idle,
    Running,
    Stopped,
    Failed(String),
}

impl fmt::Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Running => f.write_str("running"),
            Self::Stopped => f.write_str("stopped"),
            Self::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

/// A worker, thread, or service owned by the backend.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub id: ProcessId,
    pub name: String,
    pub kind: ProcessKind,
    pub state: ProcessState,
    /// Fraction of one core, `0.0..`.
    pub cpu: f64,
    pub memory_bytes: u64,
    pub uptime: Duration,
}
