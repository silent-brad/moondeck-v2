-- Bible Verse Widget
-- Fetches random verses from bible-api.com

local components = require("components")

local M = {}

function M.init(ctx)
	local fetch_interval = ctx.opts.update_interval or 3600000 -- 1 hour
	local translation = ctx.opts.translation or "kjv" -- KJV default

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		translation = translation,
		verse_text = nil,
		verse_ref = nil,
		last_fetch = fetch_interval, -- Trigger immediate fetch
		fetch_interval = fetch_interval,
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval then
		state.last_fetch = 0

		-- Fetch random verse from bible-api.com
		local url = "https://bible-api.com/data/" .. state.translation .. "/random"

		local response = net.http_get(url, {}, 10000)

		if response and response.ok and response.body then
			local data = net.json_decode(response.body)

			if data and data.random_verse then
				local verse = data.random_verse

				-- Clean up text (remove leading/trailing whitespace)
				local text = verse.text or ""
				-- Simple trim: remove leading/trailing newlines
				local clean_text = ""
				local started = false
				for i = 1, #text do
					local c = string.sub(text, i, i)
					if c ~= "\n" and c ~= "\r" then
						started = true
					end
					if started then
						clean_text = clean_text .. c
					end
				end
				-- Trim trailing
				while
					#clean_text > 0
					and (
						string.sub(clean_text, #clean_text, #clean_text) == "\n"
						or string.sub(clean_text, #clean_text, #clean_text) == "\r"
					)
				do
					clean_text = string.sub(clean_text, 1, #clean_text - 1)
				end

				state.verse_text = clean_text
				state.verse_ref = verse.book .. " " .. verse.chapter .. ":" .. verse.verse
				state.loading = false
				state.error = nil
			else
				state.error = "No verse data"
				state.loading = false
			end
		else
			state.error = "Failed to fetch"
			state.loading = false
		end
	end
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	-- Decorative cross icon (simple)
	gfx:line(px + 5, py + 5, px + 5, py + 20, th.accent_primary, 2)
	gfx:line(px, py + 10, px + 10, py + 10, th.accent_primary, 2)

	-- Title
	gfx:text(px + 20, py + 5, "Daily Verse", th.text_muted, "small")

	local content_y = py + 35

	if state.loading then
		components.loading(gfx, px, content_y + 20)
		return
	end

	if state.error then
		components.error(gfx, px, content_y, state.width - px * 2, state.error)
		return
	end

	if state.verse_text then
		-- Calculate characters per line based on width
		local chars_per_line = math.floor((state.width - px * 2) / 7)
		local lines = util.word_wrap(state.verse_text, chars_per_line)

		-- Calculate how many lines we can show
		local line_height = 18
		local max_lines = math.floor((state.height - content_y - 40) / line_height)

		-- Draw verse text
		for i = 1, #lines do
			if i > max_lines then
				-- Show ellipsis on last line
				gfx:text(px, content_y + (max_lines - 1) * line_height, "...", th.text_secondary, "medium")
				break
			end
			gfx:text(px, content_y + (i - 1) * line_height, lines[i], th.text_secondary, "medium")
		end

		-- Draw reference at bottom
		if state.verse_ref then
			gfx:text(px, state.height - py - 15, "— " .. state.verse_ref, th.text_accent, "medium")
		end
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
