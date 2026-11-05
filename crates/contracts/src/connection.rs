//! Nonblocking front-end transport, separate from backend execution.

use crate::{
    BackendEvent, BackendInfo, ClientError, ParticleFrame, ParticleRequest, ProcessInfo,
    SimCommand, SimSnapshot,
};

/// Notifications drained by a front end without performing backend work.
#[derive(Debug)]
pub enum SimUpdate {
    Telemetry(crate::DeliveryStats),
    Ready {
        info: BackendInfo,
        snapshot: Option<SimSnapshot>,
    },
    Backend(BackendEvent),
    Processes(Vec<ProcessInfo>),
    Particles {
        id: u64,
        result: Result<ParticleFrame, ClientError>,
    },
    CommandCompleted(Result<(), ClientError>),
    Faulted(String),
}

/// UI-facing endpoint. Every method must return without waiting for backend
/// execution, I/O, or a shared lock. A successful submission acknowledges
/// queue acceptance, not command completion. Execution failures arrive in
/// [`SimUpdate`]. Implementations must bound their queues and drain batches.
pub trait SimConnection: Send {
    fn command(&mut self, command: SimCommand) -> Result<(), ClientError>;

    /// Submit one query and return its correlation id.
    fn query_particles(&mut self, request: ParticleRequest) -> Result<u64, ClientError>;

    fn poll(&mut self) -> Vec<SimUpdate>;
}
