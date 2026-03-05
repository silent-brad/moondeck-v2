-- Clock Widget
-- Displays current time and date

local M = {}

function M.init(ctx)
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		show_seconds = ctx.opts.show_seconds or true,
		show_date = ctx.opts.show_date or true,
		last_update = 0,
	}
end

function M.update(state, delta_ms)
	state.last_update = state.last_update + delta_ms
end

function M.render(state, gfx)
	local now = device.seconds()

	-- Calculate time components
	local secs = now % 60
	local mins = math.floor(now / 60) % 60
	local hours = math.floor(now / 3600) % 24

	-- Format time string
	local time_str
	if state.show_seconds then
		time_str = string.format("%02d:%02d:%02d", hours, mins, secs)
	else
		time_str = string.format("%02d:%02d", hours, mins)
	end

	-- Draw background panel
	gfx:fill_rounded_rect(0, 0, state.width, state.height, 12, "#0f3460")

	-- Draw time
	gfx:text(state.width / 2 - 100, 70, time_str, "white", "xlarge")

	-- Draw date if enabled
	if state.show_date then
		local days = math.floor(now / 86400)
		local date_str = "Day " .. tostring(days) -- Simplified, ESP doesn't have full date
		gfx:text(state.width / 2 - 40, 130, date_str, "#888888", "medium")
	end

	-- Draw decorative elements
	gfx:stroke_rect(10, 10, state.width - 20, state.height - 20, "#e94560", 2)
end

function M.on_event(state, event)
	if event.type == "tap" then
		-- Toggle seconds display on tap
		state.show_seconds = not state.show_seconds
		return true
	end
	return false
end

return M
