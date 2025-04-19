// 0_ordinem/laws/standard_model/bosons.rs
// ===================================================
// Boson property kernels (mass, identity, classification)
// Pure stateless functions using strongly typed units
// ===================================================

use crate::ordinem::statics::typesafes::Kilograms;
use crate::ordinem::statics::constants::{
    MH, MW, MZ, MG, MGAMMA
};

/// Returns mass of Higgs boson
pub fn higgs_mass() -> Kilograms {
    Kilograms(MH)
}

/// Returns mass of W boson
pub fn w_boson_mass() -> Kilograms {
    Kilograms(MW)
}

/// Returns mass of Z boson
pub fn z_boson_mass() -> Kilograms {
    Kilograms(MZ)
}

/// Returns mass of gluon (expected to be zero)
pub fn gluon_mass() -> Kilograms {
    Kilograms(MG)
}

/// Returns mass of photon (expected to be zero)
pub fn photon_mass() -> Kilograms {
    Kilograms(MGAMMA)
}