-- Stocks Widget
-- Fetches stock prices from Stockdata.org API

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	-- Parse symbol list from env or opts
	local symbols_str = ctx.opts.symbols or env.get("STOCKS_SYMBOLS") or "AAPL,GOOGL"
	local symbols = {}
	for symbol in string.gmatch(symbols_str, "([^,]+)") do
		table.insert(symbols, symbol:match("^%s*(.-)%s*$"):upper())
	end

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		symbols = symbols,
		prices = {},
		changes = {},
		last_fetch = 0,
		fetch_interval = ctx.opts.update_interval or 300000, -- 5 minutes
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval or next(state.prices) == nil then
		M.fetch_prices(state)
		state.last_fetch = 0
	end
end

function M.fetch_prices(state)
	local api_key = env.get("STOCKS_API_KEY")
	if not api_key then
		state.error = "Set STOCKS_API_KEY"
		state.loading = false
		return
	end

	if #state.symbols == 0 then
		state.error = "No symbols configured"
		state.loading = false
		return
	end

	local symbols_param = table.concat(state.symbols, ",")
	local url = string.format("https://api.stockdata.org/v1/data/quote?symbols=%s&api_token=%s", symbols_param, api_key)

	local response = net.http_get(url, nil, 15000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data and data.data then
			state.prices = {}
			state.changes = {}

			for _, quote in ipairs(data.data) do
				local symbol = quote.ticker
				state.prices[symbol] = quote.price
				-- Calculate percentage change
				if quote.day_open and quote.day_open > 0 then
					local change = ((quote.price - quote.day_open) / quote.day_open) * 100
					state.changes[symbol] = change
				else
					state.changes[symbol] = quote.day_change or 0
				end
			end

			state.error = nil
		else
			state.error = data and data.error and data.error.message or "Invalid response"
		end
	else
		state.error = response.error or "Request failed"
	end

	state.loading = false
end

local function format_price(price)
	if not price then
		return "—"
	end
	return "$" .. string.format("%.2f", price)
end

local function format_change(change)
	if not change then
		return "—", "info"
	end

	local sign = change >= 0 and "+" or ""
	local status = change >= 0 and "ok" or "error"

	return sign .. string.format("%.2f%%", change), status
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
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Stocks", {
		accent = th.accent_secondary,
	})

	local content_y = py + title_h + 10

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

	for i, symbol in ipairs(state.symbols) do
		if i > max_rows then
			break
		end

		local y = content_y + (i - 1) * row_height
		local price = format_price(state.prices[symbol])
		local change_str, change_status = format_change(state.changes[symbol])

		-- Symbol
		gfx:text(px, y, symbol, th.text_primary, "medium")

		-- Price (center)
		local price_x = state.width / 2 - 30
		gfx:text(price_x, y, price, th.text_primary, "medium")

		-- Change (right)
		local change_color = change_status == "ok" and th.accent_success
			or change_status == "error" and th.accent_error
			or th.text_muted
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
