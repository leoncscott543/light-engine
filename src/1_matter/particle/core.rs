// 1_matter/particle/particle_core.rs
// ===================================================
// ParticleCore: central SoA particle struct
// O(1) access, cache-coherent layout, per-tick solver ready
// ===================================================

/// A unique index for referencing particles safely
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleID(pub usize);

/// Core SoA (Structure of Arrays) layout for all particles in the system.
/// This design ensures cache-friendly memory access and efficient per-tick updates.
pub struct ParticleCore {
    // Position components for particles
    pub pos_x: Vec<f64>,
    pub pos_y: Vec<f64>,
    pub pos_z: Vec<f64>,

    // Velocity components for particles
    pub vel_x: Vec<f64>,
    pub vel_y: Vec<f64>,
    pub vel_z: Vec<f64>,

    // Acceleration components for particles
    pub acc_x: Vec<f64>,
    pub acc_y: Vec<f64>,
    pub acc_z: Vec<f64>,

    // Mass of each particle
    pub mass: Vec<f64>,

    // Active state of each particle (true if active, false if inactive)
    pub active: Vec<bool>,

    // Free list to store reusable indices of inactive particles
    free_list: Vec<usize>,
}

impl ParticleCore {
    /// Create an empty particle core.
    /// Initializes all vectors as empty, ready to store particle data.
    pub fn new() -> Self {
        Self {
            pos_x: vec![],
            pos_y: vec![],
            pos_z: vec![],
            vel_x: vec![],
            vel_y: vec![],
            vel_z: vec![],
            acc_x: vec![],
            acc_y: vec![],
            acc_z: vec![],
            mass: vec![],
            active: vec![],
            free_list: vec![],
        }
    }

    /// Add a new particle to the system and return its unique ID.
    /// If there are reusable indices in the `free_list`, it reuses one.
    /// Otherwise, it appends new data to the vectors.
    pub fn add_particle(&mut self, pos: [f64; 3], vel: [f64; 3], mass: f64) -> ParticleID {
        let idx = if let Some(free) = self.free_list.pop() {
            // Reuse an index from the free list
            self.pos_x[free] = pos[0];
            self.pos_y[free] = pos[1];
            self.pos_z[free] = pos[2];
            self.vel_x[free] = vel[0];
            self.vel_y[free] = vel[1];
            self.vel_z[free] = vel[2];
            self.acc_x[free] = 0.0; // Reset acceleration
            self.acc_y[free] = 0.0;
            self.acc_z[free] = 0.0;
            self.mass[free] = mass;
            self.active[free] = true; // Mark as active
            free
        } else {
            // Add a new particle
            self.pos_x.push(pos[0]);
            self.pos_y.push(pos[1]);
            self.pos_z.push(pos[2]);
            self.vel_x.push(vel[0]);
            self.vel_y.push(vel[1]);
            self.vel_z.push(vel[2]);
            self.acc_x.push(0.0); // Initialize acceleration to zero
            self.acc_y.push(0.0);
            self.acc_z.push(0.0);
            self.mass.push(mass);
            self.active.push(true); // Mark as active
            self.pos_x.len() - 1 // Return the new index
        };
        ParticleID(idx)
    }

    /// Mark a particle as inactive and add its index to the free list.
    /// This allows the index to be reused for future particles.
    pub fn remove_particle(&mut self, id: ParticleID) {
        let i = id.0;
        self.active[i] = false; // Mark the particle as inactive
        self.free_list.push(i); // Add the index to the free list
    }

    /// Get the number of active particles in the system.
    /// Counts the number of `true` values in the `active` vector.
    pub fn len(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// Get the total allocated space for particles, including inactive ones.
    /// This corresponds to the length of the position vectors.
    pub fn capacity(&self) -> usize {
        self.pos_x.len()
    }
}