use super::Framebuffer;
use anyhow::{Context, Result};
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_sys as sys;
use moondeck_core::gfx::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

pub struct DisplayConfig {
    pub hsync_pulse_width: u32,
    pub hsync_back_porch: u32,
    pub hsync_front_porch: u32,
    pub vsync_pulse_width: u32,
    pub vsync_back_porch: u32,
    pub vsync_front_porch: u32,
    pub pclk_hz: u32,
    pub pclk_active_neg: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self::elecrow_5inch()
    }
}

impl DisplayConfig {
    pub fn elecrow_5inch() -> Self {
        Self {
            hsync_pulse_width: 4,
            hsync_back_porch: 8,
            hsync_front_porch: 8,
            vsync_pulse_width: 4,
            vsync_back_porch: 8,
            vsync_front_porch: 8,
            pclk_hz: 15_000_000,
            pclk_active_neg: true,
        }
    }
}

pub struct Display {
    panel_handle: sys::esp_lcd_panel_handle_t,
    _backlight_pin: Option<PinDriver<'static, AnyOutputPin, Output>>,
}

impl Display {
    pub fn new_elecrow_5inch(
        de_pin: impl Peripheral<P = impl OutputPin> + 'static,
        vsync_pin: impl Peripheral<P = impl OutputPin> + 'static,
        hsync_pin: impl Peripheral<P = impl OutputPin> + 'static,
        pclk_pin: impl Peripheral<P = impl OutputPin> + 'static,
        data_pins: [AnyOutputPin; 16],
        backlight_pin: Option<AnyOutputPin>,
    ) -> Result<Self> {
        Self::new_with_config(
            de_pin,
            vsync_pin,
            hsync_pin,
            pclk_pin,
            data_pins,
            backlight_pin,
            DisplayConfig::default(),
        )
    }

    pub fn new_with_config(
        de_pin: impl Peripheral<P = impl OutputPin> + 'static,
        vsync_pin: impl Peripheral<P = impl OutputPin> + 'static,
        hsync_pin: impl Peripheral<P = impl OutputPin> + 'static,
        pclk_pin: impl Peripheral<P = impl OutputPin> + 'static,
        data_pins: [AnyOutputPin; 16],
        backlight_pin: Option<AnyOutputPin>,
        config: DisplayConfig,
    ) -> Result<Self> {
        let mut backlight_driver = None;

        if let Some(bl_pin) = backlight_pin {
            let mut driver = PinDriver::output(bl_pin)?;
            driver.set_high()?;
            backlight_driver = Some(driver);
        }

        let panel_config = sys::esp_lcd_rgb_panel_config_t {
            clk_src: sys::soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT,
            timings: sys::esp_lcd_rgb_timing_t {
                pclk_hz: config.pclk_hz,
                h_res: DISPLAY_WIDTH,
                v_res: DISPLAY_HEIGHT,
                hsync_pulse_width: config.hsync_pulse_width,
                hsync_back_porch: config.hsync_back_porch,
                hsync_front_porch: config.hsync_front_porch,
                vsync_pulse_width: config.vsync_pulse_width,
                vsync_back_porch: config.vsync_back_porch,
                vsync_front_porch: config.vsync_front_porch,
                flags: sys::esp_lcd_rgb_timing_t__bindgen_ty_1 {
                    _bitfield_1: sys::esp_lcd_rgb_timing_t__bindgen_ty_1::new_bitfield_1(
                        0, // hsync_idle_low
                        0, // vsync_idle_low
                        0, // de_idle_high
                        config.pclk_active_neg as u32, // pclk_active_neg
                        0, // pclk_idle_high
                    ),
                    ..Default::default()
                },
            },
            data_width: 16,
            bits_per_pixel: 16,
            num_fbs: 1,
            bounce_buffer_size_px: 10 * DISPLAY_WIDTH as usize,
            sram_trans_align: 8,
            psram_trans_align: 64,
            hsync_gpio_num: hsync_pin.into_ref().pin(),
            vsync_gpio_num: vsync_pin.into_ref().pin(),
            de_gpio_num: de_pin.into_ref().pin(),
            pclk_gpio_num: pclk_pin.into_ref().pin(),
            disp_gpio_num: -1,
            data_gpio_nums: [
                data_pins[0].pin(),
                data_pins[1].pin(),
                data_pins[2].pin(),
                data_pins[3].pin(),
                data_pins[4].pin(),
                data_pins[5].pin(),
                data_pins[6].pin(),
                data_pins[7].pin(),
                data_pins[8].pin(),
                data_pins[9].pin(),
                data_pins[10].pin(),
                data_pins[11].pin(),
                data_pins[12].pin(),
                data_pins[13].pin(),
                data_pins[14].pin(),
                data_pins[15].pin(),
            ],
            flags: sys::esp_lcd_rgb_panel_config_t__bindgen_ty_1 {
                _bitfield_1: sys::esp_lcd_rgb_panel_config_t__bindgen_ty_1::new_bitfield_1(
                    0, // disp_active_low
                    0, // refresh_on_demand
                    1, // fb_in_psram
                    0, // double_fb
                    0, // no_fb
                    0, // bb_invalidate_cache
                ),
                ..Default::default()
            },
        };

        let mut panel_handle: sys::esp_lcd_panel_handle_t = std::ptr::null_mut();
        unsafe {
            sys::esp!(sys::esp_lcd_new_rgb_panel(&panel_config, &mut panel_handle))
                .context("Failed to create RGB panel")?;

            sys::esp!(sys::esp_lcd_panel_reset(panel_handle))
                .context("Failed to reset panel")?;

            sys::esp!(sys::esp_lcd_panel_init(panel_handle))
                .context("Failed to initialize panel")?;

            // Mirror both axes for correct orientation (Elecrow 5-inch)
            sys::esp!(sys::esp_lcd_panel_mirror(panel_handle, true, true))
                .context("Failed to mirror panel")?;
        }

        log::info!("Display initialized: {}x{}", DISPLAY_WIDTH, DISPLAY_HEIGHT);

        Ok(Self {
            panel_handle,
            _backlight_pin: backlight_driver,
        })
    }

    pub fn flush(&mut self, framebuffer: &Framebuffer) -> Result<()> {
        unsafe {
            sys::esp!(sys::esp_lcd_panel_draw_bitmap(
                self.panel_handle,
                0,
                0,
                DISPLAY_WIDTH as i32,
                DISPLAY_HEIGHT as i32,
                framebuffer.as_bytes().as_ptr() as *const _,
            ))
            .context("Failed to draw bitmap")?;
        }
        Ok(())
    }

    pub fn set_backlight(&mut self, on: bool) -> Result<()> {
        if let Some(ref mut bl) = self._backlight_pin {
            if on {
                bl.set_high()?;
            } else {
                bl.set_low()?;
            }
        }
        Ok(())
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            let _ = sys::esp_lcd_panel_del(self.panel_handle);
        }
    }
}
