use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Lua, Table, Value};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

// Global WiFi state (updated by main app)
static WIFI_CONNECTED: RwLock<bool> = RwLock::new(false);
static WIFI_SSID: RwLock<String> = RwLock::new(String::new());
static WIFI_IP: RwLock<String> = RwLock::new(String::new());
static WIFI_RSSI: RwLock<i32> = RwLock::new(-100);

// System info state
static FREE_HEAP: RwLock<u32> = RwLock::new(0);
static CPU_FREQ: RwLock<u32> = RwLock::new(240);
static BOOT_TIME: RwLock<u64> = RwLock::new(0);

// Timezone offset in seconds from UTC (e.g., -18000 for EST, -14400 for EDT)
static TZ_OFFSET: RwLock<i64> = RwLock::new(0);

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

        // device.seconds() -> current unix timestamp in seconds
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

        // device.millis() -> current unix timestamp in milliseconds
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

        // device.uptime() -> seconds since boot
        device_table.set(
            ctx,
            "uptime",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let boot_time = *BOOT_TIME.read().unwrap();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let uptime = now.saturating_sub(boot_time) as i64;
                stack.replace(ctx, uptime);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.wifi_connected() -> bool
        device_table.set(
            ctx,
            "wifi_connected",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let connected = *WIFI_CONNECTED.read().unwrap();
                stack.replace(ctx, connected);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.wifi_ssid() -> string
        device_table.set(
            ctx,
            "wifi_ssid",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let ssid = WIFI_SSID.read().unwrap().clone();
                stack.replace(ctx, ctx.intern(ssid.as_bytes()));
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.ip_address() -> string
        device_table.set(
            ctx,
            "ip_address",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let ip = WIFI_IP.read().unwrap().clone();
                if ip.is_empty() {
                    stack.replace(ctx, ctx.intern(b"Not connected"));
                } else {
                    stack.replace(ctx, ctx.intern(ip.as_bytes()));
                }
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.wifi_rssi() -> integer (dBm)
        device_table.set(
            ctx,
            "wifi_rssi",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let rssi = *WIFI_RSSI.read().unwrap() as i64;
                stack.replace(ctx, rssi);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.free_heap() -> integer (bytes)
        device_table.set(
            ctx,
            "free_heap",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let heap = *FREE_HEAP.read().unwrap() as i64;
                stack.replace(ctx, heap);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.cpu_freq() -> integer (MHz)
        device_table.set(
            ctx,
            "cpu_freq",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let freq = *CPU_FREQ.read().unwrap() as i64;
                stack.replace(ctx, freq);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.localtime() -> table { hour, min, sec, year, month, day, weekday, yearday }
        device_table.set(
            ctx,
            "localtime",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let unix_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let tz_offset = *TZ_OFFSET.read().unwrap();
                let local_secs = unix_secs + tz_offset;

                // Calculate date/time components
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
                result.set(ctx, "weekday", weekday)?;  // 1=Sunday, 7=Saturday
                result.set(ctx, "yearday", yearday)?;

                stack.replace(ctx, result);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // device.set_timezone(offset_hours) -> sets timezone offset
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
    match month {
        1 => 31,
        2 => if is_leap_year(year) { 29 } else { 28 },
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 30,
    }
}

// Calculate year, month, day, and day of year from unix timestamp (in local seconds)
fn unix_to_date(local_secs: i64) -> (i64, i64, i64, i64) {
    // Total days since Unix epoch
    let total_days = local_secs.div_euclid(86400);

    // Start from 1970
    let mut year = 1970i64;
    let mut remaining = total_days;

    // Handle negative days (before 1970)
    while remaining < 0 {
        year -= 1;
        remaining += if is_leap_year(year) { 366 } else { 365 };
    }

    // Count forward through years
    loop {
        let days_this_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_this_year {
            break;
        }
        remaining -= days_this_year;
        year += 1;
    }

    let yearday = remaining + 1; // 1-indexed day of year

    // Find month and day
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

    // day_of_month is 0-indexed, convert to 1-indexed
    let day = day_of_month + 1;

    (year, month, day, yearday)
}

// Calculate weekday from unix timestamp (1=Sunday, 7=Saturday)
fn weekday_from_unix(local_secs: i64) -> i64 {
    let days = local_secs.div_euclid(86400);
    // Jan 1, 1970 was Thursday
    // Thursday = 5 in 1=Sunday system
    // So days=0 should give weekday=5
    ((days + 4).rem_euclid(7) + 1) as i64
}
