use std::fs;
use std::path::Path;

const SIZES: [u32; 4] = [14, 18, 24, 32];
const CHAR_START: u8 = 32;
const CHAR_END: u8 = 126;

struct FontVariant {
    name: &'static str,
    path: &'static str,
    weight: &'static str,
    style: &'static str,
}

const INTER_VARIANTS: &[FontVariant] = &[
    FontVariant { name: "INTER_REGULAR", path: "src/assets/fonts/Inter/Inter-Regular.ttf", weight: "Regular", style: "Normal" },
    FontVariant { name: "INTER_BOLD", path: "src/assets/fonts/Inter/Inter-Bold.ttf", weight: "Bold", style: "Normal" },
    FontVariant { name: "INTER_ITALIC", path: "src/assets/fonts/Inter/Inter-Italic.ttf", weight: "Regular", style: "Italic" },
    FontVariant { name: "INTER_BOLD_ITALIC", path: "src/assets/fonts/Inter/Inter-BoldItalic.ttf", weight: "Bold", style: "Italic" },
];

const GARAMOND_VARIANTS: &[FontVariant] = &[
    FontVariant { name: "GARAMOND_REGULAR", path: "src/assets/fonts/EBGaramond/EBGaramond-Regular.ttf", weight: "Regular", style: "Normal" },
    FontVariant { name: "GARAMOND_BOLD", path: "src/assets/fonts/EBGaramond/EBGaramond-Bold.ttf", weight: "Bold", style: "Normal" },
    FontVariant { name: "GARAMOND_ITALIC", path: "src/assets/fonts/EBGaramond/EBGaramond-Italic.ttf", weight: "Regular", style: "Italic" },
    FontVariant { name: "GARAMOND_BOLD_ITALIC", path: "src/assets/fonts/EBGaramond/EBGaramond-BoldItalic.ttf", weight: "Bold", style: "Italic" },
];

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let mut code = String::new();
    code.push_str("// Auto-generated bitmap fonts\n\n");

    code.push_str("#[derive(Debug, Clone, Copy)]\n");
    code.push_str("pub struct BitmapGlyph {\n");
    code.push_str("    pub width: u8,\n");
    code.push_str("    pub height: u8,\n");
    code.push_str("    pub bearing_x: i8,\n");
    code.push_str("    pub bearing_y: i8,\n");
    code.push_str("    pub advance: u8,\n");
    code.push_str("    pub data_offset: usize,\n");
    code.push_str("    pub data_len: usize,\n");
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone, Copy)]\n");
    code.push_str("pub struct BitmapFont {\n");
    code.push_str("    pub size: u32,\n");
    code.push_str("    pub line_height: u8,\n");
    code.push_str("    pub ascent: i8,\n");
    code.push_str("    pub glyphs: &'static [BitmapGlyph],\n");
    code.push_str("    pub data: &'static [u8],\n");
    code.push_str("    pub char_start: u8,\n");
    code.push_str("    pub char_end: u8,\n");
    code.push_str("}\n\n");

    code.push_str("impl BitmapFont {\n");
    code.push_str("    pub fn glyph(&self, c: char) -> Option<&BitmapGlyph> {\n");
    code.push_str("        let code = c as u8;\n");
    code.push_str("        if code < self.char_start || code > self.char_end {\n");
    code.push_str("            return None;\n");
    code.push_str("        }\n");
    code.push_str("        Some(&self.glyphs[(code - self.char_start) as usize])\n");
    code.push_str("    }\n\n");
    code.push_str("    pub fn glyph_data(&self, glyph: &BitmapGlyph) -> &[u8] {\n");
    code.push_str("        &self.data[glyph.data_offset..glyph.data_offset + glyph.data_len]\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    let all_variants: Vec<&FontVariant> = INTER_VARIANTS.iter().chain(GARAMOND_VARIANTS.iter()).collect();

    for variant in &all_variants {
        let font_path = Path::new(&manifest_dir).join(variant.path);
        let font_data = fs::read(&font_path).expect(&format!("Failed to read font: {}", variant.path));
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

            let const_name = format!("{}_{}", variant.name, size);

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
                "pub const {}: BitmapFont = BitmapFont {{\n    size: {},\n    line_height: {},\n    ascent: {},\n    glyphs: {}_GLYPHS,\n    data: {}_DATA,\n    char_start: {},\n    char_end: {},\n}};\n\n",
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

    for variant in &all_variants {
        let family = if variant.name.starts_with("INTER") { "Inter" } else { "EBGaramond" };
        for &size in &SIZES {
            code.push_str(&format!(
                "        (FontFamily::{}, FontWeight::{}, FontStyle::{}, {}) => &{}_{},\n",
                family, variant.weight, variant.style, size, variant.name, size
            ));
        }
    }

    code.push_str("        _ => &INTER_REGULAR_18,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(Path::new(&out_dir).join("bitmap_fonts.rs"), code).unwrap();

    println!("cargo:rerun-if-changed=src/assets/fonts");
}
