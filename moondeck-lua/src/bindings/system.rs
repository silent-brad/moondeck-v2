use anyhow::Result;
use moondeck_hal::EnvConfig;
use crate::vm::{LuaString, LuaTable, Value, VmState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Global state using macro
define_state! {
    WIFI_CONNECTED: bool = false,
    WIFI_SSID: String = String::new(),
    WIFI_IP: String = String::new(),
    WIFI_RSSI: i32 = -100,
    FREE_HEAP: u32 = 0,
    CPU_FREQ: u32 = 240,
    BOOT_TIME: u64 = 0,
    TZ_OFFSET: i64 = 0,
}

// ============================================================================
// Device/System State Management
// ============================================================================

/// Set timezone offset in seconds from UTC
pub fn set_timezone_offset(offset_seconds: i64) {
    *TZ_OFFSET.write().unwrap() = offset_seconds;
}

/// Update WiFi state from the main application
pub fn set_wifi_status(connected: bool, ssid: &str, ip: &str, rssi: i32) {
    *WIFI_CONNECTED.write().unwrap() = connected;
    *WIFI_SSID.write().unwrap() = ssid.to_string();
    *WIFI_IP.write().unwrap() = ip.to_string();
    *WIFI_RSSI.write().unwrap() = rssi;
}

/// Update system info from the main application
pub fn set_system_info(free_heap: u32, cpu_freq: u32) {
    *FREE_HEAP.write().unwrap() = free_heap;
    *CPU_FREQ.write().unwrap() = cpu_freq;
}

/// Initialize boot time (call once at startup)
pub fn init_boot_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    *BOOT_TIME.write().unwrap() = now;
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// Device Registration (exposes `device` global)
// ============================================================================

pub fn register_device(vm: &mut VmState) -> Result<()> {
    // Initialize boot time on first registration
    {
        let boot = BOOT_TIME.read().unwrap();
        if *boot == 0 {
            drop(boot);
            init_boot_time();
        }
    }

    let device = LuaTable::new();

    // device.seconds() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Int(now_secs())])
    });
    let sym = vm.symbols.intern("seconds");
    device.set_sym(sym, Value::NativeFn(id));

    // device.millis() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Int(now_millis())])
    });
    let sym = vm.symbols.intern("millis");
    device.set_sym(sym, Value::NativeFn(id));

    // device.uptime() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        let boot_time = *BOOT_TIME.read().unwrap();
        Ok(vec![Value::Int((now_secs() as u64).saturating_sub(boot_time) as i64)])
    });
    let sym = vm.symbols.intern("uptime");
    device.set_sym(sym, Value::NativeFn(id));

    // device.wifi_connected() -> bool
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Bool(*WIFI_CONNECTED.read().unwrap())])
    });
    let sym = vm.symbols.intern("wifi_connected");
    device.set_sym(sym, Value::NativeFn(id));

    // device.wifi_ssid() -> string
    let id = vm.register_native_id(|_vm, _args| {
        let ssid = WIFI_SSID.read().unwrap().clone();
        Ok(vec![Value::Str(LuaString::Heap(Arc::new(ssid)))])
    });
    let sym = vm.symbols.intern("wifi_ssid");
    device.set_sym(sym, Value::NativeFn(id));

    // device.ip_address() -> string
    let id = vm.register_native_id(|_vm, _args| {
        let ip = WIFI_IP.read().unwrap().clone();
        let result = if ip.is_empty() { "Not connected".to_string() } else { ip };
        Ok(vec![Value::Str(LuaString::Heap(Arc::new(result)))])
    });
    let sym = vm.symbols.intern("ip_address");
    device.set_sym(sym, Value::NativeFn(id));

    // device.wifi_rssi() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Int(*WIFI_RSSI.read().unwrap() as i64)])
    });
    let sym = vm.symbols.intern("wifi_rssi");
    device.set_sym(sym, Value::NativeFn(id));

    // device.free_heap() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Int(*FREE_HEAP.read().unwrap() as i64)])
    });
    let sym = vm.symbols.intern("free_heap");
    device.set_sym(sym, Value::NativeFn(id));

    // device.cpu_freq() -> integer
    let id = vm.register_native_id(|_vm, _args| {
        Ok(vec![Value::Int(*CPU_FREQ.read().unwrap() as i64)])
    });
    let sym = vm.symbols.intern("cpu_freq");
    device.set_sym(sym, Value::NativeFn(id));

    // device.localtime() -> table { hour, min, sec, year, month, day, weekday, yearday }
    let id = vm.register_native_id(|vm, _args| {
        let unix_secs = now_secs();
        let tz_offset = *TZ_OFFSET.read().unwrap();
        let local_secs = unix_secs + tz_offset;

        let (year, month, day, yearday) = unix_to_date(local_secs);
        let weekday = weekday_from_unix(local_secs);

        let day_secs = local_secs.rem_euclid(86400);
        let hour = (day_secs / 3600) as i64;
        let min = ((day_secs % 3600) / 60) as i64;
        let sec = (day_secs % 60) as i64;

        let result = LuaTable::new();
        let s = |vm: &mut VmState, name: &str| vm.symbols.intern(name);
        result.set_sym(s(vm, "hour"), Value::Int(hour));
        result.set_sym(s(vm, "min"), Value::Int(min));
        result.set_sym(s(vm, "sec"), Value::Int(sec));
        result.set_sym(s(vm, "year"), Value::Int(year));
        result.set_sym(s(vm, "month"), Value::Int(month));
        result.set_sym(s(vm, "day"), Value::Int(day));
        result.set_sym(s(vm, "weekday"), Value::Int(weekday));
        result.set_sym(s(vm, "yearday"), Value::Int(yearday));

        Ok(vec![Value::Table(result)])
    });
    let sym = vm.symbols.intern("localtime");
    device.set_sym(sym, Value::NativeFn(id));

    // device.set_timezone(offset_hours)
    let id = vm.register_native_id(|_vm, args| {
        // args[0] may be self (table), args[1] is the offset
        let offset_val = if args.len() > 1 {
            match &args[0] {
                Value::Table(_) => &args[1],
                _ => &args[0],
            }
        } else {
            args.first().unwrap_or(&Value::Nil)
        };
        let offset = match offset_val {
            Value::Int(h) => h * 3600,
            Value::Num(h) => (*h * 3600.0) as i64,
            _ => 0,
        };
        *TZ_OFFSET.write().unwrap() = offset;
        Ok(vec![])
    });
    let sym = vm.symbols.intern("set_timezone");
    device.set_sym(sym, Value::NativeFn(id));

    vm.set_global("device", Value::Table(device));
    Ok(())
}

// ============================================================================
// Date/Time Helpers
// ============================================================================

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    const DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if month == 2 && is_leap_year(year) { 29 } else { DAYS[(month - 1) as usize] }
}

fn unix_to_date(local_secs: i64) -> (i64, i64, i64, i64) {
    let total_days = local_secs.div_euclid(86400);
    let mut year = 1970i64;
    let mut remaining = total_days;

    while remaining < 0 {
        year -= 1;
        remaining += if is_leap_year(year) { 366 } else { 365 };
    }

    loop {
        let days_this_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_this_year {
            break;
        }
        remaining -= days_this_year;
        year += 1;
    }

    let yearday = remaining + 1;
    let mut month = 1i64;
    let mut day_of_month = remaining;

    while month <= 12 {
        let dim = days_in_month(year, month);
        if day_of_month < dim {
            break;
        }
        day_of_month -= dim;
        month += 1;
    }

    (year, month, day_of_month + 1, yearday)
}

fn weekday_from_unix(local_secs: i64) -> i64 {
    let days = local_secs.div_euclid(86400);
    ((days + 4).rem_euclid(7) + 1) as i64
}

// ============================================================================
// Env Registration (exposes `env` global)
// ============================================================================

pub fn register_env(vm: &mut VmState, config: &EnvConfig) -> Result<()> {
    let vars: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(
        config.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    ));

    let vars_get = vars.clone();
    let vars_set = vars;

    let env_table = LuaTable::new();

    let id = vm.register_native_id(move |vm, args| {
        let key = match args.first() {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => return Ok(vec![Value::Nil]),
        };
        let vars = vars_get.lock().unwrap();
        if let Some(value) = vars.get(&key) {
            let sym = vm.symbols.intern(value);
            Ok(vec![Value::Str(LuaString::Interned(sym))])
        } else {
            Ok(vec![Value::Nil])
        }
    });
    let sym = vm.symbols.intern("get");
    env_table.set_sym(sym, Value::NativeFn(id));

    let id = vm.register_native_id(move |vm, args| {
        let key = match args.get(0) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => return Ok(vec![Value::Nil]),
        };
        let value = match args.get(1) {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => return Ok(vec![Value::Nil]),
        };
        vars_set.lock().unwrap().insert(key, value);
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("set");
    env_table.set_sym(sym, Value::NativeFn(id));

    vm.set_global("env", Value::Table(env_table));
    Ok(())
}

// ============================================================================
// Util Registration (exposes `util` global)
// ============================================================================

pub fn register_util(vm: &mut VmState) -> Result<()> {
    let util = LuaTable::new();

    // util.word_wrap(text, max_chars) -> table of lines
    let id = vm.register_native_id(|vm, args| {
        // args[0] may be self (table), then text, max_chars
        let (text_val, max_val) = if args.len() > 2 {
            match &args[0] {
                Value::Table(_) => (&args[1], &args[2]),
                _ => (&args[0], &args[1]),
            }
        } else if args.len() == 2 {
            (&args[0], &args[1])
        } else {
            return Ok(vec![Value::Table(LuaTable::new())]);
        };

        let text_str = match text_val {
            Value::Str(s) => s.as_str(&vm.symbols),
            _ => String::new(),
        };
        let max_chars = match max_val {
            Value::Int(n) => (*n).max(1) as usize,
            Value::Num(n) => (*n as i64).max(1) as usize,
            _ => 80,
        };

        let result = LuaTable::new();
        for (i, line) in word_wrap(&text_str, max_chars).iter().enumerate() {
            let sym = vm.symbols.intern(line);
            result.set_int((i + 1) as i64, Value::Str(LuaString::Interned(sym)));
        }
        Ok(vec![Value::Table(result)])
    });
    let sym = vm.symbols.intern("word_wrap");
    util.set_sym(sym, Value::NativeFn(id));

    // util.format(fmt, ...) -> string
    let id = vm.register_native_id(|vm, args| {
        // args[0] may be self (table), then fmt, arg1, arg2, ...
        let (fmt_val, format_args) = if !args.is_empty() && matches!(&args[0], Value::Table(_)) {
            (args.get(1), &args[2..])
        } else {
            (args.first(), if args.len() > 1 { &args[1..] } else { &[] })
        };

        let fmt_str = match fmt_val {
            Some(Value::Str(s)) => s.as_str(&vm.symbols),
            _ => String::new(),
        };

        let result = format_string(&fmt_str, format_args, vm);
        let sym = vm.symbols.intern(&result);
        Ok(vec![Value::Str(LuaString::Interned(sym))])
    });
    let sym = vm.symbols.intern("format");
    util.set_sym(sym, Value::NativeFn(id));

    vm.set_global("util", Value::Table(util));
    Ok(())
}

// ============================================================================
// Util Helpers
// ============================================================================

fn word_wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        if line.is_empty() {
            if word.len() > max {
                for chunk in word.as_bytes().chunks(max) {
                    lines.push(String::from_utf8_lossy(chunk).into());
                }
            } else {
                line = word.into();
            }
        } else if line.len() + 1 + word.len() <= max {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(line);
            line = if word.len() > max {
                for chunk in word.as_bytes().chunks(max) {
                    lines.push(String::from_utf8_lossy(chunk).into());
                }
                String::new()
            } else {
                word.into()
            };
        }
    }
    if !line.is_empty() { lines.push(line); }
    lines
}

fn format_string(fmt: &str, args: &[Value], vm: &VmState) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' { result.push(c); continue; }
        match chars.peek() {
            Some('%') => { result.push('%'); chars.next(); }
            Some(_) => {
                let mut spec = String::from("%");
                while chars.peek().map_or(false, |c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+') {
                    spec.push(chars.next().unwrap());
                }
                if let Some(t) = chars.next() {
                    spec.push(t);
                    if arg_idx < args.len() {
                        result.push_str(&format_value(&spec, &args[arg_idx], vm));
                        arg_idx += 1;
                    }
                }
            }
            None => result.push('%'),
        }
    }
    result
}

fn format_value(spec: &str, value: &Value, vm: &VmState) -> String {
    let num = match value {
        Value::Int(i) => *i as f64,
        Value::Num(f) => *f,
        Value::Str(s) if spec.ends_with('s') => return s.as_str(&vm.symbols),
        Value::Str(s) => s.as_str(&vm.symbols).parse().unwrap_or(0.0),
        Value::Nil => return String::new(),
        _ => 0.0,
    };

    let inner = &spec[1..spec.len() - 1];
    let zero_pad = inner.starts_with('0');
    let width: usize = inner.trim_start_matches(|c: char| !c.is_ascii_digit()).split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0);

    match spec.chars().last() {
        Some('f') => inner.find('.').and_then(|p| inner[p + 1..].parse().ok())
            .map(|prec| format!("{:.prec$}", num, prec = prec))
            .unwrap_or_else(|| format!("{:.2}", num)),
        Some('d') => {
            let i = num as i64;
            if zero_pad && width > 0 { format!("{:0>w$}", i, w = width) }
            else if width > 0 { format!("{:>w$}", i, w = width) }
            else { i.to_string() }
        }
        Some('s') => vm.value_to_string(value),
        _ => num.to_string(),
    }
}
