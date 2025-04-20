// 1_matter/particle/particle_core.rs
// ===================================================
// ParticleCore: central SoA particle struct
// O(1) access, cache-coherent layout, per-tick solver ready
// Fully aligned with Light Engine's slab-ID and capsule indexing architecture
// ===================================================

/// A unique index for referencing particles safely
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleID(pub usize);

/// Enum for particle shape, used for collision, volume, and material modeling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleShape {
    // Generic physical bodies
    Sphere,
    Cube,
    Capsule,
    Cylinder,

    // Quantum/atomic fields
    QuantumOrbitalS,
    QuantumOrbitalP,
    QuantumOrbitalD,
    QuantumOrbitalF,

    // Approximate bonded/atomic groups
    AtomEllipsoid,
    AtomDumbbell,
    AtomLatticeNode,

    // Molecular geometries
    MoleculeTetrahedral,
    MoleculeLinear,
    MoleculePlanar,
    MoleculeCluster,
    MoleculeRing,

    // Simulation approximations
    VoxelCube,
    SoftBlob,
    RigidCapsule,

    // Extension point
    Custom(u8),
}

/// The core SoA (Structure of Arrays) layout for all particles in the system.
/// All fields are stored in tightly packed vectors for optimal memory access.
/// This design supports real-time solver access, and integrates with Light Engine's
/// planned chunking and capsule system without breaking existing usage.
pub struct ParticleCore {
    // Position components (SoA layout)
    pub pos_x: Vec<f64>,
    pub pos_y: Vec<f64>,
    pub pos_z: Vec<f64>,

    // Velocity components
    pub vel_x: Vec<f64>,
    pub vel_y: Vec<f64>,
    pub vel_z: Vec<f64>,

    // Acceleration components
    pub acc_x: Vec<f64>,
    pub acc_y: Vec<f64>,
    pub acc_z: Vec<f64>,

    // Intrinsic physical properties
    pub mass: Vec<f64>,     // mass in kilograms
    pub radius: Vec<f64>,   // effective collision radius (meters)
    pub charge: Vec<f64>,   // electric charge (Coulombs)
    pub inv_mass: Vec<f64>, // 1/mass, for fast physics
    pub shape: Vec<ParticleShape>, // collision/volume representation

    // Simulation system attributes
    pub active: Vec<bool>,       // active status (true = in use)
    pub kind: Vec<u8>,           // enum-like: 0 = Crystal, 1 = Atom, etc.
    pub fidelity: Vec<u8>,       // 0 = Low, 1 = Med, 2 = High, 3 = StdModel
    pub flags: Vec<u16>,         // bitflags: pinned, frozen, etc.

    // Capsule references (0 = none)
    pub optics_idx: Vec<u16>,    // index into optics capsule pool
    pub thermal_idx: Vec<u16>,   // index into thermal capsule pool
    pub structure_idx: Vec<u16>, // index into structural capsule pool

    // Index reuse support
    free_list: Vec<usize>,       // inactive particle indices for reuse
}

impl ParticleCore {
    /// Create an empty ParticleCore with initialized vectors
    pub fn new() -> Self {
        Self {
            pos_x: vec![], pos_y: vec![], pos_z: vec![],
            vel_x: vec![], vel_y: vec![], vel_z: vec![],
            acc_x: vec![], acc_y: vec![], acc_z: vec![],
            mass: vec![], radius: vec![], charge: vec![], inv_mass: vec![],
            shape: vec![],
            active: vec![], kind: vec![], fidelity: vec![], flags: vec![],
            optics_idx: vec![], thermal_idx: vec![], structure_idx: vec![],
            free_list: vec![],
        }
    }

    /// Add a new particle with physical parameters and return its ID.
    /// Automatically reuses slots from the free list if available.
    pub fn add_particle(
        &mut self,
        pos: [f64; 3],
        vel: [f64; 3],
        mass: f64,
        radius: f64,
        charge: f64,
        shape: ParticleShape,
        kind: u8,
        fidelity: u8,
        flags: u16
    ) -> ParticleID {
        let inv_mass = if mass == 0.0 { 0.0 } else { 1.0 / mass };
        let idx = if let Some(free) = self.free_list.pop() {
            self.pos_x[free] = pos[0]; self.pos_y[free] = pos[1]; self.pos_z[free] = pos[2];
            self.vel_x[free] = vel[0]; self.vel_y[free] = vel[1]; self.vel_z[free] = vel[2];
            self.acc_x[free] = 0.0; self.acc_y[free] = 0.0; self.acc_z[free] = 0.0;
            self.mass[free] = mass; self.radius[free] = radius; self.charge[free] = charge;
            self.inv_mass[free] = inv_mass;
            self.shape[free] = shape;
            self.kind[free] = kind; self.fidelity[free] = fidelity; self.flags[free] = flags;
            self.optics_idx[free] = 0; self.thermal_idx[free] = 0; self.structure_idx[free] = 0;
            self.active[free] = true;
            free
        } else {
            self.pos_x.push(pos[0]); self.pos_y.push(pos[1]); self.pos_z.push(pos[2]);
            self.vel_x.push(vel[0]); self.vel_y.push(vel[1]); self.vel_z.push(vel[2]);
            self.acc_x.push(0.0); self.acc_y.push(0.0); self.acc_z.push(0.0);
            self.mass.push(mass); self.radius.push(radius); self.charge.push(charge);
            self.inv_mass.push(inv_mass);
            self.shape.push(shape);
            self.kind.push(kind); self.fidelity.push(fidelity); self.flags.push(flags);
            self.optics_idx.push(0); self.thermal_idx.push(0); self.structure_idx.push(0);
            self.active.push(true);
            self.pos_x.len() - 1
        };
        ParticleID(idx)
    }

    /// Mark a particle as inactive and schedule its slot for reuse.
    pub fn remove_particle(&mut self, id: ParticleID) {
        let i = id.0;
        self.active[i] = false;
        self.free_list.push(i);
    }

    /// Return the number of currently active particles
    pub fn len(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// Return the capacity of the core (total number of slots)
    pub fn capacity(&self) -> usize {
        self.pos_x.len()
    }
}
