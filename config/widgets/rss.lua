-- RSS Widget
-- Fetches feed entries from Miniflux API

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		entries = {},
		current_index = 1,
		limit = ctx.opts.limit or tonumber(env.get("MINIFLUX_LIMIT")) or 10,
		last_fetch = 0,
		fetch_interval = ctx.opts.update_interval or 300000, -- 5 minutes
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval or #state.entries == 0 then
		M.fetch_entries(state)
		state.last_fetch = 0
	end
end

function M.fetch_entries(state)
	local base_url = env.get("MINIFLUX_URL")
	local api_key = env.get("MINIFLUX_API_KEY")

	if not base_url then
		state.error = "Set MINIFLUX_URL"
		state.loading = false
		return
	end

	if not api_key then
		state.error = "Set MINIFLUX_API_KEY"
		state.loading = false
		return
	end

	-- Fetch unread entries
	local url = base_url .. "/v1/entries?status=unread&limit=" .. state.limit .. "&order=published_at&direction=desc"

	local headers = {
		["X-Auth-Token"] = api_key,
	}

	local response = net.http_get(url, headers, 15000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data and data.entries then
			state.entries = {}

			for _, entry in ipairs(data.entries) do
				table.insert(state.entries, {
					id = entry.id,
					title = entry.title,
					feed = entry.feed and entry.feed.title or "Unknown",
					url = entry.url,
					published = entry.published_at,
				})
			end

			-- Reset to first entry if we got new data
			if #state.entries > 0 and state.current_index > #state.entries then
				state.current_index = 1
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

-- Truncate text with ellipsis
local function truncate(text, max_len)
	if not text then
		return ""
	end
	if #text <= max_len then
		return text
	end
	return text:sub(1, max_len - 3) .. "..."
end

-- Format relative time
local function time_ago(timestamp)
	if not timestamp then
		return ""
	end

	-- Simple parsing - assumes ISO format
	local now = device.seconds()

	-- Very rough estimate (would need proper date parsing)
	local hours_ago = math.floor((now % 86400) / 3600)

	if hours_ago < 1 then
		return "Just now"
	elseif hours_ago < 24 then
		return hours_ago .. "h ago"
	else
		return math.floor(hours_ago / 24) .. "d ago"
	end
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

	-- Title bar with entry count
	local title = "RSS Feed"
	if #state.entries > 0 then
		title = title .. " (" .. #state.entries .. ")"
	end

	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, title, {
		accent = th.accent_primary,
	})

	local content_y = py + title_h + 10

	if state.loading then
		components.loading(gfx, px, content_y + 20)
		return
	end

	if state.error then
		components.error(gfx, px, content_y, state.width - px * 2, state.error)
		return
	end

	if #state.entries == 0 then
		gfx:text(px, content_y + 20, "No unread entries", th.text_muted, "medium")
		return
	end

	-- Display entries as list
	local row_height = 45
	local max_rows = math.floor((state.height - content_y - py - 20) / row_height)
	local title_max_chars = math.floor((state.width - px * 2) / 7)

	for i, entry in ipairs(state.entries) do
		if i > max_rows then
			break
		end

		local y = content_y + (i - 1) * row_height

		-- Entry indicator
		gfx:fill_circle(px + 4, y + 8, 3, th.accent_primary)

		-- Title
		local title_text = truncate(entry.title, title_max_chars)
		gfx:text(px + 15, y, title_text, th.text_primary, "medium")

		-- Feed name and time
		local meta = entry.feed
		gfx:text(px + 15, y + 18, meta, th.text_muted, "small")
	end

	-- Navigation hint at bottom
	if #state.entries > max_rows then
		local more = #state.entries - max_rows
		gfx:text(px, state.height - py - 5, "+" .. more .. " more", th.text_muted, "small")
	end
end

function M.on_event(state, event)
	if event.type == "tap" then
		-- Cycle through entries or refresh
		if #state.entries > 0 then
			state.current_index = (state.current_index % #state.entries) + 1
		end
		return true
	elseif event.type == "long_press" then
		-- Refresh on long press
		state.last_fetch = state.fetch_interval
		state.loading = true
		return true
	end
	return false
end

return M
