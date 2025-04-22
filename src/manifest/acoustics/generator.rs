pub fn impulse_to_waveform(energy: f64, frequency: f64, duration: f64, sample_rate: u32) -> Vec<f32> {
    let amplitude = (energy.sqrt() / 100.0).min(1.0);
    let samples = (duration * sample_rate as f64) as usize;
    let two_pi_f = 2.0 * std::f64::consts::PI * frequency;

    (0..samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            (amplitude * (two_pi_f * t).sin()) as f32
        })
        .collect()
}
