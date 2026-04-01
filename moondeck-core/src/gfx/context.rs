use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

use super::{bitmap_font::get_bitmap_font, ttf_font::TtfFont, Color, Font};

pub const DISPLAY_WIDTH: u32 = 800;
pub const DISPLAY_HEIGHT: u32 = 480;
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 2) as usize;

pub struct DrawContext<'a, T: DrawTarget<Color = Rgb565>> {
    target: &'a mut T,
    offset_x: i32,
    offset_y: i32,
}

impl<'a, T: DrawTarget<Color = Rgb565>> DrawContext<'a, T> {
    pub fn new(target: &'a mut T) -> Self {
        Self {
            target,
            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    fn pt(&self, x: i32, y: i32) -> Point {
        Point::new(x + self.offset_x, y + self.offset_y)
    }

    pub fn clear(&mut self, color: Color) {
        let _ = self.target.clear(color.into());
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let _ = Rectangle::new(self.pt(x, y), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_fill(color.into()))
            .draw(self.target);
    }

    pub fn stroke_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color, thickness: u32) {
        let _ = Rectangle::new(self.pt(x, y), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_stroke(color.into(), thickness))
            .draw(self.target);
    }

    pub fn fill_rounded_rect(&mut self, x: i32, y: i32, w: u32, h: u32, radius: u32, color: Color) {
        let _ = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.pt(x, y), Size::new(w, h)),
            Size::new(radius, radius),
        )
        .into_styled(PrimitiveStyle::with_fill(color.into()))
        .draw(self.target);
    }

    pub fn stroke_rounded_rect(
        &mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: Color,
        thickness: u32,
    ) {
        let _ = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.pt(x, y), Size::new(w, h)),
            Size::new(radius, radius),
        )
        .into_styled(PrimitiveStyle::with_stroke(color.into(), thickness))
        .draw(self.target);
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: u32, color: Color) {
        let top_left = self.pt(cx - radius as i32, cy - radius as i32);
        let _ = Circle::new(top_left, radius * 2)
            .into_styled(PrimitiveStyle::with_fill(color.into()))
            .draw(self.target);
    }

    pub fn stroke_circle(&mut self, cx: i32, cy: i32, radius: u32, color: Color, thickness: u32) {
        let top_left = self.pt(cx - radius as i32, cy - radius as i32);
        let _ = Circle::new(top_left, radius * 2)
            .into_styled(PrimitiveStyle::with_stroke(color.into(), thickness))
            .draw(self.target);
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color, thickness: u32) {
        let _ = Line::new(self.pt(x1, y1), self.pt(x2, y2))
            .into_styled(PrimitiveStyle::with_stroke(color.into(), thickness))
            .draw(self.target);
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, color: Color, font: Font) {
        const FONTS: [&embedded_graphics::mono_font::MonoFont; 4] =
            [&FONT_6X10, &FONT_7X13, &FONT_9X15, &FONT_10X20];
        let mono_font = FONTS[font as usize];
        let _ = Text::new(
            s,
            self.pt(x, y),
            MonoTextStyle::new(mono_font, color.into()),
        )
        .draw(self.target);
    }

    pub fn pixel(&mut self, x: i32, y: i32, color: Color) {
        let _ = Pixel(self.pt(x, y), Rgb565::from(color)).draw(self.target);
    }

    pub fn draw_image(&mut self, x: i32, y: i32, pixels: &[u16], img_w: u32, img_h: u32) {
        self.draw_image_scaled(x, y, pixels, img_w, img_h, img_w, img_h);
    }

    pub fn draw_image_scaled(
        &mut self,
        x: i32,
        y: i32,
        pixels: &[u16],
        img_w: u32,
        img_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) {
        // Build a row of pre-sampled pixels, then draw the whole row at once
        // to reduce per-pixel overhead through the DrawTarget trait.
        let mut row_buf: Vec<Pixel<Rgb565>> = Vec::with_capacity(dst_w as usize);
        for py in 0..dst_h {
            let src_y = (py * img_h / dst_h).min(img_h - 1);
            let dy = y + py as i32 + self.offset_y;
            if dy < 0 || dy >= DISPLAY_HEIGHT as i32 {
                continue;
            }
            row_buf.clear();
            let src_row = &pixels[(src_y * img_w) as usize..((src_y + 1) * img_w) as usize];
            for px in 0..dst_w {
                let dx = x + px as i32 + self.offset_x;
                if dx < 0 || dx >= DISPLAY_WIDTH as i32 {
                    continue;
                }
                let src_x = (px * img_w / dst_w).min(img_w - 1);
                let raw = src_row[src_x as usize];
                let color = Rgb565::new(
                    ((raw >> 11) & 0x1F) as u8,
                    ((raw >> 5) & 0x3F) as u8,
                    (raw & 0x1F) as u8,
                );
                row_buf.push(Pixel(Point::new(dx, dy), color));
            }
            let _ = self.target.draw_iter(row_buf.drain(..));
        }
    }

    pub fn text_ttf(&mut self, x: i32, y: i32, s: &str, color: Color, font: TtfFont) {
        self.text_bitmap(
            x,
            y,
            s,
            color,
            get_bitmap_font(font.family, font.weight, font.style, font.size),
        );
    }

    pub fn text_ttf_width(&self, s: &str, font: &TtfFont) -> i32 {
        let bf = get_bitmap_font(font.family, font.weight, font.style, font.size);
        let mut w = 0i32;
        for c in s.chars() {
            if let Some(glyph) = bf.glyph(c) {
                w += glyph.advance as i32;
            } else {
                w += (bf.size / 2) as i32;
            }
        }
        w
    }

    pub fn text_bitmap(&mut self, x: i32, y: i32, s: &str, color: Color, font: &super::BitmapFont) {
        let mut cursor_x = x;
        let base_y = y + font.ascent as i32;

        for c in s.chars() {
            if let Some(glyph) = font.glyph(c) {
                if glyph.width > 0 && glyph.height > 0 {
                    let data = font.glyph_data(glyph);
                    let gx = cursor_x + glyph.bearing_x as i32;
                    let gy = base_y + glyph.bearing_y as i32;

                    for py in 0..glyph.height as i32 {
                        for px in 0..glyph.width as i32 {
                            let alpha = data[(py * glyph.width as i32 + px) as usize];
                            if alpha > 32 {
                                self.pixel(gx + px, gy + py, color.with_alpha(alpha));
                            }
                        }
                    }
                }
                cursor_x += glyph.advance as i32;
            } else {
                cursor_x += (font.size / 2) as i32;
            }
        }
    }
}
