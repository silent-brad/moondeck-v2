use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Lua, Table, Value};
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

pub fn register_device(lua: &mut Lua) -> Result<()> {
    // Initialize boot time on first registration
    {
        let boot = BOOT_TIME.read().unwrap();
        if *boot == 0 {
            drop(boot);
            init_boot_time();
        }
    }

    lua.try_enter(|ctx| {
        let device_table = Table::new(&ctx);

        // Time functions
        lua_fn!(device_table, ctx, "seconds", now_secs());
        lua_fn!(device_table, ctx, "millis", now_millis());
        lua_fn!(device_table, ctx, "uptime", {
            let boot_time = *BOOT_TIME.read().unwrap();
            (now_secs() as u64).saturating_sub(boot_time) as i64
        });

        // WiFi status getters
        lua_getter!(device_table, ctx, "wifi_connected", WIFI_CONNECTED, |v: &bool| *v);
        lua_getter_string!(device_table, ctx, "wifi_ssid", WIFI_SSID);
        lua_getter_string!(device_table, ctx, "ip_address", WIFI_IP, b"Not connected");
        lua_getter!(device_table, ctx, "wifi_rssi", WIFI_RSSI, |v: &i32| *v as i64);

        // System info getters
        lua_getter!(device_table, ctx, "free_heap", FREE_HEAP, |v: &u32| *v as i64);
        lua_getter!(device_table, ctx, "cpu_freq", CPU_FREQ, |v: &u32| *v as i64);

        // device.localtime() -> table { hour, min, sec, year, month, day, weekday, yearday }
        device_table.set(
            ctx,
            "localtime",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let unix_secs = now_secs();
                let tz_offset = *TZ_OFFSET.read().unwrap();
                let local_secs = unix_secs + tz_offset;

                let (year, month, day, yearday) = unix_to_date(local_secs);
                let weekday = weekday_from_unix(local_secs);

                let day_secs = local_secs.rem_euclid(86400);
                let hour = (day_secs / 3600) as i64;
                let min = ((day_secs % 3600) / 60) as i64;
                let sec = (day_secs % 60) as i64;

                let result = Table::new(&ctx);
                result.set(ctx, "hour", hour)?;
                result.set(ctx, "min", min)?;
                result.set(ctx, "sec", sec)?;
                result.set(ctx, "year", year)?;
                result.set(ctx, "month", month)?;
                result.set(ctx, "day", day)?;
                result.set(ctx, "weekday", weekday)?;
                result.set(ctx, "yearday", yearday)?;

                stack.replace(ctx, result);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.set_timezone(offset_hours)
        device_table.set(
            ctx,
            "set_timezone",
            Callback::from_fn(&ctx, |_ctx, _exec, mut stack| {
                let (arg1, arg2): (Value, Value) = stack.consume(_ctx)?;
                let offset_hours = match arg1 {
                    Value::Table(_) => arg2,
                    _ => arg1,
                };
                let offset = match offset_hours {
                    Value::Integer(h) => h * 3600,
                    Value::Number(h) => (h * 3600.0) as i64,
                    _ => 0,
                };
                *TZ_OFFSET.write().unwrap() = offset;
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("device", device_table)?;
        Ok(())
    })?;

    Ok(())
}

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
