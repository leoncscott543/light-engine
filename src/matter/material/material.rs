// matter/material/material.rs
use crate::manifest::render::lamp::Photon;

pub struct Material {
    pub name: &'static str,
    pub absorption_coefficient: f32,
    pub reflectivity: f32,
    pub scattering_coefficient: f32,
    pub emission: f32,
    pub refractive_index: f32,
}

// Example usage of Material to ensure it is constructed
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_construction() {
        let iron = Material::iron();
        assert_eq!(iron.name, "Iron");
    }
}

impl Material {
    pub fn iron() -> Self {
        Self {
            name: "Iron",
            absorption_coefficient: 0.8,
            reflectivity: 0.6,
            scattering_coefficient: 0.1,
            emission: 0.0,
            refractive_index: 2.94, // Approximate for Iron
        }
    }

    pub fn absorb(&self, photon: &Photon) -> f32 {
        photon.energy * self.absorption_coefficient
    }
}
