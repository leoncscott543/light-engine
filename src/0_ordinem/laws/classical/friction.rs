// 0_ordinem/laws/classical/friction.rs
// ===================================================
// Friction kernels (static, kinetic, normal force)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Kilograms, Newtons, MetersPerSecondSquared
};
use crate::ordinem::statics::constants::G;

/// Computes maximum static friction force: f_s ≤ μ_s * N
pub fn static_friction_max(coefficient_static: f64, normal_force: Newtons) -> Newtons {
    Newtons(coefficient_static * normal_force.0)
}

/// Computes kinetic friction force: f_k = μ_k * N
pub fn kinetic_friction(coefficient_kinetic: f64, normal_force: Newtons) -> Newtons {
    Newtons(coefficient_kinetic * normal_force.0)
}

/// Computes the normal force on a flat surface: N = m * g
pub fn normal_force_from_mass_gravity(mass: Kilograms) -> Newtons {
    Newtons(mass.0 * G)
}