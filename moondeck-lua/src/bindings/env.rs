use anyhow::Result;
use moondeck_hal::EnvConfig;
use piccolo::Lua;

pub fn register_env(_lua: &mut Lua, _config: &EnvConfig) -> Result<()> {
    Ok(())
}
