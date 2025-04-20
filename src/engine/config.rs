use std::time::Instant;

/// Configuration for how the engine runs in real time and simulation.
/// This is set once at startup and passed to systems like the ticker and engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Target tick rate (in Hz) — how often the engine updates per second
    pub tick_rate: f64,

    /// Whether to log timing and tick info to the terminal
    pub verbose: bool,

    /// Whether to store each tick's sampled timestamp
    pub record_tick_times: bool,

    /// When the engine was first started (wall clock time)
    pub start_time: Instant,
}

impl EngineConfig {
    /// Create a new EngineConfig with the given tick rate
    pub fn new(tick_rate: f64) -> Self {
        Self {
            tick_rate,
            verbose: true,
            record_tick_times: false,
            start_time: Instant::now(),
        }
    }
}
