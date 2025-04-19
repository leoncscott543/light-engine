// 0_ordinem/laws/standard_model/decay.rs
// ===================================================
// Particle decay kernels (exponential decay, half-life, decay constant)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::Seconds;

/// Computes remaining quantity N(t) using exponential decay: N(t) = N0 * exp(-λ * t)
pub fn exponential_decay(initial_quantity: f64, decay_constant: f64, time: Seconds) -> f64 {
    initial_quantity * (-decay_constant * time.0).exp()
}

/// Computes decay constant λ from half-life: λ = ln(2) / t₁/₂
pub fn decay_constant_from_half_life(half_life: Seconds) -> f64 {
    std::f64::consts::LN_2 / half_life.0
}

/// Computes half-life from decay constant: t₁/₂ = ln(2) / λ
pub fn half_life_from_decay_constant(decay_constant: f64) -> Seconds {
    Seconds(std::f64::consts::LN_2 / decay_constant)
}