// manifest/render/backend.rs
#[cfg(target_os = "macos")]
pub fn init_backend() {
    crate::manifest::render::wgpu::init();
}

#[cfg(not(target_os = "macos"))]
pub fn init_backend() {
    crate::manifest::render::wgpu::init();
}

pub fn render() {
    crate::manifest::render::wgpu::render_frame();
}
