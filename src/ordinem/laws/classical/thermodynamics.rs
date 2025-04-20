
// 0_ordinem/classical/thermodynamics.rs
// ===================================================
// Thermodynamic law kernels (Heat, Entropy, Conduction, Expansion)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Joules, Kilograms, Kelvin, Temperature, SpecificHeatCapacity, Entropy,
    Watts, Meters, Area, ThermalConductivity
};

/// Computes heat transfer: Q = m * c * ΔT
pub fn heat_transfer(mass: Kilograms, specific_heat: SpecificHeatCapacity, delta_t: Kelvin) -> Joules {
    let q = mass.0 * specific_heat.0 * delta_t.0;
    Joules(q)
}

/// Computes entropy change: ΔS = Q / T
pub fn entropy_change(heat: Joules, temp: Kelvin) -> Entropy {
    let s = heat.0 / temp.0;
    Entropy(s)
}

/// Computes power loss due to thermal conduction: P = k * A * ΔT / d
pub fn thermal_conduction_rate(k: ThermalConductivity, area: Area, delta_t: Kelvin, thickness: Meters) -> Watts {
    let power = k.0 * area.0 * delta_t.0 / thickness.0;
    Watts(power)
}

/// Computes linear thermal expansion: ΔL = α * L₀ * ΔT
pub fn thermal_expansion_linear(coefficient: f64, initial_length: Meters, delta_t: Kelvin) -> Meters {
    let delta_l = coefficient * initial_length.0 * delta_t.0;
    Meters(delta_l)
}