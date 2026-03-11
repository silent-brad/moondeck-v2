use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Closure, Executor, Lua, String as LuaString, Table, Value};
use std::sync::RwLock;

include!(concat!(env!("OUT_DIR"), "/embedded_themes.rs"));
include!(concat!(env!("OUT_DIR"), "/embedded_modules.rs"));

static CURRENT_THEME: RwLock<String> = RwLock::new(String::new());

pub fn get_current_theme() -> String { CURRENT_THEME.read().unwrap().clone() }
pub fn get_default_theme() -> &'static str { DEFAULT_THEME }
pub fn set_current_theme(name: &str) { *CURRENT_THEME.write().unwrap() = name.to_string(); }

pub fn get_theme_bg_primary() -> &'static str {
    get_theme(&CURRENT_THEME.read().unwrap()).bg_primary
}

/// Accessor struct for current theme colors (uses generated ThemeColors from build.rs)
pub struct ThemeAccessor;

impl ThemeAccessor {
    fn current() -> &'static ThemeColors {
        let name = CURRENT_THEME.read().unwrap();
        get_theme(if name.is_empty() { DEFAULT_THEME } else { &name })
    }

    pub fn bg_primary() -> &'static str { Self::current().bg_primary }
    pub fn bg_secondary() -> &'static str { Self::current().bg_secondary }
    pub fn bg_tertiary() -> &'static str { Self::current().bg_tertiary }
    pub fn bg_card() -> &'static str { Self::current().bg_card }
    pub fn text_primary() -> &'static str { Self::current().text_primary }
    pub fn text_secondary() -> &'static str { Self::current().text_secondary }
    pub fn text_muted() -> &'static str { Self::current().text_muted }
    pub fn text_accent() -> &'static str { Self::current().text_accent }
    pub fn accent_primary() -> &'static str { Self::current().accent_primary }
    pub fn accent_secondary() -> &'static str { Self::current().accent_secondary }
    pub fn accent_success() -> &'static str { Self::current().accent_success }
    pub fn accent_warning() -> &'static str { Self::current().accent_warning }
    pub fn accent_error() -> &'static str { Self::current().accent_error }
    pub fn border_primary() -> &'static str { Self::current().border_primary }
    pub fn border_accent() -> &'static str { Self::current().border_accent }
}

fn create_theme_colors_table<'gc>(ctx: piccolo::Context<'gc>, theme_name: &str) -> Result<Table<'gc>, piccolo::Error<'gc>> {
    let colors = Table::new(&ctx);
    let theme = get_theme(theme_name);

    colors.set(ctx, "name", ctx.intern(theme_name.as_bytes()))?;
    set_theme_fields!(ctx, colors, theme,
        bg_primary, bg_secondary, bg_tertiary, bg_card,
        text_primary, text_secondary, text_muted, text_accent,
        accent_primary, accent_secondary, accent_success, accent_warning, accent_error,
        border_primary, border_accent,
    );
    colors.set(ctx, "card_radius", theme.card_radius)?;
    colors.set(ctx, "border_width", theme.border_width)?;
    Ok(colors)
}

fn get_lua_module_source(name: &str) -> Option<&'static str> {
    EMBEDDED_LUA_MODULES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

fn load_lua_module<'gc>(ctx: piccolo::Context<'gc>, name: &str, source: &str) -> Result<Value<'gc>, piccolo::Error<'gc>> {
    let closure = Closure::load(ctx, Some(name), source.as_bytes())?;
    let exec = run_with_fuel!(ctx, Executor::start(ctx, closure.into(), ()), 500000);
    match exec.take_result::<Value>(ctx)? {
        Ok(v) => { log::debug!("Module '{}' loaded", name); Ok(v) }
        Err(e) => { log::error!("Module '{}' failed: {:?}", name, e); Err(e) }
    }
}

pub fn register_modules(lua: &mut Lua) -> Result<()> {
    // Initialize theme state
    {
        let mut theme = CURRENT_THEME.write().unwrap();
        if theme.is_empty() { *theme = DEFAULT_THEME.to_string(); }
    }

    lua.try_enter(|ctx| {
        ctx.set_global("__loaded_modules", Table::new(&ctx))?;

        // Create theme module
        let theme_table = Table::new(&ctx);
        theme_table.set(ctx, "set", Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
            let (_, name): (Value, LuaString) = stack.consume(ctx)?;
            *CURRENT_THEME.write().unwrap() = name.to_str().unwrap_or("dark").to_string();
            stack.replace(ctx, true);
            Ok(CallbackReturn::Return)
        }))?;

        theme_table.set(ctx, "get", Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
            if stack.len() > 0 { let _: Value = stack.consume(ctx)?; }
            let name = CURRENT_THEME.read().unwrap();
            let theme_name = if name.is_empty() { DEFAULT_THEME } else { &name };
            stack.replace(ctx, create_theme_colors_table(ctx, theme_name)?);
            Ok(CallbackReturn::Return)
        }))?;

        // Build themes lookup table
        let themes = Table::new(&ctx);
        for name in THEME_NAMES {
            themes.set(ctx, ctx.intern(name.as_bytes()), create_theme_colors_table(ctx, name)?)?;
        }
        theme_table.set(ctx, "themes", themes)?;
        theme_table.set(ctx, "current", ctx.intern(CURRENT_THEME.read().unwrap().as_bytes()))?;
        ctx.set_global("__theme_module", theme_table)?;

        // Layout stub
        let layout = Table::new(&ctx);
        layout.set(ctx, "grid", Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
            stack.replace(ctx, Table::new(&ctx));
            Ok(CallbackReturn::Return)
        }))?;
        ctx.set_global("__layout_module", layout)?;

        // Load components module
        if let Some(src) = get_lua_module_source("components") {
            log::info!("Loading components.lua ({} bytes)", src.len());
            setup_require(ctx, false)?;
            if let Ok(m) = load_lua_module(ctx, "components", src) {
                ctx.set_global("__components_module", m)?;
                if let Value::Table(loaded) = ctx.globals().get(ctx, "__loaded_modules") {
                    let _ = loaded.set(ctx, "components", m);
                }
            }
        }

        setup_require(ctx, true)?;
        log::info!("Lua modules registered");
        Ok(())
    })?;
    Ok(())
}

fn setup_require<'gc>(ctx: piccolo::Context<'gc>, final_version: bool) -> Result<(), piccolo::Error<'gc>> {
    ctx.set_global("require", Callback::from_fn(&ctx, move |ctx, _exec, mut stack| {
        let name: LuaString = stack.consume(ctx)?;
        let name_str = name.to_str().unwrap_or("");

        let result = match name_str {
            "theme" => ctx.globals().get(ctx, "__theme_module"),
            "layout" => ctx.globals().get(ctx, "__layout_module"),
            "components" if final_version => ctx.globals().get(ctx, "__components_module"),
            _ => {
                if let Value::Table(loaded) = ctx.globals().get(ctx, "__loaded_modules") {
                    loaded.get(ctx, ctx.intern(name_str.as_bytes()))
                } else { Value::Nil }
            }
        };

        if matches!(result, Value::Nil) {
            let msg = format!("module '{}' not found", name_str);
            return Err(piccolo::Error::from_value(ctx.intern(msg.as_bytes()).into()));
        }
        stack.replace(ctx, result);
        Ok(CallbackReturn::Return)
    }))?;
    Ok(())
}
