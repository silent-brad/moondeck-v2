-- Moondeck Initialization Script
-- This file is loaded before pages.lua

print("Moondeck v2 initializing...")
print("Screen: " .. device.screen_width() .. "x" .. device.screen_height())

-- Global configuration
config = {
	refresh_rate = 30,
	theme = {
		background = "#1a1a2e",
		primary = "#0f3460",
		accent = "#e94560",
		text = "#ffffff",
		text_dim = "#888888",
	},
}

-- Utility functions available to all widgets
utils = {}

function utils.format_time(timestamp)
	local secs = timestamp % 60
	local mins = math.floor(timestamp / 60) % 60
	local hours = math.floor(timestamp / 3600) % 24
	return string.format("%02d:%02d:%02d", hours, mins, secs)
end

function utils.format_number(n)
	if n >= 1000000 then
		return string.format("%.1fM", n / 1000000)
	elseif n >= 1000 then
		return string.format("%.1fK", n / 1000)
	else
		return tostring(n)
	end
end

function utils.clamp(value, min, max)
	return math.max(min, math.min(max, value))
end

print("Initialization complete!")
