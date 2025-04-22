// src/lib.rs

pub mod engine;
pub mod matter;
pub mod ordinem;
pub mod causality;
pub mod manifest;         
pub mod darai;             
pub mod beam;
pub mod aria;
pub mod python_bridge;


// src/lib.rs
use pyo3::prelude::*;

#[pyfunction]
fn get_particle_count() -> usize {
    // return count from ParticleCore
    42
}

