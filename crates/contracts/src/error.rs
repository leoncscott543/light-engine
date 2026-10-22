use std::fmt;

use crate::bench::BenchId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The command requires an active simulation session.
    NoSession,
    /// A session already exists; destroy it first.
    SessionExists,
    UnknownBench(BenchId),
    /// Only one benchmark may run at a time.
    BenchBusy(BenchId),
    /// No benchmark is running.
    NoActiveBench,
    InvalidCommand(String),
    Backend(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSession => f.write_str("no active simulation session"),
            Self::SessionExists => f.write_str("a simulation session already exists"),
            Self::UnknownBench(id) => write!(f, "unknown benchmark `{id}`"),
            Self::BenchBusy(id) => write!(f, "benchmark `{id}` is already running"),
            Self::NoActiveBench => f.write_str("no benchmark is running"),
            Self::InvalidCommand(why) => write!(f, "invalid command: {why}"),
            Self::Backend(why) => write!(f, "backend error: {why}"),
        }
    }
}

impl std::error::Error for ClientError {}
