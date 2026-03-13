/// Macro for generating theme color accessor methods.
/// Each method reads the current theme and returns the specified field.
#[macro_export]
macro_rules! theme_accessors {
    ($($method:ident),* $(,)?) => {
        $(
            pub fn $method() -> &'static str {
                let name = Self::get_current_theme_name();
                get_theme(&name).$method
            }
        )*
    };
}

/// Macro for setting multiple fields on a Lua table from a struct.
/// Usage: set_theme_fields!(symbols, table, theme, field1, field2, ...);
#[macro_export]
macro_rules! set_theme_fields {
    ($symbols:expr, $table:expr, $theme:expr, $($field:ident),* $(,)?) => {
        $(
            let sym = $symbols.intern(stringify!($field));
            $table.set_sym(sym, Value::Str(LuaString::Interned($symbols.intern($theme.$field))));
        )*
    };
}

/// Macro for defining a set of RwLock statics with default values.
///
/// Usage:
/// ```ignore
/// define_state! {
///     WIFI_CONNECTED: bool = false,
///     WIFI_SSID: String = String::new(),
///     WIFI_RSSI: i32 = -100,
/// }
/// ```
#[macro_export]
macro_rules! define_state {
    ($($name:ident : $ty:ty = $default:expr),* $(,)?) => {
        $(
            static $name: std::sync::RwLock<$ty> = std::sync::RwLock::new($default);
        )*
    };
}

// Re-exports for use outside this module (if needed)
#[allow(unused_imports)]
pub use {define_state, theme_accessors, set_theme_fields};
