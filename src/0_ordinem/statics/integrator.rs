// 0_ordinem/statics/integrator.rs
// ===================================================
// Integrators for time evolution (Euler, Verlet, RK4)
// These are pure deterministic math modules used by causal solvers
// ===================================================

/// A universal interface for integrating position and velocity over time
pub trait Integrator {
    fn integrate(&self, x: f64, v: f64, a: f64, dt: f64) -> (f64, f64);
}

/// Basic Euler integrator: x' = x + v*dt, v' = v + a*dt
pub struct Euler;

impl Integrator for Euler {
    fn integrate(&self, x: f64, v: f64, a: f64, dt: f64) -> (f64, f64) {
        let v_new = v + a * dt;
        let x_new = x + v * dt;
        (x_new, v_new)
    }
}

/// Semi-implicit (Symplectic) Euler integrator: v' = v + a*dt, x' = x + v'*dt
pub struct SemiImplicitEuler;

impl Integrator for SemiImplicitEuler {
    fn integrate(&self, x: f64, v: f64, a: f64, dt: f64) -> (f64, f64) {
        let v_new = v + a * dt;
        let x_new = x + v_new * dt;
        (x_new, v_new)
    }
}

/// Basic Velocity Verlet integrator: x' = x + v*dt + 0.5*a*dt^2, v' = v + a*dt
pub struct Verlet;

impl Integrator for Verlet {
    fn integrate(&self, x: f64, v: f64, a: f64, dt: f64) -> (f64, f64) {
        let x_new = x + v * dt + 0.5 * a * dt * dt;
        let v_new = v + a * dt;
        (x_new, v_new)
    }
}