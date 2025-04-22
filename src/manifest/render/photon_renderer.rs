// manifest/render/photon_renderer.rs
use crate::manifest::render::lamp::{Lamp, Photon};
use crate::manifest::render::observer::Observer;
use crate::matter::material::material;

pub fn simulate_photon_interactions(lamp: &Lamp, observer: &Observer) {
    let photons = lamp.emit_photons();
    let test_material = material::Material::iron();

    for photon in photons.iter() {
        let _absorbed = test_material.absorb(photon);
    }

    let incoming: Vec<[f32; 3]> = photons.iter().map(|p| p.origin).collect();
    observer.collect(&incoming);
}

