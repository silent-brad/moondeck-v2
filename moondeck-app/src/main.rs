use anyhow::{Context, Result};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyOutputPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::*;
use std::time::{SystemTime, UNIX_EPOCH};

use moondeck_core::gfx::{Color, DrawContext, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use moondeck_core::ui::{Event, Gesture, PageManager, WidgetInstance};
use moondeck_core::TtfFont;
use moondeck_core::util::FrameTimer;
use moondeck_hal::{Display, EnvConfig, FileSystem, Framebuffer, GestureProcessor, TouchController, WifiManager};
use moondeck_lua::{
    get_default_theme, init_boot_time, set_current_theme, set_system_info, set_wifi_status,
    LuaRuntime, ThemeColors, WidgetPlugin,
};

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();

    info!("=== Moondeck v2 Starting ===");
    info!("Display: {}x{}", DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // Initialize boot time for uptime tracking
    init_boot_time();

    // Set the default theme early so loading screen uses correct colors
    set_current_theme(get_default_theme());
    info!("Using theme: {}", get_default_theme());

    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let sysloop = EspSystemEventLoop::take().context("Failed to get event loop")?;
    let nvs = EspDefaultNvsPartition::take().ok();

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

    // Create framebuffer for loading screen
    let mut framebuffer = Framebuffer::new();

    // Show initial loading screen
    draw_loading_screen(&mut framebuffer, &mut display, "Initializing...", None)?;

    info!("Initializing touch controller...");
    let mut touch_controller = TouchController::new(
        peripherals.i2c0,
        peripherals.pins.gpio19, // SDA
        peripherals.pins.gpio20, // SCL
        Some(peripherals.pins.gpio38), // INT pin (must be LOW during init for address 0x5D)
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
    )?;
    info!("Touch controller initialized");

    draw_loading_screen(&mut framebuffer, &mut display, "Mounting filesystem...", None)?;

    info!("Mounting filesystem...");
    let fs = match FileSystem::mount("storage", "/data") {
        Ok(fs) => Some(fs),
        Err(e) => {
            warn!("Failed to mount SPIFFS: {}, using defaults", e);
            None
        }
    };

    draw_loading_screen(&mut framebuffer, &mut display, "Loading configuration...", None)?;

    info!("Loading environment configuration...");
    let env = if let Some(ref fs) = fs {
        if fs.exists(".env") {
            let content = fs.read_file(".env").unwrap_or_default();
            EnvConfig::load_from_str(&content)
        } else {
            // Try embedded .env from compile time
            let embedded = include_str!("../../.env");
            if !embedded.is_empty() {
                info!("Using embedded .env configuration");
                EnvConfig::load_from_str(embedded)
            } else {
                info!("No .env file found");
                EnvConfig::new()
            }
        }
    } else {
        // No SPIFFS, use embedded .env
        let embedded = include_str!("../../.env");
        if !embedded.is_empty() {
            info!("Using embedded .env configuration");
            EnvConfig::load_from_str(embedded)
        } else {
            EnvConfig::new()
        }
    };

    // Check if theme is specified in env and update
    if let Some(theme_name) = env.get("THEME") {
        set_current_theme(theme_name);
        info!("Theme set from config: {}", theme_name);
    }

    // Initialize WiFi if credentials are available
    let mut wifi_manager: Option<WifiManager> = None;
    let wifi_ssid = env.get("WIFI_SSID").unwrap_or("");
    let wifi_password = env.get("WIFI_PASSWORD").unwrap_or("");

    if !wifi_ssid.is_empty() {
        let wifi_msg = format!("Connecting to '{}'...", wifi_ssid);
        draw_loading_screen(&mut framebuffer, &mut display, &wifi_msg, Some("This may take a moment"))?;

        info!("Connecting to WiFi '{}'...", wifi_ssid);
        match WifiManager::new(peripherals.modem, sysloop.clone(), nvs.clone()) {
            Ok(mut wifi) => {
                match wifi.connect(wifi_ssid, wifi_password) {
                    Ok(()) => {
                        let status = wifi.status();
                        let ip_str = status.ip.map(|ip| ip.to_string()).unwrap_or_default();
                        let rssi = status.rssi.unwrap_or(-100) as i32;
                        info!("WiFi connected! IP: {}", ip_str);
                        set_wifi_status(true, wifi_ssid, &ip_str, rssi);
                        wifi_manager = Some(wifi);

                        let success_msg = format!("Connected: {}", ip_str);
                        draw_loading_screen(&mut framebuffer, &mut display, "WiFi Connected!", Some(&success_msg))?;
                        FreeRtos::delay_ms(500);
                    }
                    Err(e) => {
                        warn!("WiFi connection failed: {}", e);
                        set_wifi_status(false, "", "", -100);
                        draw_loading_screen(&mut framebuffer, &mut display, "WiFi Failed", Some("Continuing without network..."))?;
                        FreeRtos::delay_ms(1000);
                    }
                }
            }
            Err(e) => {
                warn!("WiFi initialization failed: {}", e);
                set_wifi_status(false, "", "", -100);
                draw_loading_screen(&mut framebuffer, &mut display, "WiFi Error", Some("Continuing without network..."))?;
                FreeRtos::delay_ms(1000);
            }
        }
    } else {
        info!("No WiFi credentials configured");
        set_wifi_status(false, "", "", -100);
    }

    draw_loading_screen(&mut framebuffer, &mut display, "Initializing Lua runtime...", None)?;

    info!("Initializing Lua runtime...");
    let mut lua_runtime = LuaRuntime::new()?;
    lua_runtime.init(&env)?;

    draw_loading_screen(&mut framebuffer, &mut display, "Loading pages...", None)?;

    info!("Loading pages configuration...");
    let pages = lua_runtime.load_pages()?;
    let mut page_manager = PageManager::new().with_pages(pages);
    info!("Loaded {} page(s)", page_manager.page_count());

    draw_loading_screen(&mut framebuffer, &mut display, "Initializing widgets...", None)?;

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

    // Get theme background color from Lua config
    let theme_name = lua_runtime.get_current_theme();
    let theme_bg_hex = lua_runtime.get_theme_background();
    let theme_bg = Color::from_hex(&theme_bg_hex).unwrap_or(Color::WHITE);
    info!("Theme: {}, background: {}", theme_name, theme_bg_hex);

    draw_loading_screen(&mut framebuffer, &mut display, "Ready!", None)?;
    FreeRtos::delay_ms(300);

    info!("Starting main loop...");
    run_main_loop(&mut lua_runtime, &mut page_manager, &mut plugins, &mut display, &mut touch_controller, wifi_manager, theme_bg, framebuffer)?;

    Ok(())
}

fn draw_loading_screen(
    framebuffer: &mut Framebuffer,
    display: &mut Display,
    message: &str,
    sub_message: Option<&str>,
) -> Result<()> {
    // Use theme colors
    let bg_color = Color::from_hex(ThemeColors::bg_primary()).unwrap_or(Color::WHITE);
    let text_color = Color::from_hex(ThemeColors::text_primary()).unwrap_or(Color::WHITE);
    let accent_color = Color::from_hex(ThemeColors::accent_primary()).unwrap_or(Color::WHITE);
    let muted_color = Color::from_hex(ThemeColors::text_muted()).unwrap_or(Color::WHITE);

    {
        let mut draw_ctx = DrawContext::new(framebuffer);
        draw_ctx.clear(bg_color);

        // Title - using Garamond font
        draw_ctx.text_ttf(
            (DISPLAY_WIDTH as i32 / 2) - 100,
            (DISPLAY_HEIGHT as i32 / 2) - 60,
            "Moondeck",
            accent_color,
            TtfFont::ebgaramond(60),
        );

        // Main message
        let msg_width = message.len() as i32 * 8;
        draw_ctx.text_ttf(
            (DISPLAY_WIDTH as i32 / 2) - (msg_width / 2),
            (DISPLAY_HEIGHT as i32 / 2) + 10,
            message,
            text_color,
            TtfFont::inter(20),
        );

        // Sub message if provided
        if let Some(sub) = sub_message {
            let sub_width = sub.len() as i32 * 6;
            draw_ctx.text_ttf(
                (DISPLAY_WIDTH as i32 / 2) - (sub_width / 2),
                (DISPLAY_HEIGHT as i32 / 2) + 45,
                sub,
                muted_color,
                TtfFont::inter(16),
            );
        }

        // Loading indicator line
        draw_ctx.fill_rect(
            (DISPLAY_WIDTH as i32 / 2) - 100,
            (DISPLAY_HEIGHT as i32 / 2) + 80,
            200,
            3,
            accent_color,
        );
    }

    display.flush(framebuffer)?;
    Ok(())
}

fn run_main_loop(
    lua_runtime: &mut LuaRuntime,
    page_manager: &mut PageManager,
    plugins: &mut [(WidgetPlugin, WidgetInstance)],
    display: &mut Display,
    touch_controller: &mut TouchController,
    wifi_manager: Option<WifiManager>,
    bg_color: Color,
    mut framebuffer: Framebuffer,
) -> Result<()> {
    let mut frame_timer = FrameTimer::new();
    let mut gesture_processor = GestureProcessor::new();
    let mut last_status_update: u64 = 0;
    let mut last_page_index = page_manager.current_index();

    // Helper closure to process touch - polls rapidly for up to 50ms to catch swipes
    let process_touch = |touch_controller: &mut TouchController, gesture_processor: &mut GestureProcessor, page_manager: &mut PageManager| {
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        // Poll for up to 50ms to catch fast swipes
        loop {
            let current_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            
            // Poll touch
            match touch_controller.poll() {
                Ok(Some(touch_event)) => {
                    if let Some(gesture) = gesture_processor.process(touch_event, current_ms) {
                        let event = Event::Gesture(gesture.clone());
                        if page_manager.handle_event(&event) {
                            match gesture {
                                Gesture::SwipeLeft => info!("Swiped left - now on page {}", page_manager.current_index() + 1),
                                Gesture::SwipeRight => info!("Swiped right - now on page {}", page_manager.current_index() + 1),
                                _ => {}
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No touch event - check if we should keep polling
                    if current_ms - start > 50 {
                        break;
                    }
                    FreeRtos::delay_ms(5);
                }
                Err(_) => break,
            }
        }
    };

    info!("Main loop running - press Ctrl+C to exit");

    loop {
        let current_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let delta_ms = frame_timer.tick(current_ms);

        // Process touch at start of frame
        process_touch(touch_controller, &mut gesture_processor, page_manager);

        // Update WiFi and system status every 5 seconds
        if current_ms - last_status_update >= 5000 {
            last_status_update = current_ms;

            // Update system info
            let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
            set_system_info(free_heap, 240); // ESP32-S3 runs at 240 MHz

            // Update WiFi status if we have a WiFi manager
            if let Some(ref wifi) = wifi_manager {
                let status = wifi.status();
                let ip_str = status.ip.map(|ip| ip.to_string()).unwrap_or_default();
                let rssi = status.rssi.unwrap_or(-100) as i32;
                set_wifi_status(
                    status.connected,
                    status.ssid.as_deref().unwrap_or(""),
                    &ip_str,
                    rssi,
                );
            }
        }

        // Process touch after WiFi update
        process_touch(touch_controller, &mut gesture_processor, page_manager);

        // Log page changes
        if page_manager.current_index() != last_page_index {
            info!("Page changed: {} -> {}", last_page_index + 1, page_manager.current_index() + 1);
            last_page_index = page_manager.current_index();
        }

        // Update plugins with touch polling between each one
        for (plugin, _widget) in plugins.iter() {
            process_touch(touch_controller, &mut gesture_processor, page_manager);
            let _ = plugin.update(lua_runtime, delta_ms);
        }
        
        // Process touch after all plugin updates
        process_touch(touch_controller, &mut gesture_processor, page_manager);

        // Get theme colors for UI elements
        let ui_text_color = Color::from_hex(ThemeColors::text_muted()).unwrap_or(Color::WHITE);

        // Render
        {
            let mut draw_ctx = DrawContext::new(&mut framebuffer);
            draw_ctx.clear(bg_color);

            if let Some(page) = page_manager.current_page() {
                // Page indicator at bottom - using theme colors
                let page_indicator = format!(
                    "Page {}/{}: {}",
                    page_manager.current_index() + 1,
                    page_manager.page_count(),
                    &page.title
                );
                draw_ctx.text_ttf(
                    10,
                    DISPLAY_HEIGHT as i32 - 20,
                    &page_indicator,
                    ui_text_color,
                    TtfFont::inter(18),
                );

                // FPS counter - using theme colors
                let fps_text = format!("FPS: {:.1}", frame_timer.fps());
                draw_ctx.text_ttf(
                    DISPLAY_WIDTH as i32 - 80,
                    DISPLAY_HEIGHT as i32 - 20,
                    &fps_text,
                    ui_text_color,
                    TtfFont::inter(18),
                );

                // Render widgets for current page
                // Match by both page and widget to handle same module on multiple pages
                for (plugin, widget) in plugins.iter() {
                    // Check if this widget belongs to the current page
                    let belongs_to_page = page.widgets.iter().any(|w| {
                        w.module == widget.module &&
                        w.context.x == widget.context.x &&
                        w.context.y == widget.context.y
                    });

                    if belongs_to_page {
                        if let Err(e) = plugin.render(lua_runtime, &widget.context, &mut draw_ctx) {
                            warn!("Render error for '{}': {}", widget.module, e);
                        }
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
