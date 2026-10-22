//! Benchmark harness and runtime for the Light kernel.
//!
//! Benchmarking is independent of the simulation: [`BenchRuntime`] owns its
//! own worker thread and implements [`light_contracts::BenchClient`], so front
//! ends drive it exactly like any other backend. Workloads implement [`Bench`]
//! and are registered in a [`Registry`]. No real benchmarks are registered
//! yet; see [`Registry::builtin`].

pub mod bench;
pub mod registry;
pub mod runtime;

pub use bench::Bench;
pub use registry::Registry;
pub use runtime::{BenchRuntime, run_blocking};
