use super::lua_serde::{json_to_lua, lua_to_json, parse_headers, parse_timeout};
use anyhow::Result;
use crate::vm::{LuaString, LuaTable, Value, VmState};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register_net(vm: &mut VmState) -> Result<()> {
    let net = LuaTable::new();

    // net.http_get(url, headers?, timeout_ms?) -> { ok, body, error?, status }
    let id = vm.register_native_id(|vm, args| {
        let url = match args.get(0) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => String::new(),
        };
        let headers_val = args.get(1).cloned().unwrap_or(Value::Nil);
        let timeout_val = args.get(2).cloned().unwrap_or(Value::Nil);
        let hdrs = parse_headers(&vm.symbols, &headers_val);
        let timeout = parse_timeout(&timeout_val);
        let result = do_http_get(&url, &hdrs, timeout);
        let response = make_response(vm, result);
        Ok(vec![Value::Table(response)])
    });
    let sym = vm.symbols.intern("http_get");
    net.set_sym(sym, Value::NativeFn(id));

    // net.http_post(url, body, content_type?, headers?, timeout_ms?) -> { ok, body, error?, status }
    let id = vm.register_native_id(|vm, args| {
        let url = match args.get(0) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => String::new(),
        };
        let body = match args.get(1) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => String::new(),
        };
        let content_type = match args.get(2) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => "application/json".to_string(),
        };
        let headers_val = args.get(3).cloned().unwrap_or(Value::Nil);
        let timeout_val = args.get(4).cloned().unwrap_or(Value::Nil);
        let mut header_map = parse_headers(&vm.symbols, &headers_val);
        header_map.insert("Content-Type".into(), content_type);
        let result = do_http_post(&url, &body, &header_map, parse_timeout(&timeout_val));
        let response = make_response(vm, result);
        Ok(vec![Value::Table(response)])
    });
    let sym = vm.symbols.intern("http_post");
    net.set_sym(sym, Value::NativeFn(id));

    // net.json_decode(json_string) -> table or nil
    let id = vm.register_native_id(|vm, args| {
        let json_str = match args.get(0) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => String::new(),
        };
        let result = serde_json::from_str(&json_str)
            .map(|v| json_to_lua(&mut vm.symbols, &v))
            .unwrap_or(Value::Nil);
        Ok(vec![result])
    });
    let sym = vm.symbols.intern("json_decode");
    net.set_sym(sym, Value::NativeFn(id));

    // net.json_encode(table) -> string or nil
    let id = vm.register_native_id(|vm, args| {
        let value = args.get(0).cloned().unwrap_or(Value::Nil);
        let result = serde_json::to_string(&lua_to_json(&vm.symbols, &value))
            .map(|s| Value::Str(LuaString::Heap(Arc::new(s))))
            .unwrap_or(Value::Nil);
        Ok(vec![result])
    });
    let sym = vm.symbols.intern("json_encode");
    net.set_sym(sym, Value::NativeFn(id));

    vm.set_global("net", Value::Table(net));
    Ok(())
}

fn make_response(vm: &mut VmState, result: Result<(u16, String), String>) -> LuaTable {
    let response = LuaTable::new();
    match result {
        Ok((status, body)) => {
            let ok_sym = vm.symbols.intern("ok");
            response.set_sym(ok_sym, Value::Bool((200..300).contains(&status)));
            let status_sym = vm.symbols.intern("status");
            response.set_sym(status_sym, Value::Int(status as i64));
            let body_sym = vm.symbols.intern("body");
            response.set_sym(body_sym, Value::Str(LuaString::Heap(Arc::new(body))));
        }
        Err(e) => {
            let ok_sym = vm.symbols.intern("ok");
            response.set_sym(ok_sym, Value::Bool(false));
            let error_sym = vm.symbols.intern("error");
            response.set_sym(error_sym, Value::Str(LuaString::Heap(Arc::new(e))));
            let body_sym = vm.symbols.intern("body");
            response.set_sym(body_sym, Value::Str(LuaString::Heap(Arc::new(String::new()))));
        }
    }
    response
}

#[cfg(feature = "esp")]
fn do_http_get(url: &str, headers: &HashMap<String, String>, timeout_ms: u32) -> Result<(u16, String), String> {
    use moondeck_hal::HttpClient;
    log::info!("HTTP GET: {}", url);
    let client = HttpClient::with_timeout(timeout_ms);
    let pairs: Vec<_> = headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    client.get_with_headers(url, &pairs)
        .map(|r| { log::info!("HTTP {}: {} bytes", r.status, r.body.len()); (r.status, r.body) })
        .map_err(|e| { log::error!("HTTP error: {:?}", e); format!("{}", e) })
}

#[cfg(feature = "esp")]
fn do_http_post(url: &str, body: &str, headers: &HashMap<String, String>, timeout_ms: u32) -> Result<(u16, String), String> {
    use moondeck_hal::HttpClient;
    let client = HttpClient::with_timeout(timeout_ms);
    let content_type = headers.get("Content-Type").map(|s| s.as_str()).unwrap_or("application/json");
    client.post(url, body, content_type)
        .map(|r| (r.status, r.body))
        .map_err(|e| format!("{}", e))
}

#[cfg(not(feature = "esp"))]
fn do_http_get(_: &str, _: &HashMap<String, String>, _: u32) -> Result<(u16, String), String> {
    Err("HTTP not available in this build".into())
}

#[cfg(not(feature = "esp"))]
fn do_http_post(_: &str, _: &str, _: &HashMap<String, String>, _: u32) -> Result<(u16, String), String> {
    Err("HTTP not available in this build".into())
}
