use anyhow::{Context, Result};
use embedded_svc::http::Method;
use embedded_svc::io::Write;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyOutputPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{Configuration as HttpServerConfig, EspHttpServer};
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use moondeck_core::gfx::{Color, DrawContext, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use moondeck_core::ui::{Event, Gesture, PageManager, WidgetInstance};
use moondeck_core::util::FrameTimer;
use moondeck_core::TtfFont;
use moondeck_hal::{
    Display, EnvConfig, FileSystem, Framebuffer, GestureProcessor, TouchController, WifiManager,
};
use moondeck_lua::{
    embedded_widget_sources, get_default_theme, init_boot_time, set_current_theme, set_system_info,
    set_wifi_status, LuaRuntime, ThemeColors, WidgetPlugin, EMBEDDED_INIT_LUA, EMBEDDED_PAGES_LUA,
};

const CONFIG_PATH: &str = "/data/config";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn seed_lua_config(fs: &FileSystem) {
    let _ = fs.create_dir("config");
    let _ = fs.create_dir("config/widgets");

    if !fs.exists("config/init.lua") {
        if let Err(e) = fs.write_file("config/init.lua", EMBEDDED_INIT_LUA) {
            warn!("Failed to seed init.lua: {}", e);
        }
    }
    if !fs.exists("config/pages.lua") {
        if let Err(e) = fs.write_file("config/pages.lua", EMBEDDED_PAGES_LUA) {
            warn!("Failed to seed pages.lua: {}", e);
        }
    }
    for (module_name, source) in embedded_widget_sources() {
        // module_name is like "widgets.clock" -> "config/widgets/clock.lua"
        let filename = module_name.strip_prefix("widgets.").unwrap_or(module_name);
        let rel = format!("config/widgets/{}.lua", filename);
        if !fs.exists(&rel) {
            if let Err(e) = fs.write_file(&rel, source) {
                warn!("Failed to seed {}: {}", rel, e);
            }
        }
    }
    info!("Lua config seeded on SPIFFS");
}

fn init_lua_and_widgets(
    env: &EnvConfig,
) -> Result<(
    LuaRuntime,
    PageManager,
    Vec<(WidgetPlugin, WidgetInstance)>,
    Color,
)> {
    let mut lua = LuaRuntime::new()?.with_config_path(CONFIG_PATH);
    lua.init(env)?;

    let pages = lua.load_pages()?;
    let pm = PageManager::new().with_pages(pages);
    info!("Loaded {} page(s)", pm.page_count());

    let plugins: Vec<(WidgetPlugin, WidgetInstance)> = pm
        .pages()
        .iter()
        .flat_map(|p| p.widgets.iter())
        .enumerate()
        .map(|(i, w)| {
            let mut p = WidgetPlugin::new(&w.module, i);
            if let Err(e) = p.init(&mut lua, &w.context) {
                warn!("Widget init failed: {}", e);
            }
            (p, w.clone())
        })
        .collect();

    let bg = Color::from_hex(&lua.get_theme_background()).unwrap_or(Color::WHITE);
    Ok((lua, pm, plugins, bg))
}

fn start_http_server(reload_gen: Arc<AtomicU32>) -> Result<EspHttpServer<'static>> {
    let server_config = HttpServerConfig {
        stack_size: 8192,
        max_uri_handlers: 4,
        ..Default::default()
    };
    let mut server = EspHttpServer::new(&server_config)?;

    // POST /reload — trigger Lua reload
    let reload = reload_gen.clone();
    server.fn_handler::<anyhow::Error, _>("/reload", Method::Post, move |req| {
        reload.fetch_add(1, Ordering::SeqCst);
        info!("Reload requested via HTTP");
        req.into_ok_response()?.write_all(b"OK\n")?;
        Ok(())
    })?;

    // POST /upload?path=config/widgets/clock.lua — upload a file to SPIFFS
    let mount = "/data".to_string();
    server.fn_handler::<anyhow::Error, _>("/upload", Method::Post, move |mut req| {
        let uri = req.uri().to_string();
        let path = uri
            .split("path=")
            .nth(1)
            .and_then(|p| p.split('&').next())
            .unwrap_or("")
            .to_string();

        if path.is_empty() || path.contains("..") || !path.starts_with("config/") {
            req.into_status_response(400)?
                .write_all(b"Invalid path\n")?;
            return Ok(());
        }

        // Read request body before consuming the request
        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match req.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
                Err(e) => {
                    warn!("Upload read error: {:?}", e);
                    break;
                }
            }
        }

        let full_path = format!("{}/{}", mount, path);

        std::fs::write(&full_path, &body).map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;

        info!("Uploaded: {} ({} bytes)", path, body.len());
        req.into_ok_response()?.write_all(b"OK\n")?;
        Ok(())
    })?;

    info!("HTTP server started (POST /upload, POST /reload)");
    Ok(server)
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    info!("=== Moondeck v2 Starting ===");

    init_boot_time();
    set_current_theme(get_default_theme());

    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let sysloop = EspSystemEventLoop::take().context("Failed to get event loop")?;
    let nvs = EspDefaultNvsPartition::take().ok();

    let data_pins: [AnyOutputPin; 16] = [
        peripherals.pins.gpio8.into(),
        peripherals.pins.gpio3.into(),
        peripherals.pins.gpio46.into(),
        peripherals.pins.gpio9.into(),
        peripherals.pins.gpio1.into(),
        peripherals.pins.gpio5.into(),
        peripherals.pins.gpio6.into(),
        peripherals.pins.gpio7.into(),
        peripherals.pins.gpio15.into(),
        peripherals.pins.gpio16.into(),
        peripherals.pins.gpio4.into(),
        peripherals.pins.gpio45.into(),
        peripherals.pins.gpio48.into(),
        peripherals.pins.gpio47.into(),
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio14.into(),
    ];

    let mut display = Display::new_elecrow_5inch(
        peripherals.pins.gpio40,
        peripherals.pins.gpio41,
        peripherals.pins.gpio39,
        peripherals.pins.gpio0,
        data_pins,
        Some(peripherals.pins.gpio2.into()),
    )?;

    let mut fb = Framebuffer::new();
    let loading = |fb: &mut Framebuffer, d: &mut Display, msg: &str, sub: Option<&str>| {
        draw_loading_screen(fb, d, msg, sub)
    };

    loading(&mut fb, &mut display, "Initializing...", None)?;

    let mut touch = TouchController::new(
        peripherals.i2c0,
        peripherals.pins.gpio19,
        peripherals.pins.gpio20,
        Some(peripherals.pins.gpio38),
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
    )?;

    loading(&mut fb, &mut display, "Loading configuration...", None)?;

    let fs = FileSystem::mount("storage", "/data")?;
    seed_lua_config(&fs);
    let env = load_env_config(Some(&fs));
    if let Some(theme) = env.get("THEME") {
        set_current_theme(theme);
    }

    let wifi = init_wifi(&env, &mut fb, &mut display, peripherals.modem, sysloop, nvs)?;

    // Start HTTP server for live-reload (only if WiFi connected)
    let reload_gen = Arc::new(AtomicU32::new(0));
    let _server = if wifi.is_some() {
        match start_http_server(reload_gen.clone()) {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("HTTP server failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    loading(&mut fb, &mut display, "Initializing Lua...", None)?;
    let (mut lua, mut pm, mut plugins, bg) = init_lua_and_widgets(&env)?;

    loading(&mut fb, &mut display, "Ready!", None)?;
    FreeRtos::delay_ms(300);

    run_loop(
        &mut lua,
        &mut pm,
        &mut plugins,
        &mut display,
        &mut touch,
        wifi,
        bg,
        fb,
        &env,
        &reload_gen,
    )
}

fn load_env_config(fs: Option<&FileSystem>) -> EnvConfig {
    let embedded = include_str!("../../.env");
    fs.and_then(|f| f.exists(".env").then(|| f.read_file(".env").ok()).flatten())
        .map(|c| EnvConfig::load_from_str(&c))
        .or_else(|| (!embedded.is_empty()).then(|| EnvConfig::load_from_str(embedded)))
        .unwrap_or_else(EnvConfig::new)
}

fn init_wifi(
    env: &EnvConfig,
    fb: &mut Framebuffer,
    display: &mut Display,
    modem: esp_idf_hal::modem::Modem,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspDefaultNvsPartition>,
) -> Result<Option<WifiManager<'static>>> {
    let ssid = env.get("WIFI_SSID").unwrap_or("");
    let pass = env.get("WIFI_PASSWORD").unwrap_or("");
    if ssid.is_empty() {
        set_wifi_status(false, "", "", -100);
        return Ok(None);
    }

    draw_loading_screen(
        fb,
        display,
        &format!("Connecting to '{}'...", ssid),
        Some("Please wait"),
    )?;

    match WifiManager::new(modem, sysloop, nvs) {
        Ok(mut wifi) => match wifi.connect(ssid, pass) {
            Ok(()) => {
                let s = wifi.status();
                let ip = s.ip.map(|i| i.to_string()).unwrap_or_default();
                set_wifi_status(true, ssid, &ip, s.rssi.unwrap_or(-100) as i32);
                info!("WiFi connected: {}", ip);
                draw_loading_screen(fb, display, "WiFi Connected!", Some(&ip))?;
                FreeRtos::delay_ms(500);
                Ok(Some(wifi))
            }
            Err(e) => {
                warn!("WiFi failed: {}", e);
                set_wifi_status(false, "", "", -100);
                draw_loading_screen(fb, display, "WiFi Failed", Some("Continuing..."))?;
                FreeRtos::delay_ms(1000);
                Ok(None)
            }
        },
        Err(e) => {
            warn!("WiFi init error: {}", e);
            set_wifi_status(false, "", "", -100);
            Ok(None)
        }
    }
}

fn draw_loading_screen(
    fb: &mut Framebuffer,
    display: &mut Display,
    msg: &str,
    sub: Option<&str>,
) -> Result<()> {
    let (cx, cy) = (DISPLAY_WIDTH as i32 / 2, DISPLAY_HEIGHT as i32 / 2);
    let mut ctx = DrawContext::new(fb);
    ctx.clear(Color::from_hex(ThemeColors::bg_primary()).unwrap_or(Color::WHITE));
    let title_font = TtfFont::ebgaramond(80);
    let title_w = ctx.text_ttf_width("Moondeck", &title_font);
    ctx.text_ttf(
        cx - title_w / 2,
        cy - 80,
        "Moondeck",
        Color::from_hex(ThemeColors::accent_primary()).unwrap_or(Color::WHITE),
        title_font,
    );
    ctx.text_ttf(
        cx - (msg.len() as i32 * 4),
        cy + 10,
        msg,
        Color::from_hex(ThemeColors::text_primary()).unwrap_or(Color::WHITE),
        TtfFont::inter(20),
    );
    if let Some(s) = sub {
        ctx.text_ttf(
            cx - (s.len() as i32 * 3),
            cy + 45,
            s,
            Color::from_hex(ThemeColors::text_muted()).unwrap_or(Color::WHITE),
            TtfFont::inter(16),
        );
    }
    ctx.fill_rect(
        cx - 100,
        cy + 80,
        200,
        3,
        Color::from_hex(ThemeColors::accent_primary()).unwrap_or(Color::WHITE),
    );
    display.flush(fb)
}

fn run_loop(
    lua: &mut LuaRuntime,
    pm: &mut PageManager,
    plugins: &mut Vec<(WidgetPlugin, WidgetInstance)>,
    display: &mut Display,
    touch: &mut TouchController,
    wifi: Option<WifiManager>,
    mut bg: Color,
    mut fb: Framebuffer,
    env: &EnvConfig,
    reload_gen: &AtomicU32,
) -> Result<()> {
    let mut timer = FrameTimer::new();
    let mut gestures = GestureProcessor::new();
    let mut last_status = 0u64;
    let mut last_page = pm.current_index();
    let mut last_reload = reload_gen.load(Ordering::SeqCst);

    loop {
        let now = now_ms();
        let delta = timer.tick(now);

        // Check for reload request
        let current_reload = reload_gen.load(Ordering::SeqCst);
        if current_reload != last_reload {
            last_reload = current_reload;
            info!("=== Reloading Lua config ===");
            draw_loading_screen(&mut fb, display, "Reloading...", None)?;
            match init_lua_and_widgets(env) {
                Ok((new_lua, new_pm, new_plugins, new_bg)) => {
                    *lua = new_lua;
                    *pm = new_pm;
                    *plugins = new_plugins;
                    bg = new_bg;
                    last_page = pm.current_index();
                    info!("=== Reload complete ===");
                }
                Err(e) => {
                    warn!("Reload failed, keeping old config: {}", e);
                    draw_loading_screen(&mut fb, display, "Reload Failed", Some(&e.to_string()))?;
                    FreeRtos::delay_ms(1000);
                }
            }
        }

        // Process touch events
        poll_touch(touch, &mut gestures, pm);

        // Update status every 5s
        if now - last_status >= 5000 {
            last_status = now;
            set_system_info(unsafe { esp_idf_sys::esp_get_free_heap_size() }, 240);
            if let Some(ref w) = wifi {
                let s = w.status();
                set_wifi_status(
                    s.connected,
                    s.ssid.as_deref().unwrap_or(""),
                    &s.ip.map(|i| i.to_string()).unwrap_or_default(),
                    s.rssi.unwrap_or(-100) as i32,
                );
            }
        }

        if pm.current_index() != last_page {
            info!("Page: {} -> {}", last_page + 1, pm.current_index() + 1);
            last_page = pm.current_index();
        }

        // Update widgets
        for (p, _) in plugins.iter() {
            poll_touch(touch, &mut gestures, pm);
            let _ = p.update(lua, delta);
        }
        poll_touch(touch, &mut gestures, pm);

        // Render
        {
            let mut ctx = DrawContext::new(&mut fb);
            ctx.clear(bg);
            if let Some(page) = pm.current_page() {
                let ui_color = Color::from_hex(ThemeColors::text_muted()).unwrap_or(Color::WHITE);
                ctx.text_ttf(
                    10,
                    DISPLAY_HEIGHT as i32 - 20,
                    &format!(
                        "Page {}/{}: {}",
                        pm.current_index() + 1,
                        pm.page_count(),
                        page.title
                    ),
                    ui_color,
                    TtfFont::inter(18),
                );
                ctx.text_ttf(
                    DISPLAY_WIDTH as i32 - 80,
                    DISPLAY_HEIGHT as i32 - 20,
                    &format!("FPS: {:.1}", timer.fps()),
                    ui_color,
                    TtfFont::inter(18),
                );

                for (plugin, widget) in plugins.iter() {
                    if page.widgets.iter().any(|w| {
                        w.module == widget.module
                            && w.context.x == widget.context.x
                            && w.context.y == widget.context.y
                    }) {
                        let _ = plugin.render(lua, &widget.context, &mut ctx);
                    }
                }
            }
        }
        display.flush(&fb)?;

        if timer.frame_count() % 300 == 0 {
            info!("Frame {}: FPS={:.1}", timer.frame_count(), timer.fps());
        }

        let elapsed = now_ms().saturating_sub(now);
        FreeRtos::delay_ms(if elapsed < 33 {
            (33 - elapsed) as u32
        } else {
            1
        });
    }
}

fn poll_touch(touch: &mut TouchController, gestures: &mut GestureProcessor, pm: &mut PageManager) {
    let start = now_ms();
    loop {
        let now = now_ms();
        match touch.poll() {
            Ok(Some(e)) => {
                if let Some(g) = gestures.process(e, now) {
                    if pm.handle_event(&Event::Gesture(g.clone())) {
                        match g {
                            Gesture::SwipeLeft | Gesture::SwipeRight => {
                                info!("Swipe -> page {}", pm.current_index() + 1)
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(None) if now - start > 50 => break,
            Ok(None) => FreeRtos::delay_ms(5),
            Err(_) => break,
        }
    }
}
