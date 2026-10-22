//! Mock-data stand-ins for the Light kernel.
//!
//! - [`MockClient`] implements [`light_contracts::SimClient`] with plausible,
//!   deterministic (seeded) synthetic simulation state, processes, and logs.
//! - [`bench_registry`] provides synthetic workloads for
//!   [`light_bench::BenchRuntime`], which executes them like real benchmarks.
//!
//! None of the numbers here describe the real kernel.

mod bench;
mod client;
mod model;
mod particles;
mod rng;

pub use bench::{MockBench, registry as bench_registry};
pub use client::MockClient;
