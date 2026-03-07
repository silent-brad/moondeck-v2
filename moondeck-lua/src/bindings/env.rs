use anyhow::Result;
use moondeck_hal::EnvConfig;
use piccolo::{Callback, CallbackReturn, Lua, String as LuaString, Table, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub fn register_env(lua: &mut Lua, config: &EnvConfig) -> Result<()> {
    let vars: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(
        config.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    ));

    let vars_get = vars.clone();
    let vars_set = vars;

    lua.try_enter(|ctx| {
        let env_table = Table::new(&ctx);

        env_table.set(
            ctx,
            "get",
            Callback::from_fn(&ctx, move |ctx, _exec, mut stack| {
                let key: LuaString = stack.consume(ctx)?;
                let key_str = key.to_str().unwrap_or("");
                let vars = vars_get.lock().unwrap();
                if let Some(value) = vars.get(key_str) {
                    stack.replace(ctx, ctx.intern(value.as_bytes()));
                } else {
                    stack.replace(ctx, Value::Nil);
                }
                Ok(CallbackReturn::Return)
            }),
        )?;

        env_table.set(
            ctx,
            "set",
            Callback::from_fn(&ctx, move |ctx, _exec, mut stack| {
                let (key, value): (LuaString, LuaString) = stack.consume(ctx)?;
                let key_str = key.to_str().unwrap_or("").to_string();
                let value_str = value.to_str().unwrap_or("").to_string();
                vars_set.lock().unwrap().insert(key_str, value_str);
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("env", env_table)?;
        Ok(())
    })?;

    Ok(())
}
