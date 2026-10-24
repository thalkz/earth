use std::time::{Duration, Instant};

pub struct Clock {
    last_frame: Instant,
    delta: Duration,
}

impl Clock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_frame: now,
            delta: Duration::ZERO,
        }
    }

    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        self.delta = now - self.last_frame;
        self.last_frame = now;
        self.delta
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
