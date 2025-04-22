// manifest/render/observer.rs
pub struct Observer {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub field_of_view: f32,
}

impl Observer {
    pub fn collect(&self, _incoming_photons: &[[f32; 3]]) {}
}