-- Clock Widget
-- Displays current time and date with TRMNL-inspired styling

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
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

function M.render(state, gfx)
	local th = theme:get()
	local now = device.seconds()

	-- Calculate time components
	local secs = now % 60
	local mins = math.floor(now / 60) % 60
	local hours = math.floor(now / 3600) % 24

	-- Draw card background
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

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

	-- Build time string
	local time_str
	if state.show_seconds then
		time_str = string.format("%02d:%02d:%02d", display_hours, mins, secs)
	else
		time_str = string.format("%02d:%02d", display_hours, mins)
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
		local days = math.floor(now / 86400)
		-- Simple day calculation (approximate)
		local weekdays = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" }
		local weekday = weekdays[(days % 7) + 1]

		local date_str = weekday .. " • Day " .. tostring(days % 365 + 1)
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
