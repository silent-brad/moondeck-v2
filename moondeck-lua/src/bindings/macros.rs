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
/// Usage: set_theme_fields!(ctx, table, theme, field1, field2, ...);
#[macro_export]
macro_rules! set_theme_fields {
    ($ctx:expr, $table:expr, $theme:expr, $($field:ident),* $(,)?) => {
        $(
            $table.set($ctx, stringify!($field), $ctx.intern($theme.$field.as_bytes()))?;
        )*
    };
}

/// Macro for registering simple Lua getter callbacks that read from a static RwLock.
/// Reduces boilerplate for callbacks that just return a value.
///
/// Usage:
/// ```ignore
/// lua_getter!(table, ctx, "method_name", STATIC_VAR, |val| val as i64);
/// ```
#[macro_export]
macro_rules! lua_getter {
    ($table:expr, $ctx:expr, $name:expr, $static:expr, $transform:expr) => {
        $table.set(
            $ctx,
            $name,
            piccolo::Callback::from_fn(&$ctx, |ctx, _exec, mut stack| {
                let val = $static.read().unwrap();
                let result = $transform(&*val);
                stack.replace(ctx, result);
                Ok(piccolo::CallbackReturn::Return)
            }),
        )?
    };
}

/// Macro for registering Lua callbacks that return an interned string from a static RwLock<String>.
#[macro_export]
macro_rules! lua_getter_string {
    ($table:expr, $ctx:expr, $name:expr, $static:expr) => {
        $table.set(
            $ctx,
            $name,
            piccolo::Callback::from_fn(&$ctx, |ctx, _exec, mut stack| {
                let val = $static.read().unwrap().clone();
                stack.replace(ctx, ctx.intern(val.as_bytes()));
                Ok(piccolo::CallbackReturn::Return)
            }),
        )?
    };
    ($table:expr, $ctx:expr, $name:expr, $static:expr, $default:expr) => {
        $table.set(
            $ctx,
            $name,
            piccolo::Callback::from_fn(&$ctx, |ctx, _exec, mut stack| {
                let val = $static.read().unwrap().clone();
                if val.is_empty() {
                    stack.replace(ctx, ctx.intern($default));
                } else {
                    stack.replace(ctx, ctx.intern(val.as_bytes()));
                }
                Ok(piccolo::CallbackReturn::Return)
            }),
        )?
    };
}

/// Macro for registering a simple no-arg Lua callback that computes and returns a value.
/// The body expression should return something convertible to a Lua Value.
///
/// Usage:
/// ```ignore
/// lua_fn!(table, ctx, "method_name", {
///     let now = SystemTime::now();
///     now.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
/// });
/// ```
#[macro_export]
macro_rules! lua_fn {
    ($table:expr, $ctx:expr, $name:expr, $body:expr) => {
        $table.set(
            $ctx,
            $name,
            piccolo::Callback::from_fn(&$ctx, |ctx, _exec, mut stack| {
                let result = $body;
                stack.replace(ctx, result);
                Ok(piccolo::CallbackReturn::Return)
            }),
        )?
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

/// Macro for registering gfx draw commands that follow the standard pattern:
/// consume args, get offset, push DrawCommand, return nil.
///
/// Usage:
/// ```ignore
/// gfx_draw!(table, ctx, "fill_rect", (x, y, w, h, color) => |ox, oy| {
///     FillRect { x: i32(x) + ox, y: i32(y) + oy, w: u32(w), h: u32(h), color: color(color) }
/// });
/// ```
#[macro_export]
macro_rules! gfx_draw {
    ($table:expr, $ctx:expr, $name:expr, ($($arg:ident),+) => |$ox:ident, $oy:ident| $cmd:expr) => {
        $table.set(
            $ctx,
            $name,
            piccolo::Callback::from_fn(&$ctx, |ctx, _exec, mut stack| {
                #[allow(unused_parens)]
                let (_self, $($arg),+): (piccolo::Value, $(gfx_draw!(@type $arg)),+) =
                    stack.consume(ctx)?;
                let ($ox, $oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                #[allow(unused_variables)]
                let ($ox, $oy) = ($ox, $oy);
                DRAW_COMMANDS.with(|dc| dc.push($cmd));
                stack.replace(ctx, piccolo::Value::Nil);
                Ok(piccolo::CallbackReturn::Return)
            }),
        )?
    };
    // Type inference helper - all args are Value
    (@type $arg:ident) => { piccolo::Value };
}

/// Macro for running Lua executor with fuel and returning result
#[macro_export]
macro_rules! run_with_fuel {
    ($ctx:expr, $executor:expr, $fuel:expr) => {{
        let stashed = $ctx.stash($executor);
        let mut fuel = piccolo::Fuel::with($fuel);
        let exec = $ctx.fetch(&stashed);
        while !exec.step($ctx, &mut fuel) {
            if fuel.remaining() <= 0 {
                break;
            }
        }
        exec
    }};
}

// Re-exports for use outside this module (if needed)
#[allow(unused_imports)]
pub use {
    define_state, gfx_draw, lua_fn, lua_getter, lua_getter_string, run_with_fuel, set_theme_fields,
    theme_accessors,
};
