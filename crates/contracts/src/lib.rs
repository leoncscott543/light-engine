//! Contracts between Light backends (kernel, bench harness, mock data) and
//! front ends (TUI, future GUI/ML APIs).
//!
//! Everything a front end needs to drive the simulation lives here as plain
//! data plus the nonblocking [`SimConnection`] and [`BenchClient`] traits.
//! [`SimClient`] is the backend execution interface, owned by a runtime worker.
//! Front ends depend only on these contracts, never on a concrete backend.

pub mod bench;
pub mod bits;
pub mod client;
pub mod connection;
pub mod error;
pub mod explore;
pub mod log;
pub mod metrics;
pub mod observation;
pub mod particles;
pub mod process;
pub mod query;
pub mod registers;
pub mod sim;

pub use bench::{BenchDescriptor, BenchError, BenchEvent, BenchId, BenchRunConfig, BenchStats};
pub use bits::{Binary64, BitPattern, BitPatternError, MAX_CAPTURE_BITS};
pub use client::{BackendEvent, BackendInfo, BackendKind, BenchClient, SimClient};
pub use connection::{SimConnection, SimUpdate};
pub use error::ClientError;
pub use explore::{
    BinValue, ColumnStats, Histogram2d, HistogramSpec, ParticleFrame, ParticleRequest, QueryRow,
    execute,
};
pub use log::{LogEntry, LogLevel};
pub use metrics::Series;
pub use observation::{
    DataOrigin, DataQuality, DeliveryStats, FieldMeta, OBSERVATION_SCHEMA_VERSION, ObservationMeta,
    Provenance,
};
pub use particles::{FIELD_COUNT, ParticleField, ParticleRow, ParticleSource};
pub use process::{ProcessId, ProcessInfo, ProcessKind, ProcessState};
pub use query::{CmpOp, Filter, KEYWORDS, ParticleQuery, QueryError, SortOrder};
pub use registers::{
    RegisterBinding, RegisterCapability, RegisterCaptureMethod, RegisterContext,
    RegisterObservation, RegisterObservationError, RegisterSampling, RegisterScope, RegisterValue,
    ValueBitReference,
};
pub use sim::{
    Integrator, Invariants, Layout, SimCommand, SimConfig, SimSnapshot, SimStatus, StepPerf,
};
