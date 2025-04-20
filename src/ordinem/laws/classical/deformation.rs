// 0_ordinem/laws/classical/deformation.rs
// ===================================================
// Deformation kernels (Hooke's Law, strain, stress, modulus)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Newtons, Pascals, Meters
};

/// Computes force from spring deformation (Hooke's Law): F = -k * Δx
pub fn spring_force(spring_constant: f64, displacement: Meters) -> Newtons {
    Newtons(-spring_constant * displacement.0)
}

/// Computes stress: σ = F / A
pub fn stress(force: Newtons, area: Meters) -> Pascals {
    Pascals(force.0 / area.0)
}

/// Computes strain: ε = ΔL / L₀
pub fn strain(delta_length: Meters, original_length: Meters) -> f64 {
    delta_length.0 / original_length.0
}

/// Computes Young's modulus: E = stress / strain
pub fn youngs_modulus(stress: Pascals, strain: f64) -> Pascals {
    Pascals(stress.0 / strain)
}