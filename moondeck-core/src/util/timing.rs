pub struct FrameTimer {
    last_frame_ms: u64,
    frame_count: u64,
    fps: f32,
    fps_update_interval_ms: u64,
    fps_last_update_ms: u64,
    fps_frame_count: u64,
}

impl FrameTimer {
    pub fn new() -> Self {
        Self {
            last_frame_ms: 0,
            frame_count: 0,
            fps: 0.0,
            fps_update_interval_ms: 1000,
            fps_last_update_ms: 0,
            fps_frame_count: 0,
        }
    }

    pub fn tick(&mut self, current_ms: u64) -> u32 {
        let delta = if self.last_frame_ms == 0 {
            16
        } else {
            (current_ms.saturating_sub(self.last_frame_ms)) as u32
        };

        self.last_frame_ms = current_ms;
        self.frame_count += 1;
        self.fps_frame_count += 1;

        if current_ms.saturating_sub(self.fps_last_update_ms) >= self.fps_update_interval_ms {
            self.fps = (self.fps_frame_count as f32 * 1000.0)
                / (current_ms.saturating_sub(self.fps_last_update_ms)) as f32;
            self.fps_last_update_ms = current_ms;
            self.fps_frame_count = 0;
        }

        delta
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UpdateTimer {
    interval_ms: u64,
    last_update_ms: u64,
}

impl UpdateTimer {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_update_ms: 0,
        }
    }

    pub fn should_update(&mut self, current_ms: u64) -> bool {
        if current_ms.saturating_sub(self.last_update_ms) >= self.interval_ms {
            self.last_update_ms = current_ms;
            true
        } else {
            false
        }
    }

    pub fn set_interval(&mut self, interval_ms: u64) {
        self.interval_ms = interval_ms;
    }

    pub fn reset(&mut self) {
        self.last_update_ms = 0;
    }
}
