use piccolo::{Context, Table, Value};

/// Convert a serde_json::Value to a Lua Value
pub fn json_to_lua<'gc>(ctx: Context<'gc>, value: &serde_json::Value) -> Value<'gc> {
    match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(Value::Integer)
            .or_else(|| n.as_f64().map(Value::Number))
            .unwrap_or(Value::Nil),
        serde_json::Value::String(s) => Value::String(ctx.intern(s.as_bytes())),
        serde_json::Value::Array(arr) => {
            let table = Table::new(&ctx);
            for (i, v) in arr.iter().enumerate() {
                let _ = table.set(ctx, (i + 1) as i64, json_to_lua(ctx, v));
            }
            Value::Table(table)
        }
        serde_json::Value::Object(obj) => {
            let table = Table::new(&ctx);
            for (k, v) in obj.iter() {
                let _ = table.set(ctx, ctx.intern(k.as_bytes()), json_to_lua(ctx, v));
            }
            Value::Table(table)
        }
    }
}

/// Convert a Lua Value to serde_json::Value
pub fn lua_to_json<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(b),
        Value::Integer(i) => serde_json::json!(i),
        Value::Number(n) => serde_json::json!(n),
        Value::String(s) => serde_json::Value::String(s.to_str().unwrap_or("").to_string()),
        Value::Table(t) => table_to_json(ctx, t),
        _ => serde_json::Value::Null,
    }
}

/// Convert a Lua Table to serde_json::Value (array or object)
pub fn table_to_json<'gc>(ctx: Context<'gc>, table: Table<'gc>) -> serde_json::Value {
    // Check if it's an array (has integer keys starting from 1)
    if !matches!(table.get_value(Value::Integer(1)), Value::Nil) {
        let mut arr = Vec::new();
        let mut idx = 1i64;
        loop {
            let v = table.get_value(Value::Integer(idx));
            if matches!(v, Value::Nil) {
                break;
            }
            arr.push(lua_to_json(ctx, v));
            idx += 1;
        }
        serde_json::Value::Array(arr)
    } else {
        let mut map = serde_json::Map::new();
        for (k, v) in table.iter() {
            if let Value::String(ks) = k {
                if let Ok(key_str) = ks.to_str() {
                    map.insert(key_str.to_string(), lua_to_json(ctx, v));
                }
            }
        }
        serde_json::Value::Object(map)
    }
}

/// Parse Lua table of headers into HashMap
pub fn parse_headers<'gc>(headers: Value<'gc>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Value::Table(t) = headers {
        for (k, v) in t.iter() {
            if let (Value::String(key), Value::String(val)) = (k, v) {
                if let (Ok(k), Ok(v)) = (key.to_str(), val.to_str()) {
                    map.insert(k.to_string(), v.to_string());
                }
            }
        }
    }
    map
}

/// Extract timeout from Value, defaulting to 10000ms
pub fn parse_timeout(val: Value) -> u32 {
    match val {
        Value::Integer(ms) => ms as u32,
        Value::Number(ms) => ms as u32,
        _ => 10000,
    }
}
