use super::{Color, Font};
use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, RoundedRectangle, Triangle},
    text::Text,
};

pub const DISPLAY_WIDTH: u32 = 800;
pub const DISPLAY_HEIGHT: u32 = 480;
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 2) as usize;

pub struct DrawContext<'a, T: DrawTarget<Color = Rgb565>> {
    target: &'a mut T,
    offset_x: i32,
    offset_y: i32,
    clip_x: i32,
    clip_y: i32,
    clip_w: u32,
    clip_h: u32,
}

impl<'a, T: DrawTarget<Color = Rgb565>> DrawContext<'a, T> {
    pub fn new(target: &'a mut T) -> Self {
        Self {
            target,
            offset_x: 0,
            offset_y: 0,
            clip_x: 0,
            clip_y: 0,
            clip_w: DISPLAY_WIDTH,
            clip_h: DISPLAY_HEIGHT,
        }
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    pub fn with_clip(mut self, x: i32, y: i32, w: u32, h: u32) -> Self {
        self.clip_x = x;
        self.clip_y = y;
        self.clip_w = w;
        self.clip_h = h;
        self
    }

    fn translate(&self, x: i32, y: i32) -> Point {
        Point::new(x + self.offset_x, y + self.offset_y)
    }

    pub fn clear(&mut self, color: Color) {
        let _ = self.target.clear(color.into());
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let rect = Rectangle::new(self.translate(x, y), Size::new(w, h));
        let style = PrimitiveStyle::with_fill(Rgb565::from(color));
        let _ = rect.into_styled(style).draw(self.target);
    }

    pub fn stroke_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color, thickness: u32) {
        let rect = Rectangle::new(self.translate(x, y), Size::new(w, h));
        let style = PrimitiveStyle::with_stroke(Rgb565::from(color), thickness);
        let _ = rect.into_styled(style).draw(self.target);
    }

    pub fn fill_rounded_rect(&mut self, x: i32, y: i32, w: u32, h: u32, radius: u32, color: Color) {
        let rect = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.translate(x, y), Size::new(w, h)),
            Size::new(radius, radius),
        );
        let style = PrimitiveStyle::with_fill(Rgb565::from(color));
        let _ = rect.into_styled(style).draw(self.target);
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
        let rect = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.translate(x, y), Size::new(w, h)),
            Size::new(radius, radius),
        );
        let style = PrimitiveStyle::with_stroke(Rgb565::from(color), thickness);
        let _ = rect.into_styled(style).draw(self.target);
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: u32, color: Color) {
        let top_left = Point::new(cx - radius as i32, cy - radius as i32);
        let circle = Circle::new(self.translate(top_left.x, top_left.y), radius * 2);
        let style = PrimitiveStyle::with_fill(Rgb565::from(color));
        let _ = circle.into_styled(style).draw(self.target);
    }

    pub fn stroke_circle(&mut self, cx: i32, cy: i32, radius: u32, color: Color, thickness: u32) {
        let top_left = Point::new(cx - radius as i32, cy - radius as i32);
        let circle = Circle::new(self.translate(top_left.x, top_left.y), radius * 2);
        let style = PrimitiveStyle::with_stroke(Rgb565::from(color), thickness);
        let _ = circle.into_styled(style).draw(self.target);
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color, thickness: u32) {
        let line = Line::new(self.translate(x1, y1), self.translate(x2, y2));
        let style = PrimitiveStyle::with_stroke(Rgb565::from(color), thickness);
        let _ = line.into_styled(style).draw(self.target);
    }

    pub fn triangle(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        color: Color,
        filled: bool,
    ) {
        let tri = Triangle::new(
            self.translate(x1, y1),
            self.translate(x2, y2),
            self.translate(x3, y3),
        );
        let style = if filled {
            PrimitiveStyle::with_fill(Rgb565::from(color))
        } else {
            PrimitiveStyle::with_stroke(Rgb565::from(color), 1)
        };
        let _ = tri.into_styled(style).draw(self.target);
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, color: Color, font: Font) {
        let rgb_color = Rgb565::from(color);
        match font {
            Font::Small => {
                let style = MonoTextStyle::new(&FONT_6X10, rgb_color);
                let _ = Text::new(s, self.translate(x, y), style).draw(self.target);
            }
            Font::Medium => {
                let style = MonoTextStyle::new(&FONT_7X13, rgb_color);
                let _ = Text::new(s, self.translate(x, y), style).draw(self.target);
            }
            Font::Large => {
                let style = MonoTextStyle::new(&FONT_9X15, rgb_color);
                let _ = Text::new(s, self.translate(x, y), style).draw(self.target);
            }
            Font::XLarge => {
                let style = MonoTextStyle::new(&FONT_10X20, rgb_color);
                let _ = Text::new(s, self.translate(x, y), style).draw(self.target);
            }
        }
    }

    pub fn pixel(&mut self, x: i32, y: i32, color: Color) {
        let _ = Pixel(self.translate(x, y), Rgb565::from(color)).draw(self.target);
    }
}
