use anyhow::Result;
use crate::vm::{Fuel, LuaString, LuaTable, Value, VmState};
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

fn create_theme_colors_table(vm: &mut VmState, theme_name: &str) -> LuaTable {
    let colors = LuaTable::new();
    let theme = get_theme(theme_name);

    let sym = vm.symbols.intern("name");
    colors.set_sym(sym, Value::Str(LuaString::Interned(vm.symbols.intern(theme_name))));

    set_theme_fields!(vm.symbols, colors, theme,
        bg_primary, bg_secondary, bg_tertiary, bg_card,
        text_primary, text_secondary, text_muted, text_accent,
        accent_primary, accent_secondary, accent_success, accent_warning, accent_error,
        border_primary, border_accent,
    );

    let sym = vm.symbols.intern("card_radius");
    colors.set_sym(sym, Value::Int(theme.card_radius as i64));
    let sym = vm.symbols.intern("border_width");
    colors.set_sym(sym, Value::Int(theme.border_width as i64));

    colors
}

fn get_lua_module_source(name: &str) -> Option<&'static str> {
    EMBEDDED_LUA_MODULES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

pub fn register_modules(vm: &mut VmState) -> Result<()> {
    // Initialize theme state
    {
        let mut theme = CURRENT_THEME.write().unwrap();
        if theme.is_empty() { *theme = DEFAULT_THEME.to_string(); }
    }

    // Create __loaded_modules table
    vm.set_global("__loaded_modules", Value::Table(LuaTable::new()));

    // Create theme module
    let theme_table = LuaTable::new();

    // theme.set(name)
    let id = vm.register_native_id(|vm, args| {
        let name = match args.get(1).or_else(|| args.first()) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => "dark".to_string(),
        };
        *CURRENT_THEME.write().unwrap() = name;
        Ok(vec![Value::Bool(true)])
    });
    let sym = vm.symbols.intern("set");
    theme_table.set_sym(sym, Value::NativeFn(id));

    // theme.get()
    let id = vm.register_native_id(|vm, _args| {
        let name = CURRENT_THEME.read().unwrap();
        let theme_name = if name.is_empty() { DEFAULT_THEME.to_string() } else { name.clone() };
        drop(name);
        let t = create_theme_colors_table(vm, &theme_name);
        Ok(vec![Value::Table(t)])
    });
    let sym = vm.symbols.intern("get");
    theme_table.set_sym(sym, Value::NativeFn(id));

    // Build themes lookup table
    let themes = LuaTable::new();
    for name in THEME_NAMES {
        let t = create_theme_colors_table(vm, name);
        let sym = vm.symbols.intern(name);
        themes.set_sym(sym, Value::Table(t));
    }
    let sym = vm.symbols.intern("themes");
    theme_table.set_sym(sym, Value::Table(themes));

    let current_name = CURRENT_THEME.read().unwrap().clone();
    let sym = vm.symbols.intern("current");
    theme_table.set_sym(sym, Value::Str(LuaString::Interned(vm.symbols.intern(&current_name))));

    vm.set_global("__theme_module", Value::Table(theme_table));

    // Layout stub
    let layout = LuaTable::new();
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Table(LuaTable::new())])
    });
    let sym = vm.symbols.intern("grid");
    layout.set_sym(sym, Value::NativeFn(id));
    vm.set_global("__layout_module", Value::Table(layout));

    // Load components module
    if let Some(src) = get_lua_module_source("components") {
        log::info!("Loading components.lua ({} bytes)", src.len());
        setup_require(vm, false);
        let mut fuel = Fuel::with(500000);
        match vm.exec_string(Some("components"), src, &mut fuel) {
            Ok(results) => {
                if let Some(module_val) = results.into_iter().next() {
                    log::debug!("Module 'components' loaded");
                    vm.set_global("__components_module", module_val.clone());
                    // Also store in __loaded_modules
                    if let Value::Table(loaded) = vm.get_global("__loaded_modules") {
                        let sym = vm.symbols.intern("components");
                        loaded.set_sym(sym, module_val);
                    }
                }
            }
            Err(e) => {
                log::error!("Module 'components' failed: {:?}", e);
            }
        }
    }

    setup_require(vm, true);
    log::info!("Lua modules registered");
    Ok(())
}

fn setup_require(vm: &mut VmState, final_version: bool) {
    vm.register_native("require", move |vm, args| {
        let name = match args.first() {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => return Err(crate::vm::VmError::Runtime("require: expected string argument".into())),
        };

        let result = match name.as_str() {
            "theme" => vm.get_global("__theme_module"),
            "layout" => vm.get_global("__layout_module"),
            "components" if final_version => vm.get_global("__components_module"),
            other => {
                if let Value::Table(loaded) = vm.get_global("__loaded_modules") {
                    loaded.get_str(other, &vm.symbols)
                } else {
                    Value::Nil
                }
            }
        };

        if result.is_nil() {
            return Err(crate::vm::VmError::Runtime(format!("module '{}' not found", name)));
        }
        Ok(vec![result])
    });
}
