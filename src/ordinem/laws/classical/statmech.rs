// 0_ordinem/laws/classical/statmech.rs
// ===================================================
// Statistical mechanics kernels (Boltzmann, entropy, equipartition)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Joules, Kelvin, Energy, Temperature, Entropy
};
use crate::ordinem::statics::constants::KB;

/// Computes average thermal energy per degree of freedom: E = (1/2) * k_B * T
pub fn average_energy_per_dof(temperature: Kelvin) -> Joules {
    Joules(0.5 * KB * temperature.0)
}

/// Computes entropy from microstates: S = k_B * ln(Ω)
pub fn entropy_from_microstates(omega: f64) -> Entropy {
    Entropy(KB * omega.ln())
}

/// Computes Boltzmann factor: exp(-E / (k_B * T))
pub fn boltzmann_factor(energy: Joules, temperature: Kelvin) -> f64 {
    (-energy.0 / (KB * temperature.0)).exp()
}