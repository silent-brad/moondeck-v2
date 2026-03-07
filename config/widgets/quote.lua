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
	{ text = "The future belongs to those who believe in the beauty of their dreams.", author = "Eleanor Roosevelt" },
	{ text = "It is during our darkest moments that we must focus to see the light.", author = "Aristotle" },
	{ text = "The only thing we have to fear is fear itself.", author = "Franklin D. Roosevelt" },
	{ text = "In the middle of difficulty lies opportunity.", author = "Albert Einstein" },
	{ text = "Life is what happens when you're busy making other plans.", author = "John Lennon" },
	{ text = "The purpose of our lives is to be happy.", author = "Dalai Lama" },
	{ text = "Get busy living or get busy dying.", author = "Stephen King" },
	{ text = "You only live once, but if you do it right, once is enough.", author = "Mae West" },
	{
		text = "Many of life's failures are people who did not realize how close they were to success when they gave up.",
		author = "Thomas Edison",
	},
	{ text = "The mind is everything. What you think you become.", author = "Buddha" },
	{
		text = "The best time to plant a tree was 20 years ago. The second best time is now.",
		author = "Chinese Proverb",
	},
	{ text = "An unexamined life is not worth living.", author = "Socrates" },
	{ text = "Simplicity is the ultimate sophistication.", author = "Leonardo da Vinci" },
	{ text = "The only true wisdom is in knowing you know nothing.", author = "Socrates" },
	{ text = "Do what you can, with what you have, where you are.", author = "Theodore Roosevelt" },
	{ text = "Everything you've ever wanted is on the other side of fear.", author = "George Addair" },
	{
		text = "Success is not final, failure is not fatal: it is the courage to continue that counts.",
		author = "Winston Churchill",
	},
}

function M.init(ctx)
	return {
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
end

function M.update(state, delta_ms)
	state.last_change = state.last_change + delta_ms

	if state.last_change >= state.change_interval or state.quote_text == nil then
		M.next_quote(state)
		state.last_change = 0
	end
end

function M.next_quote(state)
	if state.use_api then
		M.fetch_quote_api(state)
	else
		M.get_local_quote(state)
	end
end

function M.get_local_quote(state)
	-- Use device time to pick a "random" quote
	local seed = device.seconds()
	state.quote_index = (seed % #builtin_quotes) + 1

	local quote = builtin_quotes[state.quote_index]
	state.quote_text = quote.text
	state.quote_author = quote.author
	state.error = nil
end

function M.fetch_quote_api(state)
	local api_url = env.get("QUOTES_API_URL")
	if not api_url then
		-- Fall back to local quotes
		M.get_local_quote(state)
		return
	end

	state.loading = true

	local response = net.http_get(api_url, nil, 10000)

	if response.ok then
		local data = net.json_decode(response.body)
		if data then
			-- Support various API formats
			state.quote_text = data.content or data.quote or data.text or data.q
			state.quote_author = data.author or data.a or "Unknown"
			state.error = nil
		else
			M.get_local_quote(state) -- Fallback
		end
	else
		M.get_local_quote(state) -- Fallback
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
	local px, py = 25, 20

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height, {
		bg = th.bg_card,
		border = th.border_primary,
	})

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
	local chars_per_line = math.floor((state.width - px * 2 - 20) / 8)
	local lines = wrap_text(state.quote_text, chars_per_line)

	local line_height = 22
	local text_start_y = py + 20
	local max_lines = math.floor((state.height - text_start_y - 50) / line_height)

	-- Draw quote text
	for i, line in ipairs(lines) do
		if i > max_lines then
			-- Show ellipsis
			gfx:text(px + 15, text_start_y + (max_lines - 1) * line_height, "...", th.text_primary, "medium")
			break
		end
		gfx:text(px + 15, text_start_y + (i - 1) * line_height, line, th.text_primary, "medium")
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
		-- Show next quote
		M.next_quote(state)
		return true
	end
	return false
end

return M
