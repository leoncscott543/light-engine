// 0_ordinem/classical/gravity.rs
// ===================================================
// Gravitational law kernels (Newtonian only)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::typesafes::{Kilograms, Meters, Newtons, MetersPerSecondSquared};
use crate::constants::G;

/// Computes the gravitational force between two masses separated by a distance.
/// F = G * (m1 * m2) / r^2
pub fn gravitational_force(m1: Kilograms, m2: Kilograms, r: Meters) -> Newtons {
    let force = G * (m1.0 * m2.0) / (r.0 * r.0);
    Newtons(force)
}

/// Computes the gravitational acceleration exerted by mass m2 on mass m1.
/// a = G * m2 / r^2
pub fn gravitational_acceleration(m2: Kilograms, r: Meters) -> MetersPerSecondSquared {
    let accel = G * m2.0 / (r.0 * r.0);
    MetersPerSecondSquared(accel)
}