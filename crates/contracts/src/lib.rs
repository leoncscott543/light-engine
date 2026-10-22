//! Contracts between Light backends (kernel, bench harness, mock data) and
//! front ends (TUI, future GUI/ML APIs).
//!
//! Everything a front end needs to drive the simulation lives here as plain
//! data plus the [`SimClient`] and [`BenchClient`] traits. Front ends must depend only on this
//! crate, never on a concrete backend.

pub mod bench;
pub mod client;
pub mod error;
pub mod explore;
pub mod log;
pub mod metrics;
pub mod particles;
pub mod process;
pub mod query;
pub mod sim;

pub use bench::{BenchDescriptor, BenchError, BenchEvent, BenchId, BenchRunConfig, BenchStats};
pub use client::{BackendEvent, BackendInfo, BackendKind, BenchClient, SimClient};
pub use error::ClientError;
pub use explore::{
    BinValue, ColumnStats, Histogram2d, HistogramSpec, ParticleFrame, ParticleRequest, QueryRow,
    execute,
};
pub use log::{LogEntry, LogLevel};
pub use metrics::Series;
pub use particles::{FIELD_COUNT, ParticleField, ParticleRow, ParticleSource};
pub use process::{ProcessId, ProcessInfo, ProcessKind, ProcessState};
pub use query::{CmpOp, Filter, KEYWORDS, ParticleQuery, QueryError, SortOrder};
pub use sim::{
    Integrator, Invariants, Layout, SimCommand, SimConfig, SimSnapshot, SimStatus, StepPerf,
};
