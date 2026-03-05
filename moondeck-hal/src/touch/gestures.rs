use moondeck_core::ui::{Gesture, GestureDetector, TouchEvent};

pub struct GestureProcessor {
    detector: GestureDetector,
}

impl GestureProcessor {
    pub fn new() -> Self {
        Self {
            detector: GestureDetector::default(),
        }
    }

    pub fn with_thresholds(swipe_threshold_px: i32, long_press_ms: u64) -> Self {
        Self {
            detector: GestureDetector::new(swipe_threshold_px, long_press_ms),
        }
    }

    pub fn process(&mut self, touch: TouchEvent, current_time_ms: u64) -> Option<Gesture> {
        self.detector.process(touch, current_time_ms)
    }
}

impl Default for GestureProcessor {
    fn default() -> Self {
        Self::new()
    }
}
