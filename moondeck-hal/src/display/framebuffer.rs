use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::Rgb565,
    Pixel,
};
use moondeck_core::gfx::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_SIZE};

pub struct Framebuffer {
    buffer: Box<[u16; (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize]>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Self {
            buffer: Box::new([0u16; (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize]),
        }
    }

    pub fn clear(&mut self, color: u16) {
        self.buffer.fill(color);
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr() as *const u8, FRAMEBUFFER_SIZE) }
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.buffer.as_mut_ptr() as *mut u8, FRAMEBUFFER_SIZE)
        }
    }

    pub fn as_u16_slice(&self) -> &[u16] {
        &self.buffer[..]
    }

    pub fn as_mut_u16_slice(&mut self) -> &mut [u16] {
        &mut self.buffer[..]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u16) {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT {
            self.buffer[(y * DISPLAY_WIDTH + x) as usize] = color;
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u16 {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT {
            self.buffer[(y * DISPLAY_WIDTH + x) as usize]
        } else {
            0
        }
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0
                && coord.y >= 0
                && (coord.x as u32) < DISPLAY_WIDTH
                && (coord.y as u32) < DISPLAY_HEIGHT
            {
                use embedded_graphics_core::pixelcolor::RgbColor;
                let r = color.r() as u16;
                let g = color.g() as u16;
                let b = color.b() as u16;
                let rgb565 = (r << 11) | (g << 5) | b;
                self.buffer[(coord.y as u32 * DISPLAY_WIDTH + coord.x as u32) as usize] = rgb565;
            }
        }
        Ok(())
    }
}
