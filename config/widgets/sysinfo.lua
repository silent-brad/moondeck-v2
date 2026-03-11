-- System Info Widget
-- Displays device system information

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		uptime = 0,
		free_heap = 0,
		wifi_rssi = 0,
		cpu_freq = 0,
		last_update = 0,
	}
end

function M.update(state, delta_ms)
	state.last_update = state.last_update + delta_ms

	-- Update system info every second
	if state.last_update >= 1000 then
		state.uptime = device.uptime and device.uptime() or (device.seconds() % 86400)
		state.free_heap = device.free_heap and device.free_heap() or 0
		state.wifi_rssi = device.wifi_rssi and device.wifi_rssi() or -50
		state.cpu_freq = device.cpu_freq and device.cpu_freq() or 160
		state.last_update = 0
	end
end

-- Format number with padding (no string.format dependency)
local function pad2(n)
	if n < 10 then
		return "0" .. n
	end
	return "" .. n
end

-- Format bytes to human readable (no string.format dependency)
local function format_bytes(bytes)
	if bytes >= 1048576 then
		local mb = math.floor(bytes / 104857.6) / 10
		return mb .. " MB"
	elseif bytes >= 1024 then
		local kb = math.floor(bytes / 102.4) / 10
		return kb .. " KB"
	else
		return tostring(bytes) .. " B"
	end
end

-- Format uptime (no string.format dependency)
local function format_uptime(seconds)
	local days = math.floor(seconds / 86400)
	local hours = math.floor((seconds % 86400) / 3600)
	local mins = math.floor((seconds % 3600) / 60)
	local secs = math.floor(seconds % 60)

	if days > 0 then
		return days .. "d " .. pad2(hours) .. ":" .. pad2(mins) .. ":" .. pad2(secs)
	else
		return pad2(hours) .. ":" .. pad2(mins) .. ":" .. pad2(secs)
	end
end

-- Get WiFi signal strength description
local function wifi_strength(rssi)
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
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "System", {
		accent = th.accent_secondary,
	})

	local content_y = py + title_h + 15
	local row_height = 35
	local col_width = (state.width - px * 2) / 2

	-- Uptime
	gfx:text(px, content_y, "Uptime", th.text_muted, "small")
	gfx:text(px, content_y + 14, format_uptime(state.uptime), th.text_primary, "medium")

	-- CPU Frequency
	gfx:text(px + col_width, content_y, "CPU", th.text_muted, "small")
	gfx:text(px + col_width, content_y + 14, state.cpu_freq .. " MHz", th.text_primary, "medium")

	content_y = content_y + row_height + 10

	-- Free Memory
	gfx:text(px, content_y, "Free Memory", th.text_muted, "small")
	gfx:text(px, content_y + 14, format_bytes(state.free_heap), th.text_primary, "medium")

	-- Memory bar
	local mem_total = 400000 -- Approximate total heap
	local mem_used_ratio = 1 - (state.free_heap / mem_total)
	mem_used_ratio = math.max(0, math.min(1, mem_used_ratio))

	components.progress_bar(gfx, px, content_y + 32, col_width - 20, 8, mem_used_ratio, {
		bg = th.bg_tertiary,
		fg = mem_used_ratio > 0.8 and th.accent_warning or th.accent_primary,
	})

	-- WiFi Signal
	gfx:text(px + col_width, content_y, "WiFi Signal", th.text_muted, "small")
	local strength_text, strength_status = wifi_strength(state.wifi_rssi)
	gfx:text(
		px + col_width,
		content_y + 14,
		strength_text .. " (" .. state.wifi_rssi .. " dBm)",
		th.text_primary,
		"medium"
	)

	-- WiFi bar
	local wifi_ratio = math.max(0, math.min(1, (state.wifi_rssi + 100) / 50))
	local wifi_color = strength_status == "ok" and th.accent_success
		or strength_status == "warning" and th.accent_warning
		or th.accent_error

	components.progress_bar(gfx, px + col_width, content_y + 32, col_width - 20, 8, wifi_ratio, {
		bg = th.bg_tertiary,
		fg = wifi_color,
	})

	content_y = content_y + row_height + 25

	--[[
	-- Divider
	components.divider(gfx, px, content_y, state.width - px * 2, { color = th.border_primary })

	content_y = content_y + 15

	-- Device info
	gfx:text(px, content_y, "Device", th.text_muted, "small")
	gfx:text(px, content_y + 14, "ESP32-S3 • 800x480 LCD", th.text_secondary, "small")

	-- Status indicators
	local status_y = content_y
	components.status(gfx, px + col_width, status_y, "WiFi Connected", "ok")
	components.status(gfx, px + col_width, status_y + 18, "Display Active", "ok")
  --]]
end

function M.on_event(state, event)
	return false
end

return M
