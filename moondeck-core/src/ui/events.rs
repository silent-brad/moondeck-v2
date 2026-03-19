use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TouchEvent {
    pub x: i32,
    pub y: i32,
    pub phase: TouchPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gesture {
    Tap { x: i32, y: i32 },
    LongPress { x: i32, y: i32 },
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Touch(TouchEvent),
    Gesture(Gesture),
    Tick { delta_ms: u32 },
    WifiConnected { ip: String },
    WifiDisconnected,
}

pub struct GestureDetector {
    start_x: Option<i32>,
    start_y: Option<i32>,
    start_time: Option<u64>,
    current_x: i32,
    current_y: i32,
    threshold_px: i32,
    long_press_ms: u64,
    last_swipe_time: u64,
    swipe_cooldown_ms: u64,
}

impl Default for GestureDetector {
    fn default() -> Self {
        Self {
            start_x: None,
            start_y: None,
            start_time: None,
            current_x: 0,
            current_y: 0,
            threshold_px: 8,
            long_press_ms: 800,
            last_swipe_time: 0,
            swipe_cooldown_ms: 300,
        }
    }
}

impl GestureDetector {
    pub fn new(threshold_px: i32, long_press_ms: u64) -> Self {
        Self {
            threshold_px,
            long_press_ms,
            ..Default::default()
        }
    }

    pub fn process(&mut self, touch: TouchEvent, current_time_ms: u64) -> Option<Gesture> {
        match touch.phase {
            TouchPhase::Started => {
                self.start_x = Some(touch.x);
                self.start_y = Some(touch.y);
                self.current_x = touch.x;
                self.current_y = touch.y;
                self.start_time = Some(current_time_ms);
                None
            }
            TouchPhase::Moved => {
                self.current_x = touch.x;
                self.current_y = touch.y;
                None
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let (sx, sy, st) = match (self.start_x, self.start_y, self.start_time) {
                    (Some(x), Some(y), Some(t)) => (x, y, t),
                    _ => return None,
                };

                self.start_x = None;
                self.start_y = None;
                self.start_time = None;

                // Use Ended event position directly (more reliable than tracked current position)
                let end_x = touch.x;
                let end_y = touch.y;

                let dx = end_x - sx;
                let dy = end_y - sy;
                let dt = current_time_ms.saturating_sub(st);

                log::info!(
                    "Gesture calc: start=({},{}) end=({},{}) dx={} dy={} dt={}ms threshold={}",
                    sx,
                    sy,
                    end_x,
                    end_y,
                    dx,
                    dy,
                    dt,
                    self.threshold_px
                );

                let abs_dx = dx.abs();
                let abs_dy = dy.abs();

                if abs_dx < self.threshold_px && abs_dy < self.threshold_px {
                    if dt >= self.long_press_ms {
                        return Some(Gesture::LongPress { x: sx, y: sy });
                    } else {
                        return Some(Gesture::Tap { x: sx, y: sy });
                    }
                }

                // Check cooldown to prevent multiple swipes
                if current_time_ms.saturating_sub(self.last_swipe_time) < self.swipe_cooldown_ms {
                    return Some(Gesture::Tap { x: sx, y: sy });
                }

                let gesture = if abs_dx > abs_dy {
                    if dx > self.threshold_px {
                        Some(Gesture::SwipeRight)
                    } else if dx < -self.threshold_px {
                        Some(Gesture::SwipeLeft)
                    } else {
                        None
                    }
                } else if dy > self.threshold_px {
                    Some(Gesture::SwipeDown)
                } else if dy < -self.threshold_px {
                    Some(Gesture::SwipeUp)
                } else {
                    None
                };

                if gesture.is_some() {
                    self.last_swipe_time = current_time_ms;
                }
                gesture
            }
        }
    }
}
