use std::fmt;
use std::time::Duration;

/// Time integration scheme for the Hamiltonian evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Integrator {
    /// Non-symplectic baseline; expected to drift.
    ExplicitEuler,
    SymplecticEuler,
    Leapfrog,
    /// Fourth-order symplectic composition.
    Yoshida4,
}

impl Integrator {
    pub const ALL: [Self; 4] = [
        Self::ExplicitEuler,
        Self::SymplecticEuler,
        Self::Leapfrog,
        Self::Yoshida4,
    ];

    pub fn is_symplectic(self) -> bool {
        !matches!(self, Self::ExplicitEuler)
    }
}

impl fmt::Display for Integrator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ExplicitEuler => "explicit euler",
            Self::SymplecticEuler => "symplectic euler",
            Self::Leapfrog => "leapfrog",
            Self::Yoshida4 => "yoshida-4",
        })
    }
}

/// Physical storage layout of particle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    AoS,
    SoA,
    /// Array of SIMD-width tiles; `lanes` is the tile width.
    AoSoA {
        lanes: u16,
    },
}

impl Layout {
    pub const ALL: [Self; 5] = [
        Self::AoS,
        Self::SoA,
        Self::AoSoA { lanes: 4 },
        Self::AoSoA { lanes: 8 },
        Self::AoSoA { lanes: 16 },
    ];
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AoS => f.write_str("AoS"),
            Self::SoA => f.write_str("SoA"),
            Self::AoSoA { lanes } => write!(f, "AoSoA x{lanes}"),
        }
    }
}

/// Parameters needed to create a simulation session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimConfig {
    pub particle_count: u32,
    /// Timestep in simulation seconds.
    pub dt: f64,
    pub integrator: Integrator,
    pub layout: Layout,
    pub seed: u64,
    /// Target steps per wall-clock second while running; `0` means unthrottled.
    pub target_rate: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            particle_count: 4_096,
            dt: 1e-3,
            integrator: Integrator::Leapfrog,
            layout: Layout::AoSoA { lanes: 8 },
            seed: 0x5EED,
            target_rate: 120,
        }
    }
}

/// Lifecycle state of the simulation session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimStatus {
    /// No session exists.
    Inactive,
    Idle,
    Running,
    Paused,
    Faulted(String),
}

impl SimStatus {
    pub fn has_session(&self) -> bool {
        !matches!(self, Self::Inactive)
    }
}

impl fmt::Display for SimStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inactive => f.write_str("inactive"),
            Self::Idle => f.write_str("idle"),
            Self::Running => f.write_str("running"),
            Self::Paused => f.write_str("paused"),
            Self::Faulted(reason) => write!(f, "faulted: {reason}"),
        }
    }
}

/// Commands a front end may issue to the simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum SimCommand {
    Create(SimConfig),
    Destroy,
    Start,
    Pause,
    /// Advance a fixed number of steps while not running.
    Step(u32),
    /// Restore the initial state of the current session.
    Reset,
    SetTimestep(f64),
}

/// Conserved quantities and error measures tracked per step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Invariants {
    pub energy: f64,
    pub initial_energy: f64,
    pub momentum: [f64; 3],
    pub angular_momentum: [f64; 3],
    pub symplectic_error: f64,
}

impl Invariants {
    /// `|H(n) - H(0)| / |H(0)|`, or `0` when `H(0)` is zero.
    pub fn relative_energy_error(&self) -> f64 {
        if self.initial_energy == 0.0 {
            0.0
        } else {
            ((self.energy - self.initial_energy) / self.initial_energy).abs()
        }
    }

    pub fn momentum_magnitude(&self) -> f64 {
        self.momentum.iter().map(|c| c * c).sum::<f64>().sqrt()
    }
}

/// Performance of the most recent step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StepPerf {
    pub step_time: Duration,
    pub particles_per_sec: f64,
    /// Fraction of SIMD lanes doing useful work, `0.0..=1.0`.
    pub lane_utilization: f64,
    pub memory_bytes: u64,
}

/// Point-in-time view of the simulation session.
#[derive(Debug, Clone, PartialEq)]
pub struct SimSnapshot {
    pub config: SimConfig,
    pub status: SimStatus,
    pub tick: u64,
    pub sim_time: f64,
    pub invariants: Invariants,
    pub perf: StepPerf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_energy_error() {
        let inv = Invariants {
            energy: 9.0,
            initial_energy: 10.0,
            ..Default::default()
        };
        assert!((inv.relative_energy_error() - 0.1).abs() < 1e-12);
        assert_eq!(Invariants::default().relative_energy_error(), 0.0);
    }

    #[test]
    fn only_inactive_has_no_session() {
        assert!(!SimStatus::Inactive.has_session());
        assert!(SimStatus::Paused.has_session());
    }
}
