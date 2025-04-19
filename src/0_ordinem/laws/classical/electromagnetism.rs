// 0_ordinem/classical/electromagnetism.rs
// ===================================================
// Electromagnetic law kernels (Coulomb, Electric Field, Lorentz)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::typesafes::{
    Coulombs, ElectricField, MagneticField, Meters, MetersPerSecond, Newtons, Volts
};
use crate::lib::constants::{C, KE};

/// Computes the electrostatic force between two point charges.
/// F = k_e * |q1 * q2| / r^2
pub fn coulomb_force(q1: Coulombs, q2: Coulombs, r: Meters) -> Newtons {
    let force = KE * (q1.0 * q2.0).abs() / (r.0 * r.0);
    Newtons(force)
}

/// Computes the electric field from a point charge at distance r.
/// E = k_e * |q| / r^2
pub fn electric_field(q: Coulombs, r: Meters) -> ElectricField {
    let field = KE * q.0.abs() / (r.0 * r.0);
    ElectricField(field)
}

/// Computes the Lorentz force on a charged particle.
/// F = q * (E + v × B) — simplified as scalar form for now
pub fn lorentz_force(q: Coulombs, v: MetersPerSecond, e: ElectricField, b: MagneticField) -> Newtons {
    // Assumes perpendicular v and B for scalar approximation of v × B
    let force = q.0 * (e.0 + v.0 * b.0);
    Newtons(force)
}