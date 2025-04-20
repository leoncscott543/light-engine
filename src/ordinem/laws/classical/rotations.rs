// 0_ordinem/laws/classical/rotation.rs
// ===================================================
// Rotational motion kernels (Torque, Angular Momentum, Inertia)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::{
    RadiansPerSecond, RadiansPerSecondSquared, Seconds,
    NewtonMeters, Kilograms, Meters, AngularMomentum,
    MomentOfInertia, Torque
};

/// Computes torque from moment of inertia and angular acceleration: τ = I * α
pub fn torque(inertia: MomentOfInertia, angular_accel: RadiansPerSecondSquared) -> NewtonMeters {
    NewtonMeters(inertia.0 * angular_accel.0)
}

/// Computes angular momentum: L = I * ω
pub fn angular_momentum(inertia: MomentOfInertia, angular_velocity: RadiansPerSecond) -> AngularMomentum {
    AngularMomentum(inertia.0 * angular_velocity.0)
}

/// Computes moment of inertia for a point mass: I = m * r^2
pub fn moment_of_inertia(mass: Kilograms, radius: Meters) -> MomentOfInertia {
    MomentOfInertia(mass.0 * radius.0 * radius.0)
}

/// Computes angular acceleration from torque and inertia: α = τ / I
pub fn angular_acceleration(torque: NewtonMeters, inertia: MomentOfInertia) -> RadiansPerSecondSquared {
    RadiansPerSecondSquared(torque.0 / inertia.0)
}