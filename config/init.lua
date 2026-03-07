-- Moondeck Initialization Script
-- Loads theme, layout system, and component library

print("Moondeck v2 initializing...")

-- Load core modules (these are made available globally)
theme = require("theme")
layout = require("layout")
components = require("components")

-- Set theme from environment
local theme_name = env.get("THEME") or "dark"
if theme:set(theme_name) then
	print("Theme: " .. theme_name)
else
	print("Theme not found, using dark")
	theme:set("dark")
end

-- Global configuration
config = {
	refresh_rate = 30,
	screen_width = 800,
	screen_height = 480,
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

function utils.lerp(a, b, t)
	return a + (b - a) * t
end

print("Initialization complete!")
