// 0_ordinem/laws/classical/collision.rs
// ===================================================
// Collision kernels (Elastic, Inelastic, Restitution)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Kilograms, MetersPerSecond
};

/// Computes final velocity of object 1 after 1D elastic collision.
/// v1f = ((m1 - m2) * v1 + 2 * m2 * v2) / (m1 + m2)
pub fn elastic_collision_1d(m1: Kilograms, v1: MetersPerSecond, m2: Kilograms, v2: MetersPerSecond) -> MetersPerSecond {
    let numerator = (m1.0 - m2.0) * v1.0 + 2.0 * m2.0 * v2.0;
    let denominator = m1.0 + m2.0;
    MetersPerSecond(numerator / denominator)
}

/// Computes final velocity of combined mass after 1D inelastic collision.
/// vf = (m1*v1 + m2*v2) / (m1 + m2)
pub fn inelastic_collision_1d(m1: Kilograms, v1: MetersPerSecond, m2: Kilograms, v2: MetersPerSecond) -> MetersPerSecond {
    let vf = (m1.0 * v1.0 + m2.0 * v2.0) / (m1.0 + m2.0);
    MetersPerSecond(vf)
}

/// Computes relative velocity: v_rel = v1 - v2
pub fn relative_velocity(v1: MetersPerSecond, v2: MetersPerSecond) -> MetersPerSecond {
    MetersPerSecond(v1.0 - v2.0)
}

/// Computes coefficient of restitution: e = |v2f - v1f| / |v1 - v2|
pub fn coefficient_of_restitution(v1_initial: MetersPerSecond, v2_initial: MetersPerSecond, v1_final: MetersPerSecond, v2_final: MetersPerSecond) -> f64 {
    ((v2_final.0 - v1_final.0).abs()) / (v1_initial.0 - v2_initial.0).abs()
}