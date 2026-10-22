//! Synthetic physics and performance model for a mock session.

use std::time::Duration;

use light_contracts::{
    Integrator, Invariants, Layout, SimConfig, SimSnapshot, SimStatus, StepPerf,
};

use crate::particles::ParticleField;
use crate::rng::Rng;

/// Coefficients per particle: position + momentum, grades 0..=2 in 3D (1+3+3).
const COEFFS_PER_PARTICLE: u64 = 2 * 7;
const BYTES_PER_COEFF: u64 = 8;

pub struct Session {
    pub config: SimConfig,
    pub status: SimStatus,
    pub tick: u64,
    initial_energy: f64,
    momentum: [f64; 3],
    angular_momentum: [f64; 3],
    perf: StepPerf,
    rng: Rng,
}

impl Session {
    pub fn new(config: SimConfig) -> Self {
        let mut rng = Rng::new(config.seed);
        let initial_energy = -0.25 * config.particle_count as f64 * rng.range(0.9, 1.1);
        let mut session = Self {
            config,
            status: SimStatus::Idle,
            tick: 0,
            initial_energy,
            momentum: [0.0; 3],
            angular_momentum: [rng.normal(), rng.normal(), rng.normal()],
            perf: StepPerf::default(),
            rng,
        };
        session.perf = session.sample_perf();
        session
    }

    pub fn reset(&mut self) {
        let status = self.status.clone();
        *self = Self::new(self.config);
        self.status = match status {
            SimStatus::Running => SimStatus::Running,
            _ => SimStatus::Idle,
        };
    }

    pub fn advance(&mut self, steps: u64) {
        if steps == 0 {
            return;
        }
        self.tick += steps;
        let walk = (steps as f64).sqrt() * 1e-14;
        for c in &mut self.momentum {
            *c += self.rng.normal() * walk;
        }
        for c in &mut self.angular_momentum {
            *c += self.rng.normal() * walk;
        }
        self.perf = self.sample_perf();
    }

    pub fn snapshot(&self) -> SimSnapshot {
        SimSnapshot {
            config: self.config,
            status: self.status.clone(),
            tick: self.tick,
            sim_time: self.tick as f64 * self.config.dt,
            invariants: self.invariants(),
            perf: self.perf,
        }
    }

    /// The full particle state at the current tick.
    pub fn particles(&self) -> ParticleField {
        ParticleField {
            seed: self.config.seed,
            count: u64::from(self.config.particle_count),
            layout: self.config.layout,
            tick: self.tick,
            sim_time: self.tick as f64 * self.config.dt,
            energy_error: self.energy_error(),
        }
    }

    pub fn invariants(&self) -> Invariants {
        Invariants {
            energy: self.initial_energy * (1.0 + self.energy_error()),
            initial_energy: self.initial_energy,
            momentum: self.momentum,
            angular_momentum: self.angular_momentum,
            symplectic_error: self.symplectic_error(),
        }
    }

    /// Signed relative energy error, shaped like each integrator's known behavior:
    /// bounded oscillation of order `dt^p` for symplectic schemes, secular drift otherwise.
    fn energy_error(&self) -> f64 {
        let dt = self.config.dt;
        let t = self.tick as f64 * dt;
        let wobble = (t * 2.3).sin() + 0.3 * (t * 7.1).sin();
        match self.config.integrator {
            Integrator::ExplicitEuler => 4.0 * dt * t + 0.5 * dt * wobble,
            Integrator::SymplecticEuler => 2.0 * dt * wobble,
            Integrator::Leapfrog => 40.0 * dt * dt * wobble,
            Integrator::Yoshida4 => 400.0 * dt.powi(4) * wobble + 1e-14 * wobble,
        }
    }

    fn symplectic_error(&self) -> f64 {
        let steps = self.tick as f64;
        if self.config.integrator.is_symplectic() {
            1e-15 * steps.sqrt()
        } else {
            1e-3 * self.config.dt * steps
        }
    }

    fn sample_perf(&mut self) -> StepPerf {
        let n = self.config.particle_count.max(1) as f64;
        let per_particle_ns = layout_cost_ns(self.config.layout)
            * integrator_cost(self.config.integrator)
            * self.rng.range(0.94, 1.08);
        let jitter = if self.rng.chance(0.03) { 1.6 } else { 1.0 };
        let step_secs = (n * per_particle_ns * jitter * 1e-9).max(1e-7);
        StepPerf {
            step_time: Duration::from_secs_f64(step_secs),
            particles_per_sec: n / step_secs,
            lane_utilization: self.lane_utilization(),
            memory_bytes: self.memory_bytes(),
        }
    }

    fn lane_utilization(&mut self) -> f64 {
        let n = self.config.particle_count as u64;
        match self.config.layout {
            Layout::AoS => self.rng.range(0.22, 0.30),
            Layout::SoA => self.rng.range(0.65, 0.78),
            Layout::AoSoA { lanes } => {
                let lanes = u64::from(lanes.max(1));
                let tiles = n.div_ceil(lanes).max(1);
                let fill = n as f64 / (tiles * lanes) as f64;
                fill * self.rng.range(0.93, 0.99)
            }
        }
    }

    fn memory_bytes(&self) -> u64 {
        let n = u64::from(self.config.particle_count);
        let padded = match self.config.layout {
            Layout::AoSoA { lanes } => n.next_multiple_of(u64::from(lanes.max(1))),
            _ => n,
        };
        padded * COEFFS_PER_PARTICLE * BYTES_PER_COEFF
    }
}

fn layout_cost_ns(layout: Layout) -> f64 {
    match layout {
        Layout::AoS => 11.5,
        Layout::SoA => 5.8,
        Layout::AoSoA { lanes: 0..=4 } => 4.1,
        Layout::AoSoA { lanes: 5..=8 } => 2.7,
        Layout::AoSoA { .. } => 2.2,
    }
}

fn integrator_cost(integrator: Integrator) -> f64 {
    match integrator {
        Integrator::ExplicitEuler | Integrator::SymplecticEuler => 1.0,
        Integrator::Leapfrog => 1.25,
        Integrator::Yoshida4 => 3.4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(integrator: Integrator) -> Session {
        Session::new(SimConfig {
            integrator,
            ..SimConfig::default()
        })
    }

    #[test]
    fn euler_drifts_more_than_leapfrog() {
        let mut euler = session(Integrator::ExplicitEuler);
        let mut leap = session(Integrator::Leapfrog);
        euler.advance(100_000);
        leap.advance(100_000);
        assert!(
            euler.invariants().relative_energy_error()
                > 10.0 * leap.invariants().relative_energy_error()
        );
    }

    #[test]
    fn reset_restores_tick_zero() {
        let mut s = session(Integrator::Leapfrog);
        s.advance(500);
        s.reset();
        assert_eq!(s.tick, 0);
        assert_eq!(s.invariants().relative_energy_error(), 0.0);
    }

    #[test]
    fn aosoa_memory_is_tile_padded() {
        let s = Session::new(SimConfig {
            particle_count: 10,
            layout: Layout::AoSoA { lanes: 8 },
            ..SimConfig::default()
        });
        assert_eq!(
            s.snapshot().perf.memory_bytes,
            16 * COEFFS_PER_PARTICLE * BYTES_PER_COEFF
        );
    }
}
