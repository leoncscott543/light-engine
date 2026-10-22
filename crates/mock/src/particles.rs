//! Procedural particle state for a mock session: a rotating two-armed spiral
//! disk in a logarithmic potential. Every particle is a pure function of
//! `(seed, index, sim_time)`, so any particle can be read at any time without
//! storing the field.

use std::f64::consts::PI;

use light_contracts::{FIELD_COUNT, Layout, ParticleField as F, ParticleRow, ParticleSource};

use crate::rng::Rng;

/// Circular velocity at large radius.
const V0: f64 = 1.0;
/// Core radius of the potential.
const CORE: f64 = 0.3;
/// Mock disk time units per simulation second, so rotation is visible live.
const TIME_SCALE: f64 = 8.0;
const PITCH: f64 = 0.35;
const ARM_FRACTION: f64 = 0.7;
const DISPERSION: f64 = 0.08;
const THICKNESS: f64 = 0.05;
/// Angular speed of the activity density wave.
const PATTERN_SPEED: f64 = 1.7;

/// A borrowed, consistent view of every particle at one tick.
pub struct ParticleField {
    pub seed: u64,
    pub count: u64,
    pub layout: Layout,
    pub tick: u64,
    pub sim_time: f64,
    /// Relative energy error; scales velocities like a drifting integrator.
    pub energy_error: f64,
}

fn potential(r: f64) -> f64 {
    0.5 * V0 * V0 * (r * r + CORE * CORE).ln()
}

impl ParticleSource for ParticleField {
    fn particle_count(&self) -> u64 {
        self.count
    }

    fn tick(&self) -> u64 {
        self.tick
    }

    fn sim_time(&self) -> f64 {
        self.sim_time
    }

    fn read(&self, index: u64, row: &mut ParticleRow) {
        let mut rng = Rng::new(self.seed ^ (index + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let t = self.sim_time * TIME_SCALE;

        let r = -(1.0 - 0.98 * rng.unit()).ln();
        let on_arm = rng.chance(ARM_FRACTION);
        let theta0 = if on_arm {
            let arm = if rng.chance(0.5) { PI } else { 0.0 };
            arm + (r + 0.05).ln() / PITCH.tan() + 0.25 * rng.normal()
        } else {
            rng.range(0.0, 2.0 * PI)
        };
        let vc = V0 * r / (r * r + CORE * CORE).sqrt();
        let theta = theta0 + vc / r.max(1e-9) * t;
        let (sin, cos) = theta.sin_cos();
        let (x, y) = (r * cos, r * sin);
        let z = THICKNESS * rng.normal();

        let heat = (1.0 + self.energy_error).max(0.0).sqrt();
        let vx = (-vc * sin + DISPERSION * rng.normal()) * heat;
        let vy = (vc * cos + DISPERSION * rng.normal()) * heat;
        let vz = 0.5 * DISPERSION * rng.normal() * heat;
        let speed2 = vx * vx + vy * vy + vz * vz;

        let mass = (0.5 * rng.normal()).exp();
        let kinetic = 0.5 * mass * speed2;
        let pot = mass * potential((r * r + z * z).sqrt());

        let base = if on_arm {
            rng.range(0.55, 1.0)
        } else {
            rng.range(0.05, 0.45)
        };
        let jitter = 0.4 * rng.normal();
        let wave = 0.5 + 0.5 * (2.0 * theta - PATTERN_SPEED * t + jitter).sin();
        let activity = (base * wave + 0.25 * (-r / CORE).exp()).clamp(0.0, 1.0);

        let lanes = match self.layout {
            Layout::AoSoA { lanes } => u64::from(lanes.max(1)),
            Layout::AoS | Layout::SoA => 1,
        };

        let mut set = |field: F, value: f64| row[field.index()] = value;
        set(F::Id, index as f64);
        set(F::Tile, (index / lanes) as f64);
        set(F::Lane, (index % lanes) as f64);
        set(F::X, x);
        set(F::Y, y);
        set(F::Z, z);
        set(F::Vx, vx);
        set(F::Vy, vy);
        set(F::Vz, vz);
        set(F::Speed, speed2.sqrt());
        set(F::Radius, r);
        set(F::Mass, mass);
        set(F::Kinetic, kinetic);
        set(F::Potential, pot);
        set(F::Energy, kinetic + pot);
        set(F::Lz, mass * (x * vy - y * vx));
        set(F::Activity, activity);
        debug_assert_eq!(row.len(), FIELD_COUNT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(sim_time: f64) -> ParticleField {
        ParticleField {
            seed: 9,
            count: 2_000,
            layout: Layout::AoSoA { lanes: 8 },
            tick: 0,
            sim_time,
            energy_error: 0.0,
        }
    }

    fn row(f: &ParticleField, i: u64) -> ParticleRow {
        let mut row = [0.0; FIELD_COUNT];
        f.read(i, &mut row);
        row
    }

    #[test]
    fn deterministic_and_consistent() {
        let f = field(1.0);
        for i in 0..f.count {
            let a = row(&f, i);
            assert_eq!(a, row(&f, i));
            assert!(a.iter().all(|v| v.is_finite()), "particle {i}: {a:?}");
            assert!((0.0..=1.0).contains(&a[F::Activity.index()]));
            let r = a[F::X.index()].hypot(a[F::Y.index()]);
            assert!((r - a[F::Radius.index()]).abs() < 1e-9);
            assert_eq!(
                a[F::Energy.index()],
                a[F::Kinetic.index()] + a[F::Potential.index()]
            );
            assert_eq!(a[F::Tile.index()] * 8.0 + a[F::Lane.index()], i as f64);
        }
    }

    #[test]
    fn disk_rotates_prograde() {
        let (a, b) = (field(0.0), field(0.05));
        let i = 17;
        let (r0, r1) = (row(&a, i), row(&b, i));
        assert_eq!(r0[F::Radius.index()], r1[F::Radius.index()]);
        assert_ne!(r0[F::X.index()], r1[F::X.index()]);
        let lz: f64 = (0..a.count).map(|i| row(&a, i)[F::Lz.index()]).sum();
        assert!(lz > 0.0);
    }
}
