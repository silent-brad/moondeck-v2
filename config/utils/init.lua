-- Moondeck Utils
-- Sets up core modules and provides utility functions

print("Moondeck v2 initializing...")

-- Load core modules (these are made available globally)
theme = require("themes")
layout = require("utils.layout")
components = require("utils.components")

-- Set theme from environment (or use default from themes/init.lua)
local theme_name = env.get("THEME")
if theme_name then
  if theme:set(theme_name) then
    print("Theme: " .. theme_name)
  else
    print("Theme '" .. theme_name .. "' not found, using default")
  end
end
print("Active theme: " .. (theme:get().name or "unknown"))

-- Global configuration
config = {
  refresh_rate = 30,
  screen_width = 800,
  screen_height = 480,
}

-- Utility functions
local M = {}

function M.format_time(timestamp)
  local secs = timestamp % 60
  local mins = math.floor(timestamp / 60) % 60
  local hours = math.floor(timestamp / 3600) % 24
  return string.format("%02d:%02d:%02d", hours, mins, secs)
end

function M.format_number(n)
  if n >= 1000000 then
    return string.format("%.1fM", n / 1000000)
  elseif n >= 1000 then
    return string.format("%.1fK", n / 1000)
  else
    return tostring(n)
  end
end

function M.clamp(value, min, max)
  return math.max(min, math.min(max, value))
end

function M.lerp(a, b, t)
  return a + (b - a) * t
end

print("Initialization complete!")

return M
