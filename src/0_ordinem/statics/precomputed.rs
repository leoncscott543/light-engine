// 0_ordinem/statics/precomputed.rs
// ===================================================
// Precomputed values and numerical helpers (tables, constants, lookups)
// Used by solvers and kernel functions across physics domains
// ===================================================

use crate::ordinem::statics::typesafes::{Kelvin, Meters, Joules, ElectronVolts};

/// Frequently used mathematical constants
pub const LN_2: f64 = std::f64::consts::LN_2;
pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = 2.0 * PI;
pub const SQRT_2: f64 = std::f64::consts::SQRT_2;

/// Stefan-Boltzmann constant precomputed powers (used in T^4 scaling)
pub fn temperature_fourth_power(temp: Kelvin) -> f64 {
    temp.0.powi(4)
}

/// Approximate blackbody radiance curve (for demonstration purposes only)
pub fn approximate_blackbody_radiance(temp: Kelvin, wavelength: Meters) -> f64 {
    let peak = 2.898e-3 / temp.0; // Wien's law
    let offset = (wavelength.0 - peak).abs();
    (-offset * 1e6).exp() // Gaussian falloff (not physically accurate)
}

/// Hydrogen-like atom energy levels (Z=1, Bohr model)
pub fn hydrogen_energy_level(n: u32) -> Joules {
    let rydberg_joules = 2.179_872e-18;
    Joules(-rydberg_joules / ((n * n) as f64))
}

/// Hydrogen energy level in electron volts
pub fn hydrogen_energy_level_ev(n: u32) -> ElectronVolts {
    let rydberg_ev = 13.605693;
    ElectronVolts(-rydberg_ev / (n * n) as f64)
}

/// Simple energy-wavelength conversion: E = hc / λ (Joules)
pub fn energy_from_wavelength(wavelength: Meters) -> Joules {
    const H: f64 = 6.626_070_15e-34;
    const C: f64 = 2.997_924_58e8;
    Joules(H * C / wavelength.0)
}

/// Wavelength from energy: λ = hc / E (Meters)
pub fn wavelength_from_energy(energy: Joules) -> Meters {
    const H: f64 = 6.626_070_15e-34;
    const C: f64 = 2.997_924_58e8;
    Meters(H * C / energy.0)
}