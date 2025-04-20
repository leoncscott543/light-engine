// causality/solvers/simple_gravity.rs
// ===================================================
// Simple gravity solver — pairwise Newtonian gravity
// Uses stateless kernels from ordinem
// Designed for SoA ParticleCore tick updates
// ===================================================

use crate::matter::particle::core::ParticleCore;
use crate::ordinem::laws::classical::gravity::gravity_accel_vector;
use crate::ordinem::statics::typesafes::Kilograms;

/// Apply pairwise Newtonian gravity to all active particles.
/// Updates particle acceleration vectors based on current positions and masses.
/// Uses per-particle radius to handle realistic gravity inside large masses.
pub fn apply_gravity(particles: &mut ParticleCore) {
    let n = particles.capacity();

    for i in 0..n {
        if !particles.active[i] {
            continue;
        }

        let mut ax = 0.0;
        let mut ay = 0.0;
        let mut az = 0.0;

        for j in 0..n {
            if i == j || !particles.active[j] {
                continue;
            }

            let dx = particles.pos_x[j] - particles.pos_x[i];
            let dy = particles.pos_y[j] - particles.pos_y[i];
            let dz = particles.pos_z[j] - particles.pos_z[i];

            let source_mass = Kilograms(particles.mass[j]);
            let source_radius = particles.radius[j]; // 👈 NEW: read source radius

            let (gx, gy, gz) = gravity_accel_vector(
                source_mass,
                dx,
                dy,
                dz,
                source_radius, // 👈 pass radius into kernel
            );

            ax += gx;
            ay += gy;
            az += gz;
        }

        particles.acc_x[i] = ax;
        particles.acc_y[i] = ay;
        particles.acc_z[i] = az;
    }
}
