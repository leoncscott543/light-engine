use std::time::{Duration, Instant};

/// A simple real-time monotonic clock
/// Used to measure elapsed wall time since engine start
#[derive(Debug, Clone)]
pub struct Clock {
    start: Instant,
}

impl Clock {
    /// Create a new clock anchored to current real time
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Returns the elapsed real-world time since the clock started
    pub fn elapsed(&self) -> Duration {
        Instant::now() - self.start
    }

    /// Returns the elapsed time as seconds (f64)
    pub fn seconds(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Returns the elapsed time as integer seconds
    pub fn seconds_i64(&self) -> i64 {
        self.elapsed().as_secs() as i64
    }

    /// Formatted stopwatch-style string (HH:MM:SS:MS:US:NS)
    pub fn formatted_elapsed(&self) -> String {
        format_duration(self.elapsed())
    }
}

/// Format a `Duration` as a stopwatch-style string: HH:MM:SS:ms:us:ns
pub fn format_duration(dur: Duration) -> String {
    let total_nanos = dur.as_nanos();
    let total_micros = dur.as_micros();
    let total_millis = dur.as_millis();
    let secs = dur.as_secs();

    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let millis = (total_millis % 1000) as u64;
    let micros = (total_micros % 1000) as u64;
    let nanos = (total_nanos % 1000) as u64;

    format!("{:02}:{:02}:{:02}:{:03}:{:03}:{:03}", hours, minutes, seconds, millis, micros, nanos)
}
