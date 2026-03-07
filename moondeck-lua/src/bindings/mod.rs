mod device;
mod env;
mod gfx;
mod modules;
mod net;

use anyhow::Result;
use moondeck_hal::EnvConfig;
use piccolo::Lua;

#[allow(unused_imports)]
pub use device::register_device;
#[allow(unused_imports)]
pub use env::register_env;
pub use gfx::{get_draw_commands, get_draw_offset, register_gfx, set_draw_offset, DrawCommand, LuaDrawCommands};
#[allow(unused_imports)]
pub use net::register_net;
pub use modules::register_modules;

pub fn register_all(lua: &mut Lua, env_config: &EnvConfig) -> Result<()> {
    register_gfx(lua)?;
    register_device(lua)?;
    register_env(lua, env_config)?;
    register_modules(lua)?;
    Ok(())
}
