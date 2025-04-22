// tests/photon_render_test.rs

use light_engine::manifest::render::lamp::{Lamp, Photon};
use light_engine::manifest::render::observer::Observer;
use light_engine::matter::material::material;

#[test]
fn test_single_particle_photon_absorption() {
    // Mock: single particle with absorption coefficient
    let absorption_coefficient = 0.8;


    // Setup a lamp that emits 10k photons
    let lamp = Lamp {
        position: [0.0, 0.0, 0.0],
        direction: [0.0, 0.0, -1.0],
        energy_per_photon: 1.0,
        emission_rate: 10_000,
        color_temp: 6500.0,
        spread_angle: 0.0,
    };

    let observer = Observer {
        position: [0.0, 0.0, 10.0],
        direction: [0.0, 0.0, -1.0],
        field_of_view: 1.0,
    };

    let photons = lamp.emit_photons();
    assert_eq!(photons.len(), 10_000);

    let mut total_absorbed = 0.0;
    for photon in photons {
        assert!((total_absorbed - 8000.0_f32).abs() < 10.0_f32); // Allow small numerical variation
    }

    println!("Total energy absorbed by 1 particle: {}", total_absorbed);

    // Optional: you could assert on total energy
    assert!((total_absorbed - 8000.0).abs() < 1.0); // Allow 1.0 unit wiggle
}
