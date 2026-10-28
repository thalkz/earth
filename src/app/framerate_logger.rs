use std::time::{Duration, Instant};

pub struct FramerateLogger {
    last_report: Instant,
    frame_count: u32,
}

impl FramerateLogger {
    pub fn new() -> Self {
        Self {
            last_report: Instant::now(),
            frame_count: 0,
        }
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last_report);
        if elapsed >= Duration::from_secs(1) {
            let elapsed_secs = elapsed.as_secs_f32();
            let fps = self.frame_count as f32 / elapsed_secs;
            let frame_time_ms = (elapsed_secs * 1000.0) / self.frame_count as f32;
            log::info!("FPS: {fps:.1} ({frame_time_ms:.2} ms)");
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
