// tests/first_test.rs
// ===================================================
// Verbose smoke tests for ParticleCore, Registry, and Integrator
// Output visible using: cargo test -- --nocapture
// ===================================================

use light_engine::ordinem::statics::{
    registry::{Fidelity, get_pipeline_for_fidelity, Solver},
    integrator::{Integrator, Euler},
};
use light_engine::matter::particle::particle_core::{ParticleCore, ParticleID};

#[test]
fn test_particle_add_remove() {
    let mut core = ParticleCore::new();
    println!("Creating new ParticleCore...");

    let id = core.add_particle([1.0, 2.0, 3.0], [0.0, 0.0, 0.0], 5.0);
    println!("Added particle with ID: {:?}", id);
    println!("Position: [{}, {}, {}]", core.pos_x[id.0], core.pos_y[id.0], core.pos_z[id.0]);
    println!("Mass: {}", core.mass[id.0]);

    assert_eq!(core.len(), 1);
    assert!(core.active[id.0]);

    core.remove_particle(id);
    println!("Removed particle with ID: {:?}", id);

    assert_eq!(core.len(), 0);
    assert!(!core.active[id.0]);

    let new_id = core.add_particle([9.0, 8.0, 7.0], [1.0, 1.0, 1.0], 3.0);
    println!("Reused ID: {:?}, new mass: {}", new_id, core.mass[new_id.0]);
    assert_eq!(id, new_id);
}

#[test]
fn test_solver_pipeline_lookup() {
    println!("Testing solver pipeline lookup for Fidelity::Low...");
    let pipeline = get_pipeline_for_fidelity(Fidelity::Low);
    println!("Pipeline: {:?}", pipeline);

    assert_eq!(pipeline.len(), 1);
    assert!(pipeline[0].contains(&Solver::Gravity));
}

#[test]
fn test_euler_integrator_step() {
    let integrator = Euler;
    println!("Testing Euler integrator...");

    let (x_new, v_new) = integrator.integrate(0.0, 1.0, 2.0, 0.5);
    println!("x_new = {}, v_new = {}", x_new, v_new);

    assert_eq!(v_new, 2.0); // v = 1 + 2*0.5 = 2
    assert_eq!(x_new, 0.5); // x = 0 + 1*0.5 = 0.5
}