pub fn apply_spatial_effects(
    left: &mut [f32],
    right: &mut [f32],
    position: [f32; 3],
    listener: [f32; 3],
    gain: f32,
) {
    let dx = position[0] - listener[0];
    let dz = position[2] - listener[2];
    let dist = (dx*dx + dz*dz).sqrt().max(1.0); // Avoid div by zero

    let attenuation = gain / dist;
    let pan = (dx / dist).clamp(-1.0, 1.0); // -1 = left, 1 = right

    for i in 0..left.len() {
        left[i] *= attenuation * (1.0 - pan);
        right[i] *= attenuation * (1.0 + pan);
    }
}
