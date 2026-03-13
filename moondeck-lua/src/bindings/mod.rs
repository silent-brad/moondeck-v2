#[macro_use]
mod macros;
mod gfx;
pub mod lua_serde;
mod modules;
mod net;
mod system;

use anyhow::Result;
use moondeck_hal::EnvConfig;
use crate::vm::VmState;

pub use gfx::{get_draw_commands, get_draw_offset, register_gfx, set_draw_offset, DrawCommand, LuaDrawCommands};
pub use net::register_net;
pub use modules::{get_current_theme, get_default_theme, get_theme_bg_primary, register_modules, set_current_theme, ThemeAccessor};
pub use system::{init_boot_time, register_device, register_env, register_util, set_system_info, set_timezone_offset, set_wifi_status};

pub fn register_all(vm: &mut VmState, env_config: &EnvConfig) -> Result<()> {
    register_gfx(vm)?;
    register_device(vm)?;
    register_env(vm, env_config)?;
    register_net(vm)?;
    register_util(vm)?;
    register_modules(vm)?;
    Ok(())
}
