-- Status Widget

local M = {}

function M.init(ctx)
  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    wifi_connected = false,
    wifi_ssid = "",
    ip_address = "Not connected",
    rssi = -100,
    uptime = 0,
    last_check = 0,
  }
end

function M.update(state, delta_ms)
  state.uptime = state.uptime + delta_ms
  state.last_check = state.last_check + delta_ms

  -- Check WiFi status periodically
  if state.last_check >= 5000 then
    if device.wifi_connected then
      state.wifi_connected = device.wifi_connected()
    end
    if device.wifi_ssid then
      state.wifi_ssid = device.wifi_ssid()
    end
    if device.ip_address then
      state.ip_address = device.ip_address()
    end
    if device.wifi_rssi then
      state.rssi = device.wifi_rssi()
    end
    state.last_check = 0
  end
end

local function format_uptime(ms)
  local secs = math.floor(ms / 1000)
  local mins = math.floor(secs / 60) % 60
  local hours = math.floor(secs / 3600) % 24
  local days = math.floor(secs / 86400)

  if days > 0 then
    return util.format("%dd %02d:%02d", days, hours, mins)
  else
    return util.format("%02d:%02d:%02d", hours, mins, secs % 60)
  end
end

local function rssi_to_strength(rssi)
  if rssi >= -50 then
    return "Excellent", "ok"
  elseif rssi >= -60 then
    return "Good", "ok"
  elseif rssi >= -70 then
    return "Fair", "warning"
  else
    return "Weak", "error"
  end
end

function M.render(state, gfx)
  -- Get theme colors directly from the global theme module
  local th = theme:get()
  local px, py = 15, 10

  -- Draw card
  components.card(gfx, 0, 0, state.width, state.height)

  -- Title bar
  local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Status", {
    accent = th.accent_secondary,
    font_size = 24,
  })

  local content_y = py + title_h + 8
  local row_height = 20

  -- WiFi Status
  local wifi_status = state.wifi_connected and "Connected" or "Disconnected"
  local wifi_indicator = state.wifi_connected and "ok" or "error"

  components.status(gfx, px, content_y, "WiFi: " .. wifi_status, wifi_indicator, { font_size = 11 })
  content_y = content_y + row_height

  -- SSID + Signal on same line
  if state.wifi_connected then
    local strength_text = rssi_to_strength(state.rssi)
    local info = state.wifi_ssid ~= "" and state.wifi_ssid or ""
    if info ~= "" then
      info = info .. "  |  " .. strength_text .. " (" .. state.rssi .. " dBm)"
    end
    if info ~= "" then
      gfx:text(px + 14, content_y, info, th.text_muted, "inter", 11)
      content_y = content_y + 16
    end
  end

  -- IP Address (compact)
  if state.wifi_connected then
    gfx:text(px, content_y, "IP: " .. state.ip_address, th.text_primary, "inter", 12)
    content_y = content_y + 18
  end

  -- Divider
  if content_y + 30 <= state.height then
    components.divider(gfx, px, content_y, state.width - px * 2, { color = th.border_primary })
    content_y = content_y + 10

    -- Uptime
    gfx:text(px, content_y, "Up: " .. format_uptime(state.uptime), th.text_muted, "inter", 11)
  end
end

function M.on_event(state, event)
  return false
end

return M
