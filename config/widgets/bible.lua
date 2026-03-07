-- Bible Verse Widget
-- Fetches Verse of the Day from labs.bible.org API

local theme = require("theme")
local components = require("components")

local M = {}

function M.init(ctx)
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		verse_text = nil,
		verse_ref = nil,
		last_fetch = 0,
		fetch_interval = ctx.opts.update_interval or 3600000, -- 1 hour
		loading = true,
		error = nil,
	}
end

function M.update(state, delta_ms)
	state.last_fetch = state.last_fetch + delta_ms

	if state.last_fetch >= state.fetch_interval or state.verse_text == nil then
		M.fetch_verse(state)
		state.last_fetch = 0
	end
end

function M.fetch_verse(state)
	local url = "https://labs.bible.org/api/?passage=votd&type=json&formatting=plain"

	local response = net.http_get(url, nil, 10000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data and #data > 0 then
			-- API returns array, combine all verses
			local texts = {}
			local first_ref = nil
			local last_ref = nil

			for _, verse in ipairs(data) do
				table.insert(texts, verse.text)
				if not first_ref then
					first_ref = verse.bookname .. " " .. verse.chapter .. ":" .. verse.verse
				end
				last_ref = verse.bookname .. " " .. verse.chapter .. ":" .. verse.verse
			end

			state.verse_text = table.concat(texts, " ")

			-- Format reference
			if #data == 1 then
				state.verse_ref = first_ref
			else
				-- Extract just verse numbers for range
				local first_v = data[1].verse
				local last_v = data[#data].verse
				state.verse_ref = data[1].bookname .. " " .. data[1].chapter .. ":" .. first_v .. "-" .. last_v
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

-- Word wrap helper
local function wrap_text(text, max_chars)
	local lines = {}
	local line = ""

	for word in string.gmatch(text, "%S+") do
		if #line + #word + 1 <= max_chars then
			line = line == "" and word or line .. " " .. word
		else
			if line ~= "" then
				table.insert(lines, line)
			end
			line = word
		end
	end

	if line ~= "" then
		table.insert(lines, line)
	end

	return lines
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

	-- Decorative cross icon (simple)
	gfx:line(px + 5, py + 5, px + 5, py + 20, th.accent_primary, 2)
	gfx:line(px, py + 10, px + 10, py + 10, th.accent_primary, 2)

	-- Title
	gfx:text(px + 20, py + 5, "Verse of the Day", th.text_muted, "small")

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
		local lines = wrap_text(state.verse_text, chars_per_line)

		-- Calculate how many lines we can show
		local line_height = 18
		local max_lines = math.floor((state.height - content_y - 40) / line_height)

		-- Draw verse text
		for i, line in ipairs(lines) do
			if i > max_lines then
				-- Show ellipsis on last line
				gfx:text(px, content_y + (max_lines - 1) * line_height, "...", th.text_secondary, "medium")
				break
			end
			gfx:text(px, content_y + (i - 1) * line_height, line, th.text_secondary, "medium")
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
