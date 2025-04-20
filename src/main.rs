// src/main.rs
// ===================================================
// Light Engine main entry point
// Initializes a test with 20 randomized falling particles + 1 Earth particle
// Prints all particle fields per tick
// ===================================================

mod engine;
mod matter;
mod ordinem;
mod causality;

use std::thread::sleep;
use std::time::{Duration, Instant};

use engine::clock::Clock;
use matter::particle::core::{ParticleCore, ParticleShape};
use causality::solvers::simple_gravity::apply_gravity;
use causality::solvers::contact_dynamics::apply_contact_dynamics;

use rand::Rng;

/// Basic Euler integrator for ParticleCore
fn integrate(particles: &mut ParticleCore, dt: f64) {
    for i in 0..particles.capacity() {
        if !particles.active[i] {
            continue;
        }

        particles.vel_x[i] += particles.acc_x[i] * dt;
        particles.vel_y[i] += particles.acc_y[i] * dt;
        particles.vel_z[i] += particles.acc_z[i] * dt;

        particles.pos_x[i] += particles.vel_x[i] * dt;
        particles.pos_y[i] += particles.vel_y[i] * dt;
        particles.pos_z[i] += particles.vel_z[i] * dt;
    }
}

fn main() {
    let earth_mass = 5.972e24;          // kg
    let earth_radius = 6_371_000.0;     // meters
    let object_mass_range = 0.5..3.0;   // kg
    let object_radius_range = 0.05..0.2; // meters
    let drop_height_meters = 6.0;       // ~20 feet

    let initial_z = earth_radius + drop_height_meters;

    let tick_rate = 60.0;
    let dt = 1.0 / tick_rate;
    let tick_interval = Duration::from_secs_f64(dt);
    let mut last_tick = Instant::now();
    let clock = Clock::new();
    let mut tick_count = 0;

    let mut rng = rand::thread_rng();
    let mut particles = ParticleCore::new();

    // Earth particle (index 0)
    particles.add_particle(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        earth_mass,
        earth_radius,
        0.0,
        ParticleShape::Sphere,
        1,
        0,
        0,
    );

    // 20 randomized falling particles above Earth
    for _ in 0..5 {
        let x = rng.gen_range(-2.0..2.0);
        let y = rng.gen_range(-2.0..2.0);
        let z = initial_z + rng.gen_range(-0.5..0.5);
        let vx = rng.gen_range(-0.2..0.2);
        let vy = rng.gen_range(-0.2..0.2);
        let mass = rng.gen_range(object_mass_range.clone());
        let radius = rng.gen_range(object_radius_range.clone());
        let charge = rng.gen_range(-1.0..1.0);

        particles.add_particle(
            [x, y, z],
            [vx, vy, 0.0],
            mass,
            radius,
            charge,
            ParticleShape::Sphere,
            1,
            1,
            0,
        );
    }

    loop {
        if last_tick.elapsed() >= tick_interval {
            tick_count += 1;
            last_tick = Instant::now();

            apply_gravity(&mut particles);
            integrate(&mut particles, dt);
            apply_contact_dynamics(&mut particles);

            println!("[Tick {:>5}] Time: {}", tick_count, clock.formatted_elapsed());

            for i in 0..particles.capacity() {
                if particles.active[i] {
                    let dx = particles.pos_x[i];
                    let dy = particles.pos_y[i];
                    let dz = particles.pos_z[i];
                    let r_from_center = (dx * dx + dy * dy + dz * dz).sqrt();
                    let surface_dist = if i != 0 {
                        r_from_center - earth_radius
                    } else {
                        0.0
                    };

                    println!(
                        "Particle {} →\n  pos: [{:.4}, {:.4}, {:.4}]\n  vel: [{:.4}, {:.4}, {:.4}]\n  acc: [{:.4}, {:.4}, {:.4}]\n  mass: {:.3} kg\n  inv_mass: {:.3}\n  radius: {:.4} m\n  charge: {:.3} C\n  shape: {:?}\n  kind: {}\n  fidelity: {}\n  flags: {}\n  dist_from_surface: {:.4} m",
                        i,
                        particles.pos_x[i], particles.pos_y[i], particles.pos_z[i],
                        particles.vel_x[i], particles.vel_y[i], particles.vel_z[i],
                        particles.acc_x[i], particles.acc_y[i], particles.acc_z[i],
                        particles.mass[i],
                        particles.inv_mass[i],
                        particles.radius[i],
                        particles.charge[i],
                        particles.shape[i],
                        particles.kind[i],
                        particles.fidelity[i],
                        particles.flags[i],
                        surface_dist,
                    );
                }
            }
        }

        sleep(Duration::from_micros(500));
    }
}
