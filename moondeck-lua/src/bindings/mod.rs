#[macro_use]
mod macros;
mod device;
mod env;
mod gfx;
pub mod lua_serde;
mod modules;
mod net;
mod util;

use anyhow::Result;
use moondeck_hal::EnvConfig;
use piccolo::Lua;

pub use device::{init_boot_time, register_device, set_system_info, set_timezone_offset, set_wifi_status};
pub use env::register_env;
pub use gfx::{get_draw_commands, get_draw_offset, register_gfx, set_draw_offset, DrawCommand, LuaDrawCommands};
pub use net::register_net;
pub use modules::{get_current_theme, get_default_theme, get_theme_bg_primary, register_modules, set_current_theme, ThemeAccessor};
pub use util::register_util;

pub fn register_all(lua: &mut Lua, env_config: &EnvConfig) -> Result<()> {
    register_gfx(lua)?;
    register_device(lua)?;
    register_env(lua, env_config)?;
    register_net(lua)?;
    register_util(lua)?;
    register_modules(lua)?;
    Ok(())
}
