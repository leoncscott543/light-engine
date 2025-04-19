// 0_ordinem/laws/classical/radiation.rs
// ===================================================
// Radiation kernels (blackbody, photon energy, radiant flux)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Joules, Kelvin, Watts, Area, Temperature
};
use crate::ordinem::statics::constants::{SIGMA, H, C};

/// Computes blackbody radiant flux using Stefan-Boltzmann law: P = σ * A * T⁴
pub fn blackbody_radiation(area: Area, temperature: Kelvin) -> Watts {
    Watts(SIGMA * area.0 * temperature.0.powi(4))
}

/// Computes energy of a photon given frequency: E = h * f
pub fn photon_energy_from_frequency(frequency: f64) -> Joules {
    Joules(H * frequency)
}

/// Computes energy of a photon from wavelength: E = h * c / λ
pub fn photon_energy_from_wavelength(wavelength_meters: f64) -> Joules {
    Joules(H * C / wavelength_meters)
}