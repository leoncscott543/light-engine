// manifest/render/material.rs
#[deprecated]
pub fn absorb(photon_energy: f32, absorption_coefficient: f32) -> f32 {
    photon_energy * absorption_coefficient
}
