-- Weather Widget
-- Fetches and displays weather data from OpenWeatherMap API

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		city = ctx.opts.city or env.get("WEATHER_CITY") or "New York",
		units = ctx.opts.units or env.get("WEATHER_UNITS") or "imperial",
		temperature = nil,
		feels_like = nil,
		description = nil,
		humidity = nil,
		wind_speed = nil,
		icon = nil,
		last_fetch = 0,
		fetch_interval = ctx.opts.update_interval or 300000,
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval or state.temperature == nil then
		M.fetch_weather(state)
		state.last_fetch = 0
	end
end

function M.fetch_weather(state)
	local api_key = env.get("WEATHER_API_KEY")
	if not api_key then
		state.error = "Set WEATHER_API_KEY"
		state.loading = false
		return
	end

	local url = string.format(
		"https://api.openweathermap.org/data/2.5/weather?q=%s&units=%s&appid=%s",
		state.city,
		state.units,
		api_key
	)

	local response = net.http_get(url, nil, 10000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data and data.main then
			state.temperature = math.floor(data.main.temp + 0.5)
			state.feels_like = math.floor(data.main.feels_like + 0.5)
			state.humidity = data.main.humidity
			state.description = data.weather and data.weather[1] and data.weather[1].main or "Unknown"
			state.icon = data.weather and data.weather[1] and data.weather[1].icon or nil
			state.wind_speed = data.wind and math.floor(data.wind.speed + 0.5)
			state.error = nil
		else
			state.error = "Invalid response"
		end
	else
		state.error = response.error or "Request failed"
	end

	state.loading = false
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Weather", {
		accent = th.accent_primary,
	})

	local content_y = py + title_h + 10

	if state.loading then
		components.loading(gfx, px, content_y + 30)
		return
	end

	if state.error then
		components.error(gfx, px, content_y + 10, state.width - px * 2, state.error)
		return
	end

	-- Temperature display (large)
	local temp_str = tostring(state.temperature) .. "°"
	local unit = state.units == "metric" and "C" or "F"

	gfx:text(px, content_y + 15, temp_str, th.text_primary, "xlarge")
	gfx:text(px + #temp_str * 14 + 5, content_y + 20, unit, th.text_muted, "large")

	-- Description
	if state.description then
		gfx:text(px, content_y + 55, state.description, th.text_accent, "large")
	end

	-- City
	gfx:text(px, content_y + 80, state.city, th.text_muted, "small")

	-- Additional info (right side or below based on width)
	local info_x = state.width > 300 and (state.width / 2 + 20) or px
	local info_y = state.width > 300 and (content_y + 15) or (content_y + 100)

	if state.feels_like then
		components.item_row(gfx, info_x, info_y, 140, "Feels like", state.feels_like .. "°", {
			label_color = th.text_muted,
		})
		info_y = info_y + 22
	end

	if state.humidity then
		components.item_row(gfx, info_x, info_y, 140, "Humidity", state.humidity .. "%", {
			label_color = th.text_muted,
		})
		info_y = info_y + 22
	end

	if state.wind_speed then
		local wind_unit = state.units == "metric" and "m/s" or "mph"
		components.item_row(gfx, info_x, info_y, 140, "Wind", state.wind_speed .. " " .. wind_unit, {
			label_color = th.text_muted,
		})
	end
end

function M.on_event(state, event)
	if event.type == "tap" then
		state.last_fetch = state.fetch_interval
		state.loading = true
		return true
	end
	return false
end

return M
