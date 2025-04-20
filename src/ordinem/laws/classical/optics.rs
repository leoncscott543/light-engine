// 0_ordinem/classical/optics.rs
// ===================================================
// Optics law kernels (Snell's Law, Critical Angle, Reflectance, Photon Energy)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Radians, Meters, Joules, RefractiveIndex
};
use crate::ordinem::statics::constants::{H, C};

/// Computes the angle of refraction using Snell's Law.
/// n1 * sin(θ1) = n2 * sin(θ2)
/// Returns θ2 (in radians)
pub fn snells_law(n1: RefractiveIndex, theta1: Radians, n2: RefractiveIndex) -> Radians {
    let sin_theta2 = (n1.0 / n2.0) * theta1.0.sin();
    Radians(sin_theta2.asin())
}

/// Computes the critical angle for total internal reflection.
/// θc = arcsin(n2 / n1)
pub fn critical_angle(n1: RefractiveIndex, n2: RefractiveIndex) -> Radians {
    Radians((n2.0 / n1.0).asin())
}

/// Computes reflectance at normal incidence.
/// R = ((n1 - n2) / (n1 + n2))²
pub fn reflectance_normal_incidence(n1: RefractiveIndex, n2: RefractiveIndex) -> f64 {
    let r = (n1.0 - n2.0) / (n1.0 + n2.0);
    r * r
}

/// Computes energy of a photon given its wavelength: E = h * c / λ
pub fn photon_energy_from_wavelength(wavelength: Meters) -> Joules {
    let e = H * C / wavelength.0;
    Joules(e)
}