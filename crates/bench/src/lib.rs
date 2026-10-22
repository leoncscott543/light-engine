//! Benchmark harness for the Light kernel.
//!
//! Defines how workloads are registered and executed. All data crossing the
//! crate boundary uses `light_contracts` types so any front end can consume it.
//! No benchmarks are registered yet; see [`Registry::builtin`].

pub mod bench;
pub mod registry;
pub mod runner;

pub use bench::Bench;
pub use registry::Registry;
pub use runner::{Runner, run_blocking};
