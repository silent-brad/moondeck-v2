use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontFamily {
    #[default]
    Inter,
    EBGaramond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    #[default]
    Regular,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtfFont {
    pub family: FontFamily,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub size: u32,
}

impl TtfFont {
    pub const fn new(family: FontFamily, weight: FontWeight, style: FontStyle, size: u32) -> Self {
        Self {
            family,
            weight,
            style,
            size,
        }
    }

    pub const fn inter(size: u32) -> Self {
        Self::new(FontFamily::Inter, FontWeight::Regular, FontStyle::Normal, size)
    }

    pub const fn inter_bold(size: u32) -> Self {
        Self::new(FontFamily::Inter, FontWeight::Bold, FontStyle::Normal, size)
    }

    pub const fn inter_italic(size: u32) -> Self {
        Self::new(FontFamily::Inter, FontWeight::Regular, FontStyle::Italic, size)
    }

    pub const fn inter_bold_italic(size: u32) -> Self {
        Self::new(FontFamily::Inter, FontWeight::Bold, FontStyle::Italic, size)
    }

    pub const fn garamond(size: u32) -> Self {
        Self::new(FontFamily::EBGaramond, FontWeight::Regular, FontStyle::Normal, size)
    }

    pub const fn garamond_bold(size: u32) -> Self {
        Self::new(FontFamily::EBGaramond, FontWeight::Bold, FontStyle::Normal, size)
    }

    pub const fn garamond_italic(size: u32) -> Self {
        Self::new(FontFamily::EBGaramond, FontWeight::Regular, FontStyle::Italic, size)
    }

    pub const fn garamond_bold_italic(size: u32) -> Self {
        Self::new(FontFamily::EBGaramond, FontWeight::Bold, FontStyle::Italic, size)
    }

    pub fn with_size(self, size: u32) -> Self {
        Self { size, ..self }
    }

    pub fn with_weight(self, weight: FontWeight) -> Self {
        Self { weight, ..self }
    }

    pub fn with_style(self, style: FontStyle) -> Self {
        Self { style, ..self }
    }
}

impl Default for TtfFont {
    fn default() -> Self {
        Self::inter(16)
    }
}

pub(crate) fn font_bytes(family: FontFamily, weight: FontWeight, style: FontStyle) -> &'static [u8] {
    match (family, weight, style) {
        (FontFamily::Inter, FontWeight::Regular, FontStyle::Normal) => {
            include_bytes!("../assets/fonts/Inter/Inter-Regular.ttf")
        }
        (FontFamily::Inter, FontWeight::Regular, FontStyle::Italic) => {
            include_bytes!("../assets/fonts/Inter/Inter-Italic.ttf")
        }
        (FontFamily::Inter, FontWeight::Bold, FontStyle::Normal) => {
            include_bytes!("../assets/fonts/Inter/Inter-Bold.ttf")
        }
        (FontFamily::Inter, FontWeight::Bold, FontStyle::Italic) => {
            include_bytes!("../assets/fonts/Inter/Inter-BoldItalic.ttf")
        }
        (FontFamily::EBGaramond, FontWeight::Regular, FontStyle::Normal) => {
            include_bytes!("../assets/fonts/EBGaramond/EBGaramond-Regular.ttf")
        }
        (FontFamily::EBGaramond, FontWeight::Regular, FontStyle::Italic) => {
            include_bytes!("../assets/fonts/EBGaramond/EBGaramond-Italic.ttf")
        }
        (FontFamily::EBGaramond, FontWeight::Bold, FontStyle::Normal) => {
            include_bytes!("../assets/fonts/EBGaramond/EBGaramond-Bold.ttf")
        }
        (FontFamily::EBGaramond, FontWeight::Bold, FontStyle::Italic) => {
            include_bytes!("../assets/fonts/EBGaramond/EBGaramond-BoldItalic.ttf")
        }
    }
}
