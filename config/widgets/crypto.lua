-- Crypto Widget
-- Fetches cryptocurrency prices from CoinGecko API (no API key required)

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	-- Parse coin list from env or opts
	local coins_str = ctx.opts.coins or env.get("CRYPTO_COINS") or "bitcoin,ethereum"
	local coins = {}
	for coin in string.gmatch(coins_str, "([^,]+)") do
		table.insert(coins, coin:match("^%s*(.-)%s*$")) -- trim whitespace
	end

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		coins = coins,
		currency = ctx.opts.currency or env.get("CRYPTO_CURRENCY") or "usd",
		prices = {},
		changes = {},
		last_fetch = 0,
		fetch_interval = ctx.opts.update_interval or 60000, -- 1 minute
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
	if #state.coins == 0 then
		state.error = "No coins configured"
		state.loading = false
		return
	end

	local coins_param = table.concat(state.coins, ",")
	local url = string.format(
		"https://api.coingecko.com/api/v3/simple/price?ids=%s&vs_currencies=%s&include_24hr_change=true",
		coins_param,
		state.currency
	)

	local response = net.http_get(url, nil, 10000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data then
			state.prices = {}
			state.changes = {}

			for _, coin in ipairs(state.coins) do
				if data[coin] then
					state.prices[coin] = data[coin][state.currency]
					local change_key = state.currency .. "_24h_change"
					state.changes[coin] = data[coin][change_key]
				end
			end

			state.error = nil
		else
			state.error = "Invalid response"
		end
	else
		state.error = response.error or "Request failed"
	end

	state.loading = false
end

-- Format price with appropriate precision
local function format_price(price, currency)
	if not price then
		return "—"
	end

	local symbol = currency == "usd" and "$" or currency == "eur" and "€" or currency == "gbp" and "£" or ""

	if price >= 1000 then
		return symbol .. string.format("%.0f", price)
	elseif price >= 1 then
		return symbol .. string.format("%.2f", price)
	else
		return symbol .. string.format("%.4f", price)
	end
end

-- Format change percentage
local function format_change(change)
	if not change then
		return "—", "info"
	end

	local sign = change >= 0 and "+" or ""
	local status = change >= 0 and "ok" or "error"

	return sign .. string.format("%.1f%%", change), status
end

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
}

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Crypto", {
		accent = th.accent_primary,
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

	-- Display each coin
	local row_height = 30
	local max_rows = math.floor((state.height - content_y - py) / row_height)

	for i, coin in ipairs(state.coins) do
		if i > max_rows then
			break
		end

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
		local change_color = change_status == "ok" and th.accent_success
			or change_status == "error" and th.accent_error
			or th.text_muted
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
