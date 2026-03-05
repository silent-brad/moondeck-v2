mod device;
mod env;
mod gfx;
mod net;

use anyhow::Result;
use moondeck_hal::EnvConfig;
use piccolo::Lua;

#[allow(unused_imports)]
pub use device::register_device;
#[allow(unused_imports)]
pub use env::register_env;
pub use gfx::{register_gfx, DrawCommand, LuaDrawCommands};
#[allow(unused_imports)]
pub use net::register_net;

pub fn register_all(lua: &mut Lua, _env_config: &EnvConfig) -> Result<()> {
    register_gfx(lua)?;
    register_device(lua)?;
    Ok(())
}
