-- Status Widget
-- Displays WiFi and system status

local M = {}

function M.init(ctx)
    return {
        x = ctx.x,
        y = ctx.y,
        width = ctx.width,
        height = ctx.height,
        wifi_connected = false,
        ip_address = "Not connected",
        uptime = 0,
    }
end

function M.update(state, delta_ms)
    state.uptime = state.uptime + delta_ms
end

function M.render(state, gfx)
    -- Draw background panel
    gfx:fill_rounded_rect(0, 0, state.width, state.height, 12, "#0f3460")
    
    -- Title
    gfx:text(15, 30, "System Status", "white", "large")
    gfx:line(15, 45, state.width - 15, 45, "#e94560", 2)
    
    -- WiFi status
    local wifi_color = state.wifi_connected and "#00ff00" or "#ff0000"
    local wifi_status = state.wifi_connected and "Connected" or "Disconnected"
    gfx:fill_circle(30, 80, 8, wifi_color)
    gfx:text(50, 85, "WiFi: " .. wifi_status, "white", "medium")
    
    -- IP Address
    gfx:text(50, 115, "IP: " .. state.ip_address, "#888888", "small")
    
    -- Uptime
    local uptime_secs = math.floor(state.uptime / 1000)
    local uptime_str = utils.format_time(uptime_secs)
    gfx:text(15, 160, "Uptime: " .. uptime_str, "white", "medium")
    
    -- Border
    gfx:stroke_rect(5, 5, state.width - 10, state.height - 10, "#e94560", 1)
end

function M.on_event(state, event)
    return false
end

return M
