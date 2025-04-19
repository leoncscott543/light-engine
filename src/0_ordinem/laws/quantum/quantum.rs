// 0_ordinem/laws/quantum/quantum_mechanics.rs
// ===================================================
// Quantum mechanics kernels (uncertainty, energy, wavelength)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Joules, Meters, Seconds, Hertz, ElectronVolts
};
use crate::ordinem::statics::constants::{H, HBAR};

/// Computes energy from frequency: E = h * f
pub fn energy_from_frequency(frequency: Hertz) -> Joules {
    Joules(H * frequency.0)
}

/// Computes energy from time uncertainty: ΔE ≥ ħ / (2 * Δt)
pub fn uncertainty_energy(time_uncertainty: Seconds) -> Joules {
    Joules(HBAR / (2.0 * time_uncertainty.0))
}

/// Computes de Broglie wavelength: λ = h / p
pub fn de_broglie_wavelength(momentum_kg_m_per_s: f64) -> Meters {
    Meters(H / momentum_kg_m_per_s)
}

/// Converts energy in joules to electron volts (1 eV = 1.602e-19 J)
pub fn joules_to_ev(energy: Joules) -> ElectronVolts {
    ElectronVolts(energy.0 / 1.602_176_634e-19)
}

/// Converts electron volts to joules
pub fn ev_to_joules(ev: ElectronVolts) -> Joules {
    Joules(ev.0 * 1.602_176_634e-19)
}