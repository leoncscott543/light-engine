// 0_ordinem/classical/waves.rs
// ===================================================
// Wave law kernels (speed, frequency, energy)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::typesafes::{
    Meters, Hertz, Seconds, Joules, MetersPerSecond, PlanckConstant
};
use crate::constants::H;

/// Computes wave speed: v = f * λ
pub fn wave_speed(frequency: Hertz, wavelength: Meters) -> MetersPerSecond {
    MetersPerSecond(frequency.0 * wavelength.0)
}

/// Computes frequency from wave speed and wavelength: f = v / λ
pub fn frequency_from_speed_and_wavelength(speed: MetersPerSecond, wavelength: Meters) -> Hertz {
    Hertz(speed.0 / wavelength.0)
}

/// Computes wavelength from wave speed and frequency: λ = v / f
pub fn wavelength_from_speed_and_frequency(speed: MetersPerSecond, frequency: Hertz) -> Meters {
    Meters(speed.0 / frequency.0)
}

/// Computes period from frequency: T = 1 / f
pub fn period_from_frequency(frequency: Hertz) -> Seconds {
    Seconds(1.0 / frequency.0)
}

/// Computes quantum energy from frequency: E = h * f
pub fn energy_from_frequency(frequency: Hertz) -> Joules {
    Joules(H * frequency.0)
}