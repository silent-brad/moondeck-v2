use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetContext {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub opts: HashMap<String, serde_json::Value>,
}

impl WidgetContext {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            opts: HashMap::new(),
        }
    }

    pub fn with_opt<V: Into<serde_json::Value>>(mut self, key: &str, value: V) -> Self {
        self.opts.insert(key.to_string(), value.into());
        self
    }

    pub fn get_opt<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.opts
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.opts
            .get(key)
            .and_then(|v| v.as_str().map(String::from))
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.opts.get(key).and_then(|v| v.as_f64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.opts.get(key).and_then(|v| v.as_bool())
    }
}

pub trait Widget {
    fn init(&mut self, ctx: &WidgetContext);
    fn update(&mut self, ctx: &WidgetContext, delta_ms: u32);
    fn needs_redraw(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetInstance {
    pub module: String,
    pub context: WidgetContext,
    pub last_update_ms: u64,
    pub update_interval_ms: u64,
}

impl WidgetInstance {
    pub fn new(module: &str, x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            module: module.to_string(),
            context: WidgetContext::new(x, y, width, height),
            last_update_ms: 0,
            update_interval_ms: 1000,
        }
    }

    pub fn with_update_interval(mut self, ms: u64) -> Self {
        self.update_interval_ms = ms;
        self
    }

    pub fn with_opt<V: Into<serde_json::Value>>(mut self, key: &str, value: V) -> Self {
        self.context.opts.insert(key.to_string(), value.into());
        self
    }

    pub fn should_update(&self, current_time_ms: u64) -> bool {
        current_time_ms.saturating_sub(self.last_update_ms) >= self.update_interval_ms
    }
}
