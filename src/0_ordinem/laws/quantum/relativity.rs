// 0_ordinem/laws/quantum/relativity.rs
// ===================================================
// Relativity kernels (time dilation, mass-energy, Lorentz factor)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Seconds, MetersPerSecond, Joules, Kilograms, Radians
};
use crate::ordinem::statics::constants::C;

/// Computes time dilation: t' = t / √(1 - v² / c²)
pub fn time_dilation(proper_time: Seconds, velocity: MetersPerSecond) -> Seconds {
    let factor = (1.0 - (velocity.0 * velocity.0) / (C * C)).sqrt();
    Seconds(proper_time.0 / factor)
}

/// Computes length contraction: L = L₀ * √(1 - v² / c²)
pub fn length_contraction(proper_length: f64, velocity: MetersPerSecond) -> f64 {
    proper_length * (1.0 - (velocity.0 * velocity.0) / (C * C)).sqrt()
}

/// Computes mass-energy equivalence: E = mc²
pub fn mass_energy(mass: Kilograms) -> Joules {
    Joules(mass.0 * C * C)
}

/// Computes Lorentz factor: γ = 1 / √(1 - v² / c²)
pub fn lorentz_factor(velocity: MetersPerSecond) -> f64 {
    1.0 / (1.0 - (velocity.0 * velocity.0) / (C * C)).sqrt()
}