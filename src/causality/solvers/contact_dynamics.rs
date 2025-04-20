// causality/solvers/contact_dynamics.rs
// ===================================================
// Contact solver using stateless real physics laws from ordinem
// Handles 1D elastic particle-world collisions (e.g. bounce off Earth)
// ===================================================

use crate::matter::particle::core::ParticleCore;
use crate::ordinem::laws::classical::collision::elastic_collision_1d;
use crate::ordinem::statics::typesafes::{Kilograms, MetersPerSecond};

/// Handles radial elastic contact with a fixed spherical planet (assumed to be particle 0).
/// Reverses velocity of the dynamic particle using real 1D collision physics.
pub fn apply_contact_dynamics(particles: &mut ParticleCore) {
    if particles.capacity() < 2 {
        return;
    }

    let earth_idx = 0; // assume particle 0 is Earth
    let earth_radius = particles.radius[earth_idx];
    let earth_pos = (
        particles.pos_x[earth_idx],
        particles.pos_y[earth_idx],
        particles.pos_z[earth_idx],
    );

    for i in 1..particles.capacity() {
        if !particles.active[i] {
            continue;
        }

        // Compute radial distance from Earth's center
        let dx = particles.pos_x[i] - earth_pos.0;
        let dy = particles.pos_y[i] - earth_pos.1;
        let dz = particles.pos_z[i] - earth_pos.2;
        let r = (dx * dx + dy * dy + dz * dz).sqrt();

        let object_radius = particles.radius[i];
        let contact_dist = earth_radius + object_radius;

        if r <= contact_dist {
            // Compute normal direction (unit vector)
            let nx = dx / r;
            let ny = dy / r;
            let nz = dz / r;

            // Get normal velocity component (project velocity onto normal)
            let vx = particles.vel_x[i];
            let vy = particles.vel_y[i];
            let vz = particles.vel_z[i];
            let v_dot_n = vx * nx + vy * ny + vz * nz;

            // Bounce only if moving toward Earth
            if v_dot_n < 0.0 {
                // 1D elastic bounce using ordinem law
                let m1 = Kilograms(particles.mass[i]);
                let m2 = Kilograms(particles.mass[earth_idx]); // Earth

                let v1 = MetersPerSecond(v_dot_n);
                let v2 = MetersPerSecond(0.0); // Earth is stationary

                let v1_new = elastic_collision_1d(m1, v1, m2, v2).0;

                let impulse = v1_new - v_dot_n;

                // Reflect velocity along normal direction
                particles.vel_x[i] += impulse * nx;
                particles.vel_y[i] += impulse * ny;
                particles.vel_z[i] += impulse * nz;

                // Positional correction: prevent sinking into Earth
                let correction = contact_dist - r;
                particles.pos_x[i] += nx * correction;
                particles.pos_y[i] += ny * correction;
                particles.pos_z[i] += nz * correction;
            }
        }
    }
}
