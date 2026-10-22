use std::fmt;

/// Errors from a [`crate::SimClient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The command requires an active simulation session.
    NoSession,
    /// A session already exists; destroy it first.
    SessionExists,
    InvalidCommand(String),
    Backend(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSession => f.write_str("no active simulation session"),
            Self::SessionExists => f.write_str("a simulation session already exists"),
            Self::InvalidCommand(why) => write!(f, "invalid command: {why}"),
            Self::Backend(why) => write!(f, "backend error: {why}"),
        }
    }
}

impl std::error::Error for ClientError {}
