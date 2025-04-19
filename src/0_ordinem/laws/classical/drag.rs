// 0_ordinem/laws/classical/drag.rs
// ===================================================
// Drag force kernels (quadratic, linear, terminal velocity)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Kilograms, MetersPerSecond, Newtons, Area
};
use crate::ordinem::statics::constants::G;

/// Computes quadratic drag force: F_d = 0.5 * ρ * v² * C_d * A
pub fn drag_force_quadratic(air_density: f64, velocity: MetersPerSecond, drag_coefficient: f64, area: Area) -> Newtons {
    let force = 0.5 * air_density * velocity.0.powi(2) * drag_coefficient * area.0;
    Newtons(force)
}

/// Computes linear drag force (e.g., Stokes' law): F_d = k * v
pub fn drag_force_linear(damping_coefficient: f64, velocity: MetersPerSecond) -> Newtons {
    Newtons(damping_coefficient * velocity.0)
}

/// Computes terminal velocity: v_t = sqrt((2 * m * g) / (ρ * A * C_d))
pub fn terminal_velocity(mass: Kilograms, air_density: f64, drag_coefficient: f64, area: Area) -> MetersPerSecond {
    let vt = ((2.0 * mass.0 * G) / (air_density * area.0 * drag_coefficient)).sqrt();
    MetersPerSecond(vt)
}