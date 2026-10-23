use std::time::{Duration, Instant};

pub struct FramerateLogger {
    last_report: Instant,
    frame_count: u32,
    interval: Duration,
    fps: f32,
    frame_time_ms: f32,
    enabled: bool,
}

impl FramerateLogger {
    pub fn new() -> Self {
        Self::with_interval(Duration::from_secs(1))
    }

    pub fn with_interval(interval: Duration) -> Self {
        Self {
            last_report: Instant::now(),
            frame_count: 0,
            interval,
            fps: 0.0,
            frame_time_ms: 0.0,
            enabled: true,
        }
    }

    /// Records a new frame. Once the interval has elapsed, computes and prints the average framerate.
    pub fn tick(&mut self) {
        self.tick_at(Instant::now());
    }

    /// Records a frame at a specified timestamp (useful for deterministic tests).
    pub fn tick_at(&mut self, now: Instant) {
        self.frame_count += 1;
        let elapsed = now.saturating_duration_since(self.last_report);
        if elapsed >= self.interval {
            let elapsed_secs = elapsed.as_secs_f32();
            if elapsed_secs > 0.0 {
                self.fps = self.frame_count as f32 / elapsed_secs;
                self.frame_time_ms = (elapsed_secs * 1000.0) / self.frame_count as f32;
            }
            if self.enabled {
                println!("FPS: {:.1} ({:.2} ms)", self.fps, self.frame_time_ms);
            }
            self.frame_count = 0;
            self.last_report = now;
        }
    }
}

impl Default for FramerateLogger {
    fn default() -> Self {
        Self::new()
    }
}
