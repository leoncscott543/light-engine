// manifest/acoustics/mod.rs

pub mod generator;
pub mod output;
pub mod spacial; 


pub fn init_audio_backend() {
    // Setup CPAL output stream (or no-op for now)
}

pub fn process_audio_tick(/* pass ParticleCore or derived audio events */) {
    // Detect impulses, generate waveforms, push to stream
}

