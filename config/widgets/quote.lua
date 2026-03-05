-- Quote Widget
-- Displays an inspirational quote

local M = {}

local quotes = {
	{ text = "The only way to do great work is to love what you do.", author = "Steve Jobs" },
	{ text = "Innovation distinguishes between a leader and a follower.", author = "Steve Jobs" },
	{ text = "Stay hungry, stay foolish.", author = "Steve Jobs" },
	{ text = "Code is like humor. When you have to explain it, it's bad.", author = "Cory House" },
	{ text = "First, solve the problem. Then, write the code.", author = "John Johnson" },
	{ text = "Simplicity is the soul of efficiency.", author = "Austin Freeman" },
	{ text = "Make it work, make it right, make it fast.", author = "Kent Beck" },
	{ text = "Any fool can write code that a computer can understand.", author = "Martin Fowler" },
}

function M.init(ctx)
	local idx = (math.floor(device.seconds() / 60) % #quotes) + 1
	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		quote_index = idx,
		current_quote = quotes[idx],
	}
end

function M.update(state, delta_ms)
	-- Change quote periodically based on time
	local idx = (math.floor(device.seconds() / 60) % #quotes) + 1
	if idx ~= state.quote_index then
		state.quote_index = idx
		state.current_quote = quotes[idx]
	end
end

function M.render(state, gfx)
	-- Draw background panel
	gfx:fill_rounded_rect(0, 0, state.width, state.height, 12, "#0f3460")

	-- Quote icon/decoration
	gfx:text(15, 40, '"', "#e94560", "xlarge")

	-- Quote text (simple word wrap simulation)
	local quote = state.current_quote
	if quote then
		local text = quote.text
		local max_chars = math.floor(state.width / 8) - 4

		if #text > max_chars then
			-- Simple line break
			local line1 = text:sub(1, max_chars)
			local space = line1:reverse():find(" ")
			if space then
				line1 = text:sub(1, max_chars - space + 1)
			end
			local line2 = text:sub(#line1 + 1):gsub("^%s+", "")

			gfx:text(40, 70, line1, "white", "medium")
			gfx:text(40, 95, line2, "white", "medium")
		else
			gfx:text(40, 80, text, "white", "medium")
		end

		-- Author
		gfx:text(state.width - 150, state.height - 40, "— " .. quote.author, "#888888", "small")
	end

	-- Border
	gfx:stroke_rect(5, 5, state.width - 10, state.height - 10, "#e94560", 1)
end

function M.on_event(state, event)
	if event.type == "tap" then
		-- Cycle to next quote on tap
		state.quote_index = (state.quote_index % #quotes) + 1
		state.current_quote = quotes[state.quote_index]
		return true
	end
	return false
end

return M
