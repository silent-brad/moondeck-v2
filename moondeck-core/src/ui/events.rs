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
    threshold_px: i32,
    long_press_ms: u64,
}

impl Default for GestureDetector {
    fn default() -> Self {
        Self {
            start_x: None,
            start_y: None,
            start_time: None,
            threshold_px: 30,
            long_press_ms: 500,
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
                self.start_time = Some(current_time_ms);
                None
            }
            TouchPhase::Moved => None,
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let (sx, sy, st) = match (self.start_x, self.start_y, self.start_time) {
                    (Some(x), Some(y), Some(t)) => (x, y, t),
                    _ => return None,
                };

                self.start_x = None;
                self.start_y = None;
                self.start_time = None;

                let dx = touch.x - sx;
                let dy = touch.y - sy;
                let dt = current_time_ms.saturating_sub(st);

                let abs_dx = dx.abs();
                let abs_dy = dy.abs();

                if abs_dx < self.threshold_px && abs_dy < self.threshold_px {
                    if dt >= self.long_press_ms {
                        return Some(Gesture::LongPress { x: sx, y: sy });
                    } else {
                        return Some(Gesture::Tap { x: sx, y: sy });
                    }
                }

                if abs_dx > abs_dy {
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
                }
            }
        }
    }
}
