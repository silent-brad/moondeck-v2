-- Crypto Widget
-- Fetches cryptocurrency prices from CoinGecko API (no API key required)

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	-- TODO: Add env var for coins
	local coins = ctx.opts.coins or { "bitcoin", "ethereum" }
	local currency = ctx.opts.currency or "usd"

	local fetch_interval = ctx.opts.update_interval or 60000

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		coins = coins,
		currency = currency,
		prices = {},
		changes = {},
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

		-- Build coin IDs string
		local ids = ""
		for i = 1, #state.coins do
			if i > 1 then
				ids = ids .. ","
			end
			ids = ids .. state.coins[i]
		end

		-- CoinGecko API URL
		local url = "https://api.coingecko.com/api/v3/simple/price?ids="
			.. ids
			.. "&vs_currencies="
			.. state.currency
			.. "&include_24hr_change=true"

		local response = net.http_get(url, {}, 15000)

		if response and response.ok and response.body then
			local data = net.json_decode(response.body)

			if data then
				for i = 1, #state.coins do
					local coin = state.coins[i]
					local coin_data = data[coin]
					if coin_data then
						state.prices[coin] = coin_data[state.currency]
						local change_key = state.currency .. "_24h_change"
						state.changes[coin] = coin_data[change_key]
					end
				end
				state.loading = false
				state.error = nil
			else
				state.error = "Invalid response"
				state.loading = false
			end
		else
			state.error = response and response.error or "Network error"
			state.loading = false
		end
	end
end

-- Format price with appropriate precision
local function format_price(price, currency)
	if not price then
		return "—"
	end

	local symbol = ""
	if currency == "usd" then
		symbol = "$"
	elseif currency == "eur" then
		symbol = "€"
	elseif currency == "gbp" then
		symbol = "£"
	end

	if price >= 1000 then
		return symbol .. util.format("%.0f", price)
	elseif price >= 1 then
		return symbol .. util.format("%.2f", price)
	else
		return symbol .. util.format("%.4f", price)
	end
end

-- Format change percentage
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

	return sign .. util.format("%.1f", change) .. "%", status
end

-- TODO: Move to config
-- Coin display names
local coin_names = {
	bitcoin = "BTC",
	ethereum = "ETH",
	solana = "SOL",
	cardano = "ADA",
	dogecoin = "DOGE",
	ripple = "XRP",
	polkadot = "DOT",
	avalanche = "AVAX",
	chainlink = "LINK",
	polygon = "MATIC",
	litecoin = "LTC",
	monero = "XMR",
}

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Crypto", {
		accent = th.accent_primary,
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

	-- Display each coin
	local row_height = 30
	local max_rows = math.floor((state.height - content_y - py) / row_height)

	for i = 1, #state.coins do
		if i > max_rows then
			break
		end

		local coin = state.coins[i]
		local y = content_y + (i - 1) * row_height
		local name = coin_names[coin] or coin:upper():sub(1, 4)
		local price = format_price(state.prices[coin], state.currency)
		local change_str, change_status = format_change(state.changes[coin])

		-- Coin name
		gfx:text(px, y, name, th.text_primary, "medium")

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
