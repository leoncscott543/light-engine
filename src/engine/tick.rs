use std::time::{Duration, Instant};
use std::thread::sleep;

use crate::engine::clock::{Clock, format_duration};
use crate::engine::config::EngineConfig;

/// A real-time fixed-rate ticker that drives the simulation forward.
pub struct Ticker {
    pub config: EngineConfig,
    pub clock: Clock,
    pub tick_interval: Duration,
    pub tick_count: u64,
    pub sim_time: f64,
    pub last_tick_time: Instant,
}

impl Ticker {
    /// Create a new Ticker with given config
    pub fn new(config: EngineConfig) -> Self {
        let tick_interval = Duration::from_secs_f64(1.0 / config.tick_rate);

        Self {
            clock: Clock::new(),
            tick_interval,
            tick_count: 0,
            sim_time: 0.0,
            last_tick_time: Instant::now(),
            config,
        }
    }

    /// Run the ticker forever, advancing time at a fixed tick rate
    pub fn run_forever(&mut self) {
        loop {
            if self.last_tick_time.elapsed() >= self.tick_interval {
                self.tick();
            }

            // Prevent busy-waiting
            sleep(Duration::from_micros(500));
        }
    }

    /// Advance simulation time by one tick
    fn tick(&mut self) {
        self.tick_count += 1;
        self.sim_time += self.tick_interval.as_secs_f64();
        self.last_tick_time = Instant::now();

        if self.config.verbose {
            println!(
                "[Ticker] Tick {:>5} | Sim Time: {:>10.6} s | Real Time: {}",
                self.tick_count,
                self.sim_time,
                self.clock.formatted_elapsed()
            );
        }

        if self.config.record_tick_times {
            // TODO: push to log or sample buffer if needed
        }
    }
}
