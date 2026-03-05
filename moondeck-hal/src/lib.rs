pub mod display;
pub mod fs;
pub mod http;
pub mod touch;
pub mod wifi;

pub use display::{Display, Framebuffer};
pub use fs::FileSystem;
pub use http::HttpClient;
pub use touch::TouchController;
pub use wifi::WifiManager;

use std::collections::HashMap;

pub struct EnvConfig {
    vars: HashMap<String, String>,
}

impl EnvConfig {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn load_from_str(content: &str) -> Self {
        let mut vars = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
                vars.insert(key, value);
            }
        }
        Self { vars }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.vars.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::new()
    }
}
