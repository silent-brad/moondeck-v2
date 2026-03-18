-- Clock Widget

local M = {}

function M.init(ctx)
  -- Set timezone on init (hours offset from UTC)
  -- Examples: -5 for EST, -4 for EDT, 0 for UTC, 1 for CET
  local tz_offset = ctx.opts.timezone or -5 -- Default to EST
  device.set_timezone(tz_offset)

  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    show_seconds = ctx.opts.show_seconds ~= false,
    show_date = ctx.opts.show_date ~= false,
    format_24h = ctx.opts.format_24h or false,
    last_update = 0,
  }
end

function M.update(state, delta_ms)
  state.last_update = state.last_update + delta_ms
end

local function pad2(n)
  if n < 10 then
    return "0" .. n
  end
  return "" .. n
end

local month_names = { "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec" }
local weekday_names = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" }

function M.render(state, gfx)
  local th = theme:get()
  local t = device.localtime()
  --print(t.hour, t.min, t.sec, t.weekday, t.month, t.day, t.year)

  local hours = t.hour
  local mins = t.min
  local secs = t.sec

  -- Draw card background
  components.card(gfx, 0, 0, state.width, state.height)

  -- Padding
  local px, py = 20, 15

  -- Format time
  local display_hours = hours
  local am_pm = ""

  if not state.format_24h then
    am_pm = hours >= 12 and "PM" or "AM"
    display_hours = hours % 12
    if display_hours == 0 then
      display_hours = 12
    end
  end

  -- Build time string (hours not padded, minutes/seconds padded)
  local time_str
  if state.show_seconds then
    time_str = display_hours .. ":" .. pad2(mins) .. ":" .. pad2(secs)
  else
    time_str = display_hours .. ":" .. pad2(mins)
  end

  -- Draw time (centered, large)
  local time_x = state.width / 2 - (#time_str * 14) / 2
  local time_y = state.height / 2 - 10

  gfx:text(time_x, time_y, time_str, th.text_primary, "xlarge")

  -- Draw AM/PM indicator
  if not state.format_24h then
    gfx:text(time_x + #time_str * 14 + 10, time_y + 8, am_pm, th.text_muted, "medium")
  end

  -- Draw date if enabled
  if state.show_date then
    local weekday = weekday_names[t.weekday]
    local month = month_names[t.month]
    local date_str = weekday .. ", " .. month .. " " .. t.day .. ", " .. t.year

    local date_x = state.width / 2 - (#date_str * 4)
    gfx:text(date_x, time_y + 40, date_str, th.text_muted, "medium")
  end

  -- Accent line at top
  gfx:line(px, py, state.width - px, py, th.accent_primary, 2)
end

function M.on_event(state, event)
  if event.type == "tap" then
    state.show_seconds = not state.show_seconds
    return true
  end
  return false
end

return M
