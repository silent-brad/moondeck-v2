use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Closure, Executor, Fuel, Lua, String as LuaString, Table, Value};
use std::sync::RwLock;

// Auto-generated theme definitions from config/theme.lua
include!(concat!(env!("OUT_DIR"), "/embedded_themes.rs"));

// Auto-generated embedded Lua module sources
include!(concat!(env!("OUT_DIR"), "/embedded_modules.rs"));

// Global current theme state (accessible from Rust)
static CURRENT_THEME: RwLock<String> = RwLock::new(String::new());

/// Get the current theme name
pub fn get_current_theme() -> String {
    CURRENT_THEME.read().unwrap().clone()
}

/// Get the default theme name (from config/theme.lua)
pub fn get_default_theme() -> &'static str {
    DEFAULT_THEME
}

/// Get the current theme's background color
pub fn get_theme_bg_primary() -> &'static str {
    let theme_name = CURRENT_THEME.read().unwrap();
    let theme = get_theme(&theme_name);
    theme.bg_primary
}

/// Get available theme names
#[allow(dead_code)]
pub fn get_theme_names() -> &'static [&'static str] {
    THEME_NAMES
}

/// Theme color accessors for Rust code (uses current theme or default if not set)
#[allow(dead_code)]
pub struct ThemeAccessors;

impl ThemeColors {
    fn get_current_theme_name() -> String {
        let theme_name = CURRENT_THEME.read().unwrap();
        if theme_name.is_empty() {
            DEFAULT_THEME.to_string()
        } else {
            theme_name.clone()
        }
    }

    pub fn bg_primary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).bg_primary
    }

    pub fn bg_secondary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).bg_secondary
    }

    pub fn bg_tertiary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).bg_tertiary
    }

    pub fn bg_card() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).bg_card
    }

    pub fn text_primary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).text_primary
    }

    pub fn text_secondary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).text_secondary
    }

    pub fn text_muted() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).text_muted
    }

    pub fn text_accent() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).text_accent
    }

    pub fn accent_primary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).accent_primary
    }

    pub fn accent_secondary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).accent_secondary
    }

    pub fn accent_success() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).accent_success
    }

    pub fn accent_warning() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).accent_warning
    }

    pub fn accent_error() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).accent_error
    }

    pub fn border_primary() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).border_primary
    }

    pub fn border_accent() -> &'static str {
        let name = Self::get_current_theme_name();
        get_theme(&name).border_accent
    }
}

/// Set the current theme by name (for early initialization before Lua runs)
pub fn set_current_theme(name: &str) {
    *CURRENT_THEME.write().unwrap() = name.to_string();
}

fn create_theme_colors_table<'gc>(ctx: piccolo::Context<'gc>, theme_name: &str) -> Result<Table<'gc>, piccolo::Error<'gc>> {
    let colors = Table::new(&ctx);
    let theme = get_theme(theme_name);

    colors.set(ctx, "name", ctx.intern(theme_name.as_bytes()))?;
    // Background colors
    colors.set(ctx, "bg_primary", ctx.intern(theme.bg_primary.as_bytes()))?;
    colors.set(ctx, "bg_secondary", ctx.intern(theme.bg_secondary.as_bytes()))?;
    colors.set(ctx, "bg_tertiary", ctx.intern(theme.bg_tertiary.as_bytes()))?;
    colors.set(ctx, "bg_card", ctx.intern(theme.bg_card.as_bytes()))?;
    // Text colors
    colors.set(ctx, "text_primary", ctx.intern(theme.text_primary.as_bytes()))?;
    colors.set(ctx, "text_secondary", ctx.intern(theme.text_secondary.as_bytes()))?;
    colors.set(ctx, "text_muted", ctx.intern(theme.text_muted.as_bytes()))?;
    colors.set(ctx, "text_accent", ctx.intern(theme.text_accent.as_bytes()))?;
    // Accent colors
    colors.set(ctx, "accent_primary", ctx.intern(theme.accent_primary.as_bytes()))?;
    colors.set(ctx, "accent_secondary", ctx.intern(theme.accent_secondary.as_bytes()))?;
    colors.set(ctx, "accent_success", ctx.intern(theme.accent_success.as_bytes()))?;
    colors.set(ctx, "accent_warning", ctx.intern(theme.accent_warning.as_bytes()))?;
    colors.set(ctx, "accent_error", ctx.intern(theme.accent_error.as_bytes()))?;
    // Border colors
    colors.set(ctx, "border_primary", ctx.intern(theme.border_primary.as_bytes()))?;
    colors.set(ctx, "border_accent", ctx.intern(theme.border_accent.as_bytes()))?;
    // Component specific
    colors.set(ctx, "card_radius", theme.card_radius)?;
    colors.set(ctx, "border_width", theme.border_width)?;

    Ok(colors)
}

fn get_lua_module_source(name: &str) -> Option<&'static str> {
    EMBEDDED_LUA_MODULES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, src)| *src)
}

fn load_lua_module<'gc>(ctx: piccolo::Context<'gc>, name: &str, source: &str) -> Result<Value<'gc>, piccolo::Error<'gc>> {
    let closure = match Closure::load(ctx, Some(name), source.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to compile Lua module '{}': {:?}", name, e);
            return Err(e.into());
        }
    };

    let executor = Executor::start(ctx, closure.into(), ());
    let stashed = ctx.stash(executor);

    let mut fuel = Fuel::with(500000);
    let exec = ctx.fetch(&stashed);
    while !exec.step(ctx, &mut fuel) {
        if fuel.remaining() <= 0 {
            log::error!("Module {} ran out of fuel during load (this should not happen)", name);
            break;
        }
    }

    let result = exec.take_result::<Value>(ctx)?;
    match result {
        Ok(v) => {
            log::debug!("Module '{}' loaded successfully, returned: {:?}", name, 
                match &v {
                    Value::Table(_) => "Table".to_string(),
                    Value::Function(_) => "Function".to_string(),
                    Value::Nil => "Nil".to_string(),
                    other => format!("{:?}", other),
                }
            );
            Ok(v)
        }
        Err(e) => {
            log::error!("Module '{}' execution failed: {:?}", name, e);
            Err(e)
        }
    }
}

pub fn register_modules(lua: &mut Lua) -> Result<()> {
    // Initialize global theme state with default from config/theme.lua
    {
        let mut theme = CURRENT_THEME.write().unwrap();
        if theme.is_empty() {
            *theme = DEFAULT_THEME.to_string();
        }
    }

    lua.try_enter(|ctx| {
        // First, set up the basic require function that will be used by embedded modules
        // We need a two-phase approach: first create placeholder, then load modules

        // Create a table to store loaded modules
        let loaded_modules = Table::new(&ctx);
        ctx.set_global("__loaded_modules", loaded_modules)?;

        // Create theme module (Rust implementation that syncs with Rust state)
        // This wraps the Lua theme module with Rust state synchronization
        let theme_table = Table::new(&ctx);

        theme_table.set(
            ctx,
            "set",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self_table, theme_name): (Value, LuaString) = stack.consume(ctx)?;
                let theme_str = theme_name.to_str().unwrap_or("dark").to_string();
                log::debug!("theme:set('{}') called", theme_str);
                // Update global theme state
                *CURRENT_THEME.write().unwrap() = theme_str;
                stack.replace(ctx, true);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // Create a get function that handles method call syntax: theme:get()
        // Piccolo passes self as first argument for method calls
        theme_table.set(
            ctx,
            "get",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                // Consume self argument if present (method call: theme:get())
                // Stack might be empty for direct function call
                if stack.len() > 0 {
                    let _self_table: Value = stack.consume(ctx)?;
                }
                
                let theme_name_str = CURRENT_THEME.read().unwrap().clone();
                let theme_name: &str = if theme_name_str.is_empty() { 
                    DEFAULT_THEME 
                } else { 
                    &theme_name_str
                };
                log::trace!("theme:get() returning colors for theme '{}'", theme_name);
                let colors = create_theme_colors_table(ctx, theme_name)?;
                stack.replace(ctx, colors);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // Also expose themes table for compatibility with Lua theme module
        let themes_table = Table::new(&ctx);
        for theme_name in THEME_NAMES {
            let theme_colors = create_theme_colors_table(ctx, theme_name)?;
            themes_table.set(ctx, ctx.intern(theme_name.as_bytes()), theme_colors)?;
        }
        theme_table.set(ctx, "themes", themes_table)?;

        // Set current theme name
        {
            let current = CURRENT_THEME.read().unwrap();
            theme_table.set(ctx, "current", ctx.intern(current.as_bytes()))?;
        }

        ctx.set_global("__theme_module", theme_table)?;

        // Create layout module (stub)
        let layout_table = Table::new(&ctx);
        layout_table.set(
            ctx,
            "grid",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                stack.replace(ctx, Table::new(&ctx));
                Ok(CallbackReturn::Return)
            }),
        )?;
        ctx.set_global("__layout_module", layout_table)?;

        // Load components.lua module
        log::info!("Available Lua modules: {:?}", EMBEDDED_LUA_MODULES.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        if let Some(components_src) = get_lua_module_source("components") {
            log::info!("Loading components.lua module ({} bytes)", components_src.len());

            // Temporary require for loading components - creates fresh theme table each time
            ctx.set_global(
                "require",
                Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                    let module_name: LuaString = stack.consume(ctx)?;
                    let module_str = module_name.to_str().unwrap_or("");

                    let result: Value = match module_str {
                        "theme" => {
                            log::debug!("require('theme') called during components load");
                            // Return the theme module table directly from global
                            ctx.globals().get(ctx, "__theme_module")
                        }
                        "layout" => ctx.globals().get(ctx, "__layout_module"),
                        _ => {
                            // Check if already loaded
                            if let Value::Table(loaded) = ctx.globals().get(ctx, "__loaded_modules") {
                                loaded.get(ctx, ctx.intern(module_str.as_bytes()))
                            } else {
                                Value::Nil
                            }
                        }
                    };

                    if matches!(result, Value::Nil) {
                        let msg = format!("module '{}' not found", module_str);
                        log::warn!("require failed: {}", msg);
                        let err_val: Value = ctx.intern(msg.as_bytes()).into();
                        return Err(piccolo::Error::from_value(err_val));
                    }

                    stack.replace(ctx, result);
                    Ok(CallbackReturn::Return)
                }),
            )?;

            match load_lua_module(ctx, "components", components_src) {
                Ok(components_module) => {
                    log::info!("Successfully loaded components.lua module");
                    ctx.set_global("__components_module", components_module)?;
                    // Also store in loaded modules
                    if let Value::Table(loaded) = ctx.globals().get(ctx, "__loaded_modules") {
                        let _ = loaded.set(ctx, "components", components_module);
                    }
                }
                Err(e) => {
                    log::error!("Failed to load components.lua module: {:?}", e);
                }
            }
        } else {
            log::warn!("components.lua not found in embedded modules");
        }

        // Create final require function
        ctx.set_global(
            "require",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let module_name: LuaString = stack.consume(ctx)?;
                let module_str = module_name.to_str().unwrap_or("");

                let result = match module_str {
                    "theme" => ctx.globals().get(ctx, "__theme_module"),
                    "layout" => ctx.globals().get(ctx, "__layout_module"),
                    "components" => ctx.globals().get(ctx, "__components_module"),
                    _ => {
                        // Check loaded modules
                        if let Value::Table(loaded) = ctx.globals().get(ctx, "__loaded_modules") {
                            loaded.get(ctx, ctx.intern(module_str.as_bytes()))
                        } else {
                            Value::Nil
                        }
                    }
                };

                if matches!(result, Value::Nil) {
                    let msg = format!("module '{}' not found", module_str);
                    log::warn!("require('{}') failed: module not found", module_str);
                    let err_val: Value = ctx.intern(msg.as_bytes()).into();
                    return Err(piccolo::Error::from_value(err_val));
                }

                stack.replace(ctx, result);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // Run a preflight check to verify stdlib is available
        let preflight_check = r#"
            return true, "ok"
        "#;

        match Closure::load(ctx, Some("preflight"), preflight_check.as_bytes()) {
            Ok(closure) => {
                let executor = Executor::start(ctx, closure.into(), ());
                let stashed = ctx.stash(executor);
                let mut fuel = Fuel::with(10000);
                let exec = ctx.fetch(&stashed);
                while !exec.step(ctx, &mut fuel) {
                    if fuel.remaining() <= 0 { break; }
                }
                match exec.take_result::<(bool, LuaString)>(ctx) {
                    Ok(Ok((true, _))) => {
                        log::info!("Lua stdlib preflight check passed");
                    }
                    Ok(Ok((false, missing))) => {
                        let missing_str = missing.to_str().unwrap_or("unknown");
                        log::error!("Lua stdlib preflight check FAILED, missing: {}", missing_str);
                    }
                    Ok(Err(e)) => {
                        log::error!("Preflight check execution error: {:?}", e);
                    }
                    Err(e) => {
                        log::error!("Preflight check result error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to compile preflight check: {:?}", e);
            }
        }

        log::info!("Lua modules registered successfully");
        Ok(())
    })?;

    Ok(())
}
