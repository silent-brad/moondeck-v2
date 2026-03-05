use anyhow::{Context, Result};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyOutputPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::*;
use std::time::{SystemTime, UNIX_EPOCH};

use moondeck_core::gfx::{Color, DrawContext, Font, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use moondeck_core::ui::{GestureDetector, PageManager, WidgetInstance};
use moondeck_core::util::FrameTimer;
use moondeck_hal::{Display, EnvConfig, FileSystem, Framebuffer};
use moondeck_lua::{LuaRuntime, WidgetPlugin};

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();

    info!("=== Moondeck v2 Starting ===");
    info!("Display: {}x{}", DISPLAY_WIDTH, DISPLAY_HEIGHT);

    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let _sysloop = EspSystemEventLoop::take().context("Failed to get event loop")?;
    let _nvs = EspDefaultNvsPartition::take().ok();

    info!("Initializing display...");
    // Elecrow CrowPanel 5-inch (800x480) RGB565 pinout
    // Data pins in order D0-D15 for ESP-IDF RGB panel (B0-B4, G0-G5, R0-R4)
    let data_pins: [AnyOutputPin; 16] = [
        peripherals.pins.gpio8.into(),  // D0  = B0
        peripherals.pins.gpio3.into(),  // D1  = B1
        peripherals.pins.gpio46.into(), // D2  = B2
        peripherals.pins.gpio9.into(),  // D3  = B3
        peripherals.pins.gpio1.into(),  // D4  = B4
        peripherals.pins.gpio5.into(),  // D5  = G0
        peripherals.pins.gpio6.into(),  // D6  = G1
        peripherals.pins.gpio7.into(),  // D7  = G2
        peripherals.pins.gpio15.into(), // D8  = G3
        peripherals.pins.gpio16.into(), // D9  = G4
        peripherals.pins.gpio4.into(),  // D10 = G5
        peripherals.pins.gpio45.into(), // D11 = R0
        peripherals.pins.gpio48.into(), // D12 = R1
        peripherals.pins.gpio47.into(), // D13 = R2
        peripherals.pins.gpio21.into(), // D14 = R3
        peripherals.pins.gpio14.into(), // D15 = R4
    ];

    let mut display = Display::new_elecrow_5inch(
        peripherals.pins.gpio40, // DE
        peripherals.pins.gpio41, // VSYNC
        peripherals.pins.gpio39, // HSYNC
        peripherals.pins.gpio0,  // PCLK
        data_pins,
        Some(peripherals.pins.gpio2.into()), // Backlight
    )?;

    info!("Mounting filesystem...");
    let fs = match FileSystem::mount("storage", "/data") {
        Ok(fs) => Some(fs),
        Err(e) => {
            warn!("Failed to mount SPIFFS: {}, using defaults", e);
            None
        }
    };

    info!("Loading environment configuration...");
    let env = if let Some(ref fs) = fs {
        if fs.exists(".env") {
            let content = fs.read_file(".env").unwrap_or_default();
            EnvConfig::load_from_str(&content)
        } else {
            info!("No .env file found");
            EnvConfig::new()
        }
    } else {
        EnvConfig::new()
    };

    info!("Initializing Lua runtime...");
    let mut lua_runtime = LuaRuntime::new()?;
    lua_runtime.init(&env)?;

    info!("Loading pages configuration...");
    let pages = lua_runtime.load_pages()?;
    let mut page_manager = PageManager::new().with_pages(pages);
    info!("Loaded {} page(s)", page_manager.page_count());

    let mut plugins: Vec<(WidgetPlugin, WidgetInstance)> = Vec::new();
    let mut plugin_index = 0;

    for page in page_manager.pages() {
        for widget in &page.widgets {
            let mut plugin = WidgetPlugin::new(&widget.module, plugin_index);
            if let Err(e) = plugin.init(&mut lua_runtime, &widget.context) {
                warn!("Failed to init widget '{}': {}", widget.module, e);
            }
            plugins.push((plugin, widget.clone()));
            plugin_index += 1;
        }
    }

    info!("Initialized {} widget(s)", plugins.len());

    info!("Starting main loop...");
    run_main_loop(&mut lua_runtime, &mut page_manager, &mut plugins, &mut display)?;

    Ok(())
}

fn run_main_loop(
    lua_runtime: &mut LuaRuntime,
    page_manager: &mut PageManager,
    plugins: &mut [(WidgetPlugin, WidgetInstance)],
    display: &mut Display,
) -> Result<()> {
    let mut framebuffer = Framebuffer::new();
    let mut frame_timer = FrameTimer::new();
    let _gesture_detector = GestureDetector::default();

    info!("Main loop running - press Ctrl+C to exit");

    loop {
        let current_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let delta_ms = frame_timer.tick(current_ms);

        for (plugin, _widget) in plugins.iter() {
            let _ = plugin.update(lua_runtime, delta_ms);
        }

        let bg_color = page_manager
            .current_page()
            .and_then(|p| p.background_color.as_ref())
            .and_then(|c| Color::from_hex(c))
            .unwrap_or(Color::BLACK);

        {
            let mut draw_ctx = DrawContext::new(&mut framebuffer);
            draw_ctx.clear(bg_color);

            if let Some(page) = page_manager.current_page() {
                let page_indicator = format!(
                    "Page {}/{}: {}",
                    page_manager.current_index() + 1,
                    page_manager.page_count(),
                    &page.title
                );
                draw_ctx.text(
                    10,
                    DISPLAY_HEIGHT as i32 - 20,
                    &page_indicator,
                    Color::GRAY,
                    Font::Small,
                );

                let fps_text = format!("FPS: {:.1}", frame_timer.fps());
                draw_ctx.text(
                    DISPLAY_WIDTH as i32 - 80,
                    DISPLAY_HEIGHT as i32 - 20,
                    &fps_text,
                    Color::GRAY,
                    Font::Small,
                );

                for (plugin, widget) in plugins.iter() {
                    if page.widgets.iter().any(|w| w.module == widget.module) {
                        let _ = plugin.render(lua_runtime, &widget.context, &mut draw_ctx);
                    }
                }
            }
        }

        display.flush(&framebuffer)?;

        if frame_timer.frame_count() % 300 == 0 {
            info!(
                "Frame {}: FPS={:.1}, Page={}",
                frame_timer.frame_count(),
                frame_timer.fps(),
                page_manager.current_index() + 1
            );
        }

        let target_frame_ms: u64 = 33;
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
            .saturating_sub(current_ms);

        if elapsed < target_frame_ms {
            FreeRtos::delay_ms((target_frame_ms - elapsed) as u32);
        } else {
            // Always yield to feed the watchdog
            FreeRtos::delay_ms(1);
        }
    }
}
