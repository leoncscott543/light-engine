// matter/elements/iron.rs
// ===================================================
// Iron (Fe) elemental physical constants for simulation
// ===================================================

/// Atomic number
pub const ATOMIC_NUMBER: u8 = 26;

/// Atomic mass (approx) in atomic mass units (u)
pub const ATOMIC_MASS: f64 = 55.845; // g/mol

/// Density of solid iron at room temperature
pub const DENSITY: f64 = 7874.0; // kg/m^3

/// Young's modulus (stiffness)
pub const YOUNG_MODULUS: f64 = 200e9; // Pascals (N/m^2)

/// Poisson's ratio
pub const POISSON_RATIO: f64 = 0.29;

/// Coefficient of restitution (empirical range ~0.6 for steel/iron on steel)
pub const RESTITUTION: f64 = 0.6;

/// Melting point of iron (K)
pub const MELTING_POINT: f64 = 1811.0;

/// Boiling point (K)
pub const BOILING_POINT: f64 = 3134.0;

/// Specific heat capacity (J/kg·K)
pub const SPECIFIC_HEAT: f64 = 449.0;

/// Thermal conductivity (W/m·K)
pub const THERMAL_CONDUCTIVITY: f64 = 80.4;

/// Computes the mass of a solid iron sphere given its radius (m)
pub fn mass_from_radius(radius_m: f64) -> f64 {
    let volume = (4.0 / 3.0) * std::f64::consts::PI * radius_m.powi(3);
    DENSITY * volume
}
