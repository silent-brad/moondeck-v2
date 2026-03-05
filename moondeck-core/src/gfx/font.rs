use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    #[default]
    Small,
    Medium,
    Large,
    XLarge,
}

impl Font {
    pub fn height(&self) -> u32 {
        match self {
            Font::Small => 8,
            Font::Medium => 12,
            Font::Large => 16,
            Font::XLarge => 24,
        }
    }

    pub fn char_width(&self) -> u32 {
        match self {
            Font::Small => 6,
            Font::Medium => 7,
            Font::Large => 9,
            Font::XLarge => 14,
        }
    }
}
