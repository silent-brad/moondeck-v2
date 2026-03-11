-- Stocks Widget
-- Fetches stock prices from Finnhub.io API

local components = require("components")

local M = {}

function M.init(ctx)
	local symbols = ctx.opts.symbols or { "AAPL", "GOOGL" }

	local fetch_interval = ctx.opts.update_interval or 300000 -- 5 minutes

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		symbols = symbols,
		prices = {},
		changes = {},
		current_symbol_index = 1,
		last_fetch = fetch_interval,
		fetch_interval = fetch_interval,
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval then
		state.last_fetch = 0

		local api_key = env.get("STOCKS_API_KEY")
		if not api_key then
			state.error = "No API key"
			state.loading = false
			return
		end

		-- Finnhub only supports one symbol per request, fetch all symbols
		local success_count = 0
		for i = 1, #state.symbols do
			local symbol = state.symbols[i]
			local url = "https://finnhub.io/api/v1/quote?symbol=" .. symbol .. "&token=" .. api_key

			local response = net.http_get(url, {}, 10000)

			if response and response.ok and response.body then
				local data = net.json_decode(response.body)

				if data and data.c then
					state.prices[symbol] = data.c -- current price
					state.changes[symbol] = data.dp -- percent change
					success_count = success_count + 1
				end
			end
		end

		if success_count > 0 then
			state.loading = false
			state.error = nil
		else
			state.error = "Failed to fetch"
			state.loading = false
		end
	end
end

local function format_price(price)
	if not price then
		return "—"
	end
	return "$" .. util.format("%.2f", price)
end

local function format_change(change)
	if not change then
		return "—", "info"
	end

	local sign = ""
	local status = "info"
	if change >= 0 then
		sign = "+"
		status = "ok"
	else
		status = "error"
	end

	return sign .. util.format("%.2f", change) .. "%", status
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Stocks", {
		accent = th.accent_secondary,
	})

	local content_y = py + title_h + 25

	if state.loading then
		components.loading(gfx, px, content_y + 20)
		return
	end

	if state.error then
		components.error(gfx, px, content_y + 10, state.width - px * 2, state.error)
		return
	end

	-- Display each stock
	local row_height = 28
	local max_rows = math.floor((state.height - content_y - py) / row_height)

	for i = 1, #state.symbols do
		if i > max_rows then
			break
		end

		local symbol = state.symbols[i]
		local y = content_y + (i - 1) * row_height
		local price = format_price(state.prices[symbol])
		local change_str, change_status = format_change(state.changes[symbol])

		-- Symbol
		gfx:text(px, y, symbol, th.text_primary, "medium")

		-- Price (center)
		local price_x = state.width / 2 - 30
		gfx:text(price_x, y, price, th.text_primary, "medium")

		-- Change (right)
		local change_color = th.text_muted
		if change_status == "ok" then
			change_color = th.accent_success
		elseif change_status == "error" then
			change_color = th.accent_error
		end
		gfx:text(state.width - px - 60, y, change_str, change_color, "small")
	end

	-- Market status indicator
	local now_hours = math.floor(device.seconds() / 3600) % 24
	local market_open = now_hours >= 14 and now_hours < 21 -- Rough EST market hours in UTC
	local status_text = market_open and "Market Open" or "Market Closed"
	local status_color = market_open and th.accent_success or th.text_muted

	gfx:text(px, state.height - py - 10, status_text, status_color, "small")
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
