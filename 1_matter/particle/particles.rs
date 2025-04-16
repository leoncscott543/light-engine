
// src: 1_matter/particles/particle.rs

/// Core trait that defines behavior for all particles in Light Engine.
/// Particles are the most fundamental simulated units — they must move, interact, and obey physics.
pub trait Particle {
    /// Unique identifier
    fn id(&self) -> &str;

    /// The name of the particle type (photon, electron, custom)
    fn kind(&self) -> &str;

    /// Position in simulation space (3D vector)
    fn position(&self) -> [f64; 3];

    /// Velocity in simulation space (meters/sec)
    fn velocity(&self) -> [f64; 3];

    /// Energy in joules (or equivalent)
    fn energy(&self) -> f64;

    /// Mass in kg (can be 0 for photons)
    fn mass(&self) -> f64;

    /// Timestep-based state advancement
    fn advance(&mut self, delta_time: f64);

    /// Optional metadata or tags (for interaction purposes)
    fn metadata(&self) -> Option<&str>;

    /// Optional debug representation
    fn describe(&self) {
        println!("{} at {:?} with energy {}", self.kind(), self.position(), self.energy());
    }
}
