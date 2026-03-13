use crate::vm::value::{LuaString, LuaTable, SymbolTable, TableKey, Value};

/// Convert a serde_json::Value to a Lua Value
pub fn json_to_lua(symbols: &mut SymbolTable, value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            n.as_i64().map(Value::Int)
                .or_else(|| n.as_f64().map(Value::Num))
                .unwrap_or(Value::Nil)
        }
        serde_json::Value::String(s) => {
            let sym = symbols.intern(s);
            Value::Str(LuaString::Interned(sym))
        }
        serde_json::Value::Array(arr) => {
            let table = LuaTable::new();
            for (i, v) in arr.iter().enumerate() {
                table.set_int((i + 1) as i64, json_to_lua(symbols, v));
            }
            Value::Table(table)
        }
        serde_json::Value::Object(obj) => {
            let table = LuaTable::new();
            for (k, v) in obj.iter() {
                let sym = symbols.intern(k);
                table.set_sym(sym, json_to_lua(symbols, v));
            }
            Value::Table(table)
        }
    }
}

/// Convert a Lua Value to serde_json::Value
pub fn lua_to_json(symbols: &SymbolTable, value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::json!(i),
        Value::Num(n) => serde_json::json!(n),
        Value::Str(s) => serde_json::Value::String(s.as_str(symbols)),
        Value::Table(t) => table_to_json(t, symbols),
        _ => serde_json::Value::Null,
    }
}

/// Convert a Lua Table to serde_json::Value (array or object)
pub fn table_to_json(table: &LuaTable, symbols: &SymbolTable) -> serde_json::Value {
    // Check if it's an array (has integer keys starting from 1)
    let first = table.get_int(1);
    if !first.is_nil() {
        let mut arr = Vec::new();
        let mut idx = 1i64;
        loop {
            let v = table.get_int(idx);
            if v.is_nil() { break; }
            arr.push(lua_to_json(symbols, &v));
            idx += 1;
        }
        serde_json::Value::Array(arr)
    } else {
        let mut map = serde_json::Map::new();
        for (k, v) in table.iter() {
            let key_str = match &k {
                TableKey::Sym(sym) => symbols.resolve(*sym).to_string(),
                TableKey::Str(arc) => arc.as_ref().clone(),
                TableKey::Int(_) => continue,
            };
            map.insert(key_str, lua_to_json(symbols, &v));
        }
        serde_json::Value::Object(map)
    }
}

/// Parse Lua table of headers into HashMap
pub fn parse_headers(symbols: &SymbolTable, headers: &Value) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Value::Table(t) = headers {
        for (k, v) in t.iter() {
            let key_str = match &k {
                TableKey::Sym(sym) => symbols.resolve(*sym).to_string(),
                TableKey::Str(arc) => arc.as_ref().clone(),
                _ => continue,
            };
            let val_str = match &v {
                Value::Str(s) => s.as_str(symbols),
                _ => continue,
            };
            map.insert(key_str, val_str);
        }
    }
    map
}

/// Extract timeout from Value, defaulting to 10000ms
pub fn parse_timeout(val: &Value) -> u32 {
    match val {
        Value::Int(ms) => *ms as u32,
        Value::Num(ms) => *ms as u32,
        _ => 10000,
    }
}
