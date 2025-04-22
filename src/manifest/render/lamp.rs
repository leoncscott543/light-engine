// manifest/render/lamp.rs
pub struct Lamp {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub energy_per_photon: f32,
    pub emission_rate: u32,
    pub color_temp: f32,
    pub spread_angle: f32,
}

pub struct Photon {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub wavelength: f32,
    pub energy: f32,
}

impl Lamp {
    pub fn emit_photons(&self) -> Vec<Photon> {
        let mut photons = Vec::with_capacity(self.emission_rate as usize);
        for _ in 0..self.emission_rate {
            photons.push(Photon {
                origin: self.position,
                direction: self.direction,
                wavelength: 1.0 / self.color_temp,
                energy: self.energy_per_photon,
            });
        }
        photons
    }
}