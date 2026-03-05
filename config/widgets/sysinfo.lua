-- System Info Widget
-- Displays device system information

local M = {}

function M.init(ctx)
    return {
        x = ctx.x,
        y = ctx.y,
        width = ctx.width,
        height = ctx.height,
        uptime_ms = 0,
        frame_count = 0,
    }
end

function M.update(state, delta_ms)
    state.uptime_ms = state.uptime_ms + delta_ms
    state.frame_count = state.frame_count + 1
end

function M.render(state, gfx)
    -- Draw background panel
    gfx:fill_rounded_rect(0, 0, state.width, state.height, 16, "#1a1a2e")
    
    -- Title
    gfx:text(20, 40, "System Information", "white", "large")
    gfx:line(20, 55, state.width - 20, 55, "#e94560", 2)
    
    local y = 90
    local line_height = 40
    
    -- Device info
    gfx:text(30, y, "Device:", "#888888", "medium")
    gfx:text(200, y, "ESP32-S3 Moondeck", "white", "medium")
    y = y + line_height
    
    -- Display
    gfx:text(30, y, "Display:", "#888888", "medium")
    gfx:text(200, y, device.screen_width() .. "x" .. device.screen_height() .. " RGB565", "white", "medium")
    y = y + line_height
    
    -- Uptime
    local uptime_secs = math.floor(state.uptime_ms / 1000)
    local hours = math.floor(uptime_secs / 3600)
    local mins = math.floor((uptime_secs % 3600) / 60)
    local secs = uptime_secs % 60
    local uptime_str = string.format("%02d:%02d:%02d", hours, mins, secs)
    
    gfx:text(30, y, "Uptime:", "#888888", "medium")
    gfx:text(200, y, uptime_str, "white", "medium")
    y = y + line_height
    
    -- Frame count
    gfx:text(30, y, "Frames:", "#888888", "medium")
    gfx:text(200, y, utils.format_number(state.frame_count), "white", "medium")
    y = y + line_height
    
    -- Memory (placeholder - actual ESP memory would need HAL)
    gfx:text(30, y, "Free Heap:", "#888888", "medium")
    gfx:text(200, y, "-- KB", "#888888", "medium")
    y = y + line_height
    
    -- Version
    gfx:text(30, y, "Version:", "#888888", "medium")
    gfx:text(200, y, "v0.1.0", "#00d9ff", "medium")
    
    -- Progress bar decoration
    local bar_y = state.height - 60
    local bar_width = state.width - 60
    local progress = (state.uptime_ms % 10000) / 10000
    
    gfx:fill_rounded_rect(30, bar_y, bar_width, 20, 10, "#0f3460")
    gfx:fill_rounded_rect(30, bar_y, math.floor(bar_width * progress), 20, 10, "#e94560")
    
    -- Border
    gfx:stroke_rounded_rect(5, 5, state.width - 10, state.height - 10, 12, "#e94560", 2)
end

function M.on_event(state, event)
    return false
end

return M
