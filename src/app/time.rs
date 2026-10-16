use std::time::{Duration, Instant};

#[allow(dead_code)]
pub struct Clock {
    last_frame: Instant,
    start_time: Instant,
    frame_count: u64,
    delta: Duration,
}

#[allow(dead_code)]
impl Clock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_frame: now,
            start_time: now,
            frame_count: 0,
            delta: Duration::ZERO,
        }
    }

    /// Advances the clock and returns the elapsed time (`dt`) since the last tick.
    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        self.delta = now - self.last_frame;
        self.last_frame = now;
        self.frame_count += 1;
        self.delta
    }

    /// The delta time of the most recent frame.
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// The delta time of the most recent frame in seconds.
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// Total elapsed time since the clock was created.
    pub fn elapsed(&self) -> Duration {
        self.last_frame.duration_since(self.start_time)
    }

    /// Total elapsed time in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed().as_secs_f32()
    }

    /// Total number of rendered frames.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
