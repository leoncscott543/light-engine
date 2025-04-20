//========================================================
// UNIVERSIAL PHYSICAL CONSTANTS (RUST-NATIVE FORMAT)
//========================================================

/// =================================
/// UNIVERSAL PHYSICAL CONSTANTS
/// =================================

/// Fine-structure constant
pub const ALPHA: f64 = 0.007_297_352_569_3; // dimensionless

/// Atomic mass constant
pub const MU: f64 = 1.660_539_066_60e-27; // kg


/// =================================
/// CLASSICAL MECHANICS
/// =================================

/// Gravitational constant
pub const G: f64 = 6.67430e-11; // m^3 kg^-1 s^-2

/// Gravitational acceleration
pub const GEARTH: f64 = 9.806_65; // m/s^2

/// Universal gas constant
pub const R: f64 = 8.314_462_618; // J K^-1 mol^-1


/// =================================
/// ELECTROMAGNETISM
/// =================================

/// Coulomb's constant
pub const KE: f64 = 8.987_551_787_368_1764e9; // N m^2 C^-2

/// Elementary charge
pub const E: f64 = 1.602_176_634e-19; // C

/// Vacuum permittivity
pub const EPSILON_0: f64 = 8.854_187_817e-12; // C^2 N^-1 m^-2

/// vacuum permeability
pub const MU0: f64 = 1.256_637_062e-6; // N A^-2


/// =================================
/// THERMODYNAMICS
/// =================================

/// Boltzmann constant
pub const KB: f64 = 1.380_649e-23; // J K^-1

/// Stefan-Boltzmann constant
pub const SIGMA: f64 = 5.670_374e-8; // W m^-2 K^-4


/// =================================
/// QUANTUM MECHANICS
/// =================================

/// Planck's constant
pub const H: f64 = 6.626_070_15e-34; // J s

/// Reduced Planck's constant
pub const HBAR: f64 = 1.054_571_817e-34; // J s



/// =================================
/// RELATIVITY
/// =================================

/// Speed of light in vacuum
pub const C: f64 = 299_792_458.0; // m/s

/// Avogadro's number
pub const NA: f64 = 6.022_140_76e23; // mol^-1


/// =================================
/// PARTICLE PHYSICS
/// =================================

/// Mass of the electron
pub const ME: f64 = 9.109_383_56e-31; // kg

/// Mass of the proton
pub const MP: f64 = 1.672_621_9e-27; // kg

/// Mass of the neutron
pub const MN: f64 = 1.675_001_84e-27; // kg


/// =================================
/// COSMOLOGY/PLANK'S SCALE
/// =================================

/// Hubble's constant
pub const H_0: f64 = 70.0; // km/s/Mpc

/// Cosmological constant
pub const LAMBDA: f64 = 1.0e-52; // m^-2

/// Plank's length
pub const LP: f64 = 1.616_255e-35; // m

/// Plank's time
pub const TP: f64 = 5.391_247e-44; // s

/// Plank's energy
pub const EP: f64 = 1.220_910e28; // J

/// Plank's charge
pub const QP: f64 = 1.875_545_956e-18; // C

/// Plank's volume
pub const VP: f64 = 4.221_848_749e-71; // m^3

/// Plank's area
pub const AP: f64 = 1.112_536_929e-70; // m^2

/// Plank's force
pub const FP: f64 = 1.696_121e-8; // N

/// Plank's pressure
pub const PP: f64 = 4.633_123_935e-27; // N/m^2

/// Plank's energy density
pub const EDENSITY_P: f64 = 1.220_910e28; // J/m^3

/// Plank's entropy
pub const SP: f64 = 1.416_808e32; // J/K



/// =================================
/// STANDARD MODEL
/// =================================

/// Electron neutrino mass
pub const MNUE: f64 = 1.0e-37; // kg

/// Muon neutrino mass
pub const MNUMU: f64 = 1.0e-37; // kg

/// Tau neutrino mass
pub const MNUTAU: f64 = 1.0e-37; // kg

/// Muon mass
pub const MMU: f64 = 1.883_531_627e-28; // kg

/// Tau mass
pub const MTAU: f64 = 3.167_547_6e-27; // kg

/// W boson mass
pub const MW: f64 = 8.419_100_2e-25; // kg

/// Z boson mass
pub const MZ: f64 = 9.118_760_4e-25; // kg

/// Higgs boson mass
pub const MH: f64 = 2.220_000_0e-25; // kg

/// Top quark mass
pub const MT: f64 = 1.732_000_0e-25; // kg

/// Bottom quark mass
pub const MB: f64 = 4.180_000_0e-27; // kg

/// Down quark mass
pub const MD: f64 = 4.8e-27; // kg

/// Strange quark mass
pub const MS: f64 = 95.0e-27; // kg

/// Charm quark mass
pub const MC: f64 = 1.27e-25; // kg

/// Gluon mass
pub const MG: f64 = 0.0; // kg (massless)

/// Photon mass
pub const M_GAMMA: f64 = 0.0; // kg (massless)

/// =================================
/// Coupling Constants
/// =================================

/// Strong coupling constant
pub const ALPHA_S: f64 = 0.118; // dimensionless

/// Weak coupling constant
pub const ALPHA_W: f64 = 0.033; // dimensionless

/// Electromagnetic coupling constant
pub const ALPHA_EM: f64 = 1.0 / (4.0 * std::f64::consts::PI * EPSILON_0); // dimensionless
