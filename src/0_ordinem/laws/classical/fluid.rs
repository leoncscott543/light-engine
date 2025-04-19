// 0_ordinem/laws/classical/fluids.rs
// ===================================================
// Fluid dynamics kernels (pressure, flow rate, Bernoulli)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Newtons, Pascals, Kilograms, MetersPerSecond, Area, Joules
};

/// Computes fluid pressure: P = F / A
pub fn pressure(force: Newtons, area: Area) -> Pascals {
    Pascals(force.0 / area.0)
}

/// Computes mass flow rate: ṁ = ρ * A * v
pub fn mass_flow_rate(density: f64, area: Area, velocity: MetersPerSecond) -> Kilograms {
    Kilograms(density * area.0 * velocity.0)
}

/// Computes Bernoulli energy per unit volume: P + 0.5ρv²
pub fn bernoulli_pressure(dynamic_pressure: Pascals, density: f64, velocity: MetersPerSecond) -> Pascals {
    let kinetic = 0.5 * density * velocity.0.powi(2);
    Pascals(dynamic_pressure.0 + kinetic)
}

/// Computes kinetic energy of a moving fluid element: KE = 0.5 * m * v²
pub fn fluid_kinetic_energy(mass: Kilograms, velocity: MetersPerSecond) -> Joules {
    Joules(0.5 * mass.0 * velocity.0.powi(2))
}