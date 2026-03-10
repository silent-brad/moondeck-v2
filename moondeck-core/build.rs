use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const SIZES: [u32; 4] = [14, 18, 24, 32];
const CHAR_START: u8 = 32;
const CHAR_END: u8 = 126;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FontVariant {
    family: String,
    weight: String,
    style: String,
    path: String,
}

fn discover_fonts(fonts_dir: &Path) -> Vec<FontVariant> {
    let mut variants = Vec::new();

    let Ok(families) = fs::read_dir(fonts_dir) else {
        return variants;
    };

    for family_entry in families.flatten() {
        let family_path = family_entry.path();
        if !family_path.is_dir() {
            continue;
        }

        let family_name = family_entry.file_name().to_string_lossy().to_string();

        let Ok(files) = fs::read_dir(&family_path) else {
            continue;
        };

        for file_entry in files.flatten() {
            let file_path = file_entry.path();
            let Some(ext) = file_path.extension() else {
                continue;
            };
            if ext != "ttf" && ext != "otf" {
                continue;
            }

            let file_name = file_entry.file_name().to_string_lossy().to_string();
            let stem = file_path.file_stem().unwrap().to_string_lossy();

            let (weight, style) = parse_font_style(&stem);

            let relative_path = format!(
                "src/assets/fonts/{}/{}",
                family_name, file_name
            );

            variants.push(FontVariant {
                family: family_name.clone(),
                weight,
                style,
                path: relative_path,
            });
        }
    }

    variants.sort();
    variants
}

fn parse_font_style(stem: &str) -> (String, String) {
    let parts: Vec<&str> = stem.split('-').collect();
    let style_part = parts.get(1).unwrap_or(&"Regular");

    let style_lower = style_part.to_lowercase();

    let (weight, style) = if style_lower.contains("bolditalic") {
        ("Bold", "Italic")
    } else if style_lower.contains("bold") {
        ("Bold", "Normal")
    } else if style_lower.contains("italic") {
        ("Regular", "Italic")
    } else {
        ("Regular", "Normal")
    };

    (weight.to_string(), style.to_string())
}

fn variant_const_name(variant: &FontVariant) -> String {
    let family_upper = variant.family.to_uppercase();
    let suffix = match (variant.weight.as_str(), variant.style.as_str()) {
        ("Regular", "Normal") => "REGULAR".to_string(),
        ("Bold", "Normal") => "BOLD".to_string(),
        ("Regular", "Italic") => "ITALIC".to_string(),
        ("Bold", "Italic") => "BOLD_ITALIC".to_string(),
        (w, s) => format!("{}_{}", w.to_uppercase(), s.to_uppercase()),
    };
    format!("{}_{}", family_upper, suffix)
}

fn constructor_name(variant: &FontVariant) -> String {
    let family_lower = variant.family.to_lowercase();
    match (variant.weight.as_str(), variant.style.as_str()) {
        ("Regular", "Normal") => family_lower,
        ("Bold", "Normal") => format!("{}_bold", family_lower),
        ("Regular", "Italic") => format!("{}_italic", family_lower),
        ("Bold", "Italic") => format!("{}_bold_italic", family_lower),
        (w, s) => format!("{}_{}_{}", family_lower, w.to_lowercase(), s.to_lowercase()),
    }
}

fn generate_bitmap_fonts(variants: &[FontVariant], manifest_dir: &str) -> String {
    let mut code = String::new();

    code.push_str(
        r#"// Auto-generated bitmap fonts

#[derive(Debug, Clone, Copy)]
pub struct BitmapGlyph {
    pub width: u8,
    pub height: u8,
    pub bearing_x: i8,
    pub bearing_y: i8,
    pub advance: u8,
    pub data_offset: usize,
    pub data_len: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct BitmapFont {
    pub size: u32,
    pub line_height: u8,
    pub ascent: i8,
    pub glyphs: &'static [BitmapGlyph],
    pub data: &'static [u8],
    pub char_start: u8,
    pub char_end: u8,
}

impl BitmapFont {
    pub fn glyph(&self, c: char) -> Option<&BitmapGlyph> {
        let code = c as u8;
        if code < self.char_start || code > self.char_end {
            return None;
        }
        Some(&self.glyphs[(code - self.char_start) as usize])
    }

    pub fn glyph_data(&self, glyph: &BitmapGlyph) -> &[u8] {
        &self.data[glyph.data_offset..glyph.data_offset + glyph.data_len]
    }
}

"#,
    );

    for variant in variants {
        let font_path = Path::new(manifest_dir).join(&variant.path);
        let font_data = fs::read(&font_path).unwrap_or_else(|_| panic!("Failed to read font: {}", variant.path));
        let font = rusttype::Font::try_from_vec(font_data).expect("Failed to parse font");

        for &size in &SIZES {
            let scale = rusttype::Scale::uniform(size as f32);
            let v_metrics = font.v_metrics(scale);
            let line_height = (v_metrics.ascent - v_metrics.descent + v_metrics.line_gap).ceil() as u8;
            let ascent = v_metrics.ascent.ceil() as i8;

            let mut glyphs_data: Vec<u8> = Vec::new();
            let mut glyph_infos: Vec<(u8, u8, i8, i8, u8, usize, usize)> = Vec::new();

            for c in CHAR_START..=CHAR_END {
                let chr = c as char;
                let glyph = font.glyph(chr).scaled(scale);
                let h_metrics = glyph.h_metrics();
                let advance = h_metrics.advance_width.ceil() as u8;

                let positioned = glyph.positioned(rusttype::point(0.0, 0.0));

                if let Some(bb) = positioned.pixel_bounding_box() {
                    let width = (bb.max.x - bb.min.x) as u8;
                    let height = (bb.max.y - bb.min.y) as u8;
                    let bearing_x = bb.min.x as i8;
                    let bearing_y = bb.min.y as i8;

                    let data_offset = glyphs_data.len();
                    let mut glyph_pixels = vec![0u8; width as usize * height as usize];

                    positioned.draw(|x, y, v| {
                        let idx = y as usize * width as usize + x as usize;
                        if idx < glyph_pixels.len() {
                            glyph_pixels[idx] = (v * 255.0) as u8;
                        }
                    });

                    glyphs_data.extend_from_slice(&glyph_pixels);
                    let data_len = glyph_pixels.len();

                    glyph_infos.push((width, height, bearing_x, bearing_y, advance, data_offset, data_len));
                } else {
                    glyph_infos.push((0, 0, 0, 0, advance, 0, 0));
                }
            }

            let const_name = format!("{}_{}", variant_const_name(variant), size);

            code.push_str(&format!("const {}_GLYPHS: &[BitmapGlyph] = &[\n", const_name));
            for (width, height, bearing_x, bearing_y, advance, data_offset, data_len) in &glyph_infos {
                code.push_str(&format!(
                    "    BitmapGlyph {{ width: {}, height: {}, bearing_x: {}, bearing_y: {}, advance: {}, data_offset: {}, data_len: {} }},\n",
                    width, height, bearing_x, bearing_y, advance, data_offset, data_len
                ));
            }
            code.push_str("];\n\n");

            code.push_str(&format!("const {}_DATA: &[u8] = &[\n", const_name));
            for chunk in glyphs_data.chunks(20) {
                code.push_str("    ");
                for b in chunk {
                    code.push_str(&format!("{}, ", b));
                }
                code.push('\n');
            }
            code.push_str("];\n\n");

            code.push_str(&format!(
                "pub const {}: BitmapFont = BitmapFont {{ size: {}, line_height: {}, ascent: {}, glyphs: {}_GLYPHS, data: {}_DATA, char_start: {}, char_end: {} }};\n\n",
                const_name, size, line_height, ascent, const_name, const_name, CHAR_START, CHAR_END
            ));
        }
    }

    code.push_str("pub fn get_bitmap_font(family: FontFamily, weight: FontWeight, style: FontStyle, size: u32) -> &'static BitmapFont {\n");
    code.push_str("    let nearest_size = match size {\n");
    code.push_str("        0..=15 => 14,\n");
    code.push_str("        16..=20 => 18,\n");
    code.push_str("        21..=27 => 24,\n");
    code.push_str("        _ => 32,\n");
    code.push_str("    };\n");
    code.push_str("    match (family, weight, style, nearest_size) {\n");

    for variant in variants {
        for &size in &SIZES {
            code.push_str(&format!(
                "        (FontFamily::{}, FontWeight::{}, FontStyle::{}, {}) => &{}_{},\n",
                variant.family, variant.weight, variant.style, size, variant_const_name(variant), size
            ));
        }
    }

    let default_font = variants
        .iter()
        .find(|v| v.weight == "Regular" && v.style == "Normal")
        .map(|v| format!("{}_{}", variant_const_name(v), 18))
        .unwrap_or_else(|| format!("{}_{}", variant_const_name(&variants[0]), 18));

    code.push_str(&format!("        _ => &{},\n", default_font));
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

fn generate_ttf_font_types(variants: &[FontVariant]) -> String {
    let families: BTreeSet<&str> = variants.iter().map(|v| v.family.as_str()).collect();
    let weights: BTreeSet<&str> = variants.iter().map(|v| v.weight.as_str()).collect();
    let styles: BTreeSet<&str> = variants.iter().map(|v| v.style.as_str()).collect();

    let mut code = String::new();

    code.push_str(
        r#"// Auto-generated font types
use serde::{Deserialize, Serialize};

"#,
    );

    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]\n");
    code.push_str("#[serde(rename_all = \"lowercase\")]\n");
    code.push_str("pub enum FontFamily {\n");
    for (i, family) in families.iter().enumerate() {
        if i == 0 {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", family));
    }
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]\n");
    code.push_str("#[serde(rename_all = \"lowercase\")]\n");
    code.push_str("pub enum FontWeight {\n");
    for weight in &weights {
        if *weight == "Regular" {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", weight));
    }
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]\n");
    code.push_str("#[serde(rename_all = \"lowercase\")]\n");
    code.push_str("pub enum FontStyle {\n");
    for style in &styles {
        if *style == "Normal" {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", style));
    }
    code.push_str("}\n\n");

    code.push_str(
        r#"#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtfFont {
    pub family: FontFamily,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub size: u32,
}

impl TtfFont {
    pub const fn new(family: FontFamily, weight: FontWeight, style: FontStyle, size: u32) -> Self {
        Self { family, weight, style, size }
    }

"#,
    );

    let variants_by_family: BTreeMap<&str, Vec<&FontVariant>> = {
        let mut map: BTreeMap<&str, Vec<&FontVariant>> = BTreeMap::new();
        for v in variants {
            map.entry(&v.family).or_default().push(v);
        }
        map
    };

    for (family, family_variants) in &variants_by_family {
        for variant in family_variants {
            let fn_name = constructor_name(variant);
            code.push_str(&format!(
                "    pub const fn {}(size: u32) -> Self {{\n        Self::new(FontFamily::{}, FontWeight::{}, FontStyle::{}, size)\n    }}\n\n",
                fn_name, family, variant.weight, variant.style
            ));
        }
    }

    code.push_str(
        r#"    pub fn with_size(self, size: u32) -> Self {
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
"#,
    );

    let default_constructor = variants
        .iter()
        .find(|v| v.weight == "Regular" && v.style == "Normal")
        .map(|v| constructor_name(v))
        .unwrap_or_else(|| constructor_name(&variants[0]));

    code.push_str(&format!("        Self::{}(16)\n", default_constructor));
    code.push_str("    }\n}\n");

    code
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let fonts_dir = Path::new(&manifest_dir).join("src/assets/fonts");
    let variants = discover_fonts(&fonts_dir);

    if variants.is_empty() {
        panic!("No fonts found in {:?}", fonts_dir);
    }

    let bitmap_code = generate_bitmap_fonts(&variants, &manifest_dir);
    fs::write(Path::new(&out_dir).join("bitmap_fonts.rs"), bitmap_code).unwrap();

    let ttf_code = generate_ttf_font_types(&variants);
    fs::write(Path::new(&out_dir).join("ttf_font.rs"), ttf_code).unwrap();

    println!("cargo:rerun-if-changed=src/assets/fonts");
}
