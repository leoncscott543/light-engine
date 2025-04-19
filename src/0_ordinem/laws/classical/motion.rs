// 0_ordinem/laws/classical/motion.rs
// ===================================================
// Classical motion kernels (Newton's Second Law, Momentum, Impulse)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    Kilograms, MetersPerSecond, MetersPerSecondSquared, Seconds,
    Newtons, Momentum, Impulse
};

/// Computes force from mass and acceleration: F = m * a
pub fn force_from_mass_accel(mass: Kilograms, accel: MetersPerSecondSquared) -> Newtons {
    Newtons(mass.0 * accel.0)
}

/// Computes momentum: p = m * v
pub fn momentum(mass: Kilograms, velocity: MetersPerSecond) -> Momentum {
    Momentum(mass.0 * velocity.0)
}

/// Computes impulse from force and time: J = F * Δt
pub fn impulse_from_force_time(force: Newtons, time: Seconds) -> Impulse {
    Impulse(force.0 * time.0)
}

/// Computes acceleration from force and mass: a = F / m
pub fn acceleration_from_force_mass(force: Newtons, mass: Kilograms) -> MetersPerSecondSquared {
    MetersPerSecondSquared(force.0 / mass.0)
}