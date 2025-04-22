// simple_box.wgsl
@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4(position, 1.0);
}

@fragment
fn fs_main(@location(1) color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4(color, 1.0);
}
