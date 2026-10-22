use std::time::{Duration, Instant};

#[allow(dead_code)]
pub struct FramerateLogger {
    last_report: Instant,
    frame_count: u32,
    interval: Duration,
    fps: f32,
    frame_time_ms: f32,
    enabled: bool,
}

#[allow(dead_code)]
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

    /// The most recently averaged frames per second.
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// The most recently averaged frame duration in milliseconds.
    pub fn frame_time_ms(&self) -> f32 {
        self.frame_time_ms
    }

    /// Sets the logging and calculation interval.
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Enables or disables printing to stdout.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for FramerateLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framerate_logger_calculation() {
        let mut logger = FramerateLogger::new();
        logger.set_enabled(false);
        let start = logger.last_report;

        // 60 frames spaced across 1 second
        let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
        for i in 1..=59 {
            logger.tick_at(start + frame_duration * i);
            assert_eq!(logger.fps(), 0.0);
        }

        // 60th frame reaches 1.0s
        logger.tick_at(start + Duration::from_secs(1));
        assert!(
            (logger.fps() - 60.0).abs() < 0.1,
            "Expected ~60 FPS, got {}",
            logger.fps()
        );
        assert!((logger.frame_time_ms() - (1000.0 / 60.0)).abs() < 0.1);
    }

    #[test]
    fn test_framerate_logger_interval_reset() {
        let mut logger = FramerateLogger::new();
        logger.set_enabled(false);
        let start = logger.last_report;

        for i in 1..=30 {
            logger.tick_at(start + Duration::from_millis(i * 33));
        }
        logger.tick_at(start + Duration::from_secs(1));
        assert!(
            (logger.fps() - 31.0).abs() < 1.0,
            "Expected ~31 FPS, got {}",
            logger.fps()
        );

        let next_start = start + Duration::from_secs(1);
        for i in 1..=120 {
            logger.tick_at(next_start + Duration::from_secs_f32(i as f32 / 120.0));
        }
        assert!(
            (logger.fps() - 120.0).abs() < 0.5,
            "Expected ~120 FPS, got {}",
            logger.fps()
        );
    }
}
