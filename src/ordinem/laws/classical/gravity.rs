/// Directional gravitational acceleration vector exerted by m2 on m1
/// - Outside mass: Newtonian gravity (1/r²)
/// - Inside mass: Linear gravity (a ∝ r)

use crate::ordinem::statics::typesafes::Kilograms;
use crate::ordinem::statics::constants::G;

pub fn gravity_accel_vector(
    m2: Kilograms,
    dx: f64,
    dy: f64,
    dz: f64,
    radius: f64, // 👈 required for inside/outside logic
) -> (f64, f64, f64) {
    let r_squared = dx * dx + dy * dy + dz * dz;
    let r = r_squared.sqrt();

    // Prevent divide-by-zero
    if r < 1e-4 {
        return (0.0, 0.0, 0.0);
    }

    // Choose gravity model based on distance
    let accel_mag = if r >= radius {
        G * m2.0 / r_squared
    } else {
        G * m2.0 * r / (radius * radius * radius)
    };

    // Normalize direction vector and scale
    let ax = accel_mag * dx / r;
    let ay = accel_mag * dy / r;
    let az = accel_mag * dz / r;

    (ax, ay, az)
}
