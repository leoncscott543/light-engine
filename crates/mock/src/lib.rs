//! Mock-data implementation of [`light_contracts::SimClient`].
//!
//! Produces plausible, deterministic (seeded) synthetic simulation state,
//! benchmark timings, processes, and logs so front ends can be built before
//! the kernel exists. None of the numbers here are measurements.

mod bench;
mod client;
mod model;
mod rng;

pub use client::MockClient;
