use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Lua, Table};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn register_device(lua: &mut Lua) -> Result<()> {
    lua.try_enter(|ctx| {
        let device_table = Table::new(&ctx);

        device_table.set(
            ctx,
            "seconds",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                stack.replace(ctx, secs);
                Ok(CallbackReturn::Return)
            }),
        )?;

        device_table.set(
            ctx,
            "millis",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                stack.replace(ctx, millis);
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("device", device_table)?;
        Ok(())
    })?;

    Ok(())
}
