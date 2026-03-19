use anyhow::{Context, Result};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::{I2c, I2cConfig, I2cDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::Hertz;
use moondeck_core::ui::{TouchEvent, TouchPhase};

const GT911_ADDR_PRIMARY: u8 = 0x5D;
const GT911_ADDR_SECONDARY: u8 = 0x14;
const GT911_PRODUCT_ID_REG: u16 = 0x8140;
const GT911_TOUCH_STATUS_REG: u16 = 0x814E;
const GT911_POINT1_REG: u16 = 0x814F;

pub struct TouchController<'d> {
    i2c: I2cDriver<'d>,
    address: u8,
    last_touch: Option<TouchEvent>,
    start_x: i32,
    start_y: i32,
    last_x: i32,
    last_y: i32,
    width: u32,
    height: u32,
}

impl<'d> TouchController<'d> {
    pub fn new<I2C: I2c>(
        i2c: impl Peripheral<P = I2C> + 'd,
        sda: impl Peripheral<P = impl InputPin + OutputPin> + 'd,
        scl: impl Peripheral<P = impl InputPin + OutputPin> + 'd,
        int_pin: Option<impl Peripheral<P = impl InputPin + OutputPin> + 'd>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // The GT911 INT pin (GPIO38 on Elecrow) must be set LOW during init
        // to select I2C address 0x5D. After init it becomes an input for interrupts.
        if let Some(int) = int_pin {
            let mut int_driver = PinDriver::output(int)?;
            int_driver.set_low()?;
            FreeRtos::delay_ms(50);
        }

        // Use slower I2C speed for better compatibility
        let config = I2cConfig::new().baudrate(Hertz(100_000));
        let i2c_driver =
            I2cDriver::new(i2c, sda, scl, &config).context("Failed to initialize I2C")?;

        let mut controller = Self {
            i2c: i2c_driver,
            address: GT911_ADDR_PRIMARY,
            last_touch: None,
            start_x: 0,
            start_y: 0,
            last_x: 0,
            last_y: 0,
            width,
            height,
        };

        // Wait for GT911 to be ready after power-up
        FreeRtos::delay_ms(100);

        // Try primary address first, then secondary
        let mut found = false;
        for retry in 0..3 {
            if controller.read_register(GT911_PRODUCT_ID_REG, 4).is_ok() {
                found = true;
                break;
            }
            log::warn!("GT911 not responding at 0x5D, retry {}", retry + 1);
            FreeRtos::delay_ms(50);
        }

        if !found {
            controller.address = GT911_ADDR_SECONDARY;
            for retry in 0..3 {
                if controller.read_register(GT911_PRODUCT_ID_REG, 4).is_ok() {
                    found = true;
                    break;
                }
                log::warn!("GT911 not responding at 0x14, retry {}", retry + 1);
                FreeRtos::delay_ms(50);
            }
        }

        if !found {
            anyhow::bail!("GT911 not found at either address (0x5D or 0x14)");
        }

        log::info!(
            "GT911 touch controller initialized at address 0x{:02X}",
            controller.address
        );

        Ok(controller)
    }

    fn write_register(&mut self, reg: u16, data: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; 2 + data.len()];
        buf[0] = (reg >> 8) as u8;
        buf[1] = (reg & 0xFF) as u8;
        buf[2..].copy_from_slice(data);

        self.i2c
            .write(self.address, &buf, 100)
            .map_err(|e| anyhow::anyhow!("I2C write error: {:?}", e))?;
        Ok(())
    }

    fn read_register(&mut self, reg: u16, len: usize) -> Result<Vec<u8>> {
        let reg_bytes = [(reg >> 8) as u8, (reg & 0xFF) as u8];
        let mut buf = vec![0u8; len];

        self.i2c
            .write(self.address, &reg_bytes, 100)
            .map_err(|e| anyhow::anyhow!("I2C write error: {:?}", e))?;
        self.i2c
            .read(self.address, &mut buf, 100)
            .map_err(|e| anyhow::anyhow!("I2C read error: {:?}", e))?;

        Ok(buf)
    }

    pub fn poll(&mut self) -> Result<Option<TouchEvent>> {
        let status = self.read_register(GT911_TOUCH_STATUS_REG, 1)?;
        let buffer_ready = (status[0] & 0x80) != 0;

        if !buffer_ready {
            // Data is not valid yet — skip this poll cycle
            return Ok(None);
        }

        let num_touches = status[0] & 0x0F;
        self.write_register(GT911_TOUCH_STATUS_REG, &[0])?;

        if num_touches == 0 {
            // No touch - emit Ended if we had an active touch
            if self.last_touch.take().is_some() {
                log::info!(
                    "Touch ended: start=({},{}) last=({},{})",
                    self.start_x,
                    self.start_y,
                    self.last_x,
                    self.last_y
                );
                return Ok(Some(TouchEvent {
                    x: self.last_x,
                    y: self.last_y,
                    phase: TouchPhase::Ended,
                }));
            }
            return Ok(None);
        }

        // We have a touch - read the current position
        let point_data = self.read_register(GT911_POINT1_REG, 8)?;

        let x = ((point_data[1] as u32) | ((point_data[2] as u32) << 8)) as i32;
        let y = ((point_data[3] as u32) | ((point_data[4] as u32) << 8)) as i32;

        let x = x.clamp(0, self.width as i32 - 1);
        let y = y.clamp(0, self.height as i32 - 1);

        // Track latest position for Ended events
        self.last_x = x;
        self.last_y = y;

        if self.last_touch.is_some() {
            // Already tracking - this is a move
            let event = TouchEvent {
                x,
                y,
                phase: TouchPhase::Moved,
            };
            self.last_touch = Some(event);
            Ok(Some(event))
        } else {
            // New touch - record start position
            self.start_x = x;
            self.start_y = y;
            let event = TouchEvent {
                x,
                y,
                phase: TouchPhase::Started,
            };
            self.last_touch = Some(event);
            Ok(Some(event))
        }
    }

    pub fn is_touched(&mut self) -> Result<bool> {
        let status = self.read_register(GT911_TOUCH_STATUS_REG, 1)?;
        Ok((status[0] & 0x0F) > 0)
    }
}
