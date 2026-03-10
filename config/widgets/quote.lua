-- Quote Widget
-- Displays inspirational quotes with elegant styling

local theme = require("theme")
local components = require("components")

local M = {}

-- Built-in quotes collection
local builtin_quotes = {
	{ text = "The only way to do great work is to love what you do.", author = "Steve Jobs" },
	{ text = "Innovation distinguishes between a leader and a follower.", author = "Steve Jobs" },
	{ text = "Stay hungry, stay foolish.", author = "Steve Jobs" },
	{ text = "It is during our darkest moments that we must focus to see the light.", author = "Aristotle" },
	{ text = "In the middle of difficulty lies opportunity.", author = "Albert Einstein" },
	{
		text = "Many of life's failures are people who did not realize how close they were to success when they gave up.",
		author = "Thomas Edison",
	},
	{ text = "An unexamined life is not worth living.", author = "Socrates" },
	{ text = "Simplicity is the ultimate sophistication.", author = "Leonardo da Vinci" },
	{ text = "The only true wisdom is in knowing you know nothing.", author = "Socrates" },
	{ text = "Do what you can, with what you have, where you are.", author = "Theodore Roosevelt" },
	{ text = "Everything you've ever wanted is on the other side of fear.", author = "George Addair" },
}

local function set_quote(state, index)
	local quote = builtin_quotes[index]
	state.quote_text = quote.text
	state.quote_author = quote.author
	state.quote_index = index
end

function M.init(ctx)
	local state = {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		quote_text = nil,
		quote_author = nil,
		quote_index = 1,
		last_change = 0,
		change_interval = ctx.opts.change_interval or 60000, -- 1 minute
		use_api = ctx.opts.use_api or false,
		loading = false,
		error = nil,
	}
	set_quote(state, 1)
	return state
end

function M.update(state, delta_ms)
	state.last_change = state.last_change + delta_ms

	if state.last_change >= state.change_interval then
		state.last_change = 0
		local next_index = (state.quote_index % #builtin_quotes) + 1
		set_quote(state, next_index)
	end
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
	local px, py = 25, 20

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	if state.loading then
		components.loading(gfx, px, state.height / 2 - 10)
		return
	end

	if not state.quote_text then
		gfx:text(px, state.height / 2 - 10, "No quote available", th.text_muted, "medium")
		return
	end

	-- Opening quotation mark (decorative)
	gfx:text(px - 5, py + 10, '"', th.accent_primary, "xlarge")

	-- Calculate text layout
	local chars_per_line = floor((state.width - px * 2 - 20) / 8)
	local lines = wrap_text(state.quote_text, chars_per_line)

	local line_height = 22
	local text_start_y = py + 20
	local max_lines = floor((state.height - text_start_y - 50) / line_height)

	-- Draw quote text
	local num_lines = #lines
	for i = 1, num_lines do
		if i > max_lines then
			gfx:text(px + 15, text_start_y + (max_lines - 1) * line_height, "...", th.text_primary, "medium")
			break
		end
		gfx:text(px + 15, text_start_y + (i - 1) * line_height, lines[i], th.text_primary, "medium")
	end

	-- Author attribution
	if state.quote_author then
		local author_y = state.height - py - 15
		gfx:text(px + 15, author_y, "— " .. state.quote_author, th.text_accent, "medium")
	end

	-- Subtle accent line
	gfx:line(px, state.height - py - 35, px + 3, state.height - py - 35, th.accent_primary, 2)
end

function M.on_event(state, event)
	if event.type == "tap" then
		-- Show next quote by resetting the timer
		state.last_change = state.change_interval
		return true
	end
	return false
end

return M
