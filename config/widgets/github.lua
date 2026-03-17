-- GitHub Heatmap Widget
-- Displays a GitHub-style contribution calendar using the GraphQL API

local M = {}

function M.init(ctx)
	local fetch_interval = ctx.opts.update_interval or 3600000 -- 1 hour

	return {
		x = ctx.x,
		y = ctx.y,
		width = ctx.width,
		height = ctx.height,
		username = ctx.opts.username or env.get("GITHUB_USERNAME") or "",
		weeks = {},
		total = 0,
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

		local token = env.get("GITHUB_TOKEN")
		if not token then
			state.error = "No GITHUB_TOKEN"
			state.loading = false
			return
		end

		if state.username == "" then
			state.error = "No username"
			state.loading = false
			return
		end

		local query = '{"query":"query{user(login:\\"'
			.. state.username
			.. '\\"){contributionsCollection{contributionCalendar{totalContributions weeks{contributionDays{contributionCount date}}}}}}"}'

		local headers = {
			Authorization = "Bearer " .. token,
			["User-Agent"] = "moondeck",
		}

		local response = net.http_post("https://api.github.com/graphql", query, "application/json", headers, 15000)

		if response and response.ok and response.body then
			local data = net.json_decode(response.body)

			if data and data.data and data.data.user then
				local cal = data.data.user.contributionsCollection.contributionCalendar
				state.total = cal.totalContributions or 0
				state.weeks = cal.weeks or {}
				state.loading = false
				state.error = nil
			else
				state.error = "User not found"
				state.loading = false
			end
		else
			state.error = response and response.error or "Network error"
			state.loading = false
		end
	end
end

-- Map contribution count to a color intensity
local function count_to_color(count)
	if count == 0 then
		return "#161b22"
	elseif count <= 3 then
		return "#0e4429"
	elseif count <= 6 then
		return "#006d32"
	elseif count <= 9 then
		return "#26a641"
	else
		return "#39d353"
	end
end

function M.render(state, gfx)
	local th = theme:get()
	local px, py = 20, 15

	-- Draw card
	components.card(gfx, 0, 0, state.width, state.height)

	-- Title bar
	local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "GitHub", {
		accent = th.accent_success,
	})

	local content_y = py + title_h + 15

	if state.loading then
		components.loading(gfx, px, content_y + 20)
		return
	end

	if state.error then
		components.error(gfx, px, content_y + 10, state.width - px * 2, state.error)
		return
	end

	-- Username and total contributions
	gfx:text(px, content_y, "@" .. state.username, th.text_accent, "medium")

	local total_str = tostring(state.total) .. " contributions"
	gfx:text(px, content_y + 18, total_str, th.text_muted, "small")

	-- Draw heatmap grid
	local grid_y = content_y + 40
	local available_w = state.width - px * 2
	local available_h = state.height - grid_y - py - 10

	-- 7 rows (days of week), up to 52 columns (weeks)
	local num_weeks = #state.weeks
	if num_weeks == 0 then
		gfx:text(px, grid_y, "No data", th.text_muted, "small")
		return
	end

	-- Calculate cell size to fit the available space
	local gap = 2
	local cell_w = math.floor((available_w - (num_weeks - 1) * gap) / num_weeks)
	local cell_h = math.floor((available_h - 6 * gap) / 7)

	-- Clamp cell size to keep squares
	local cell = math.min(cell_w, cell_h)
	cell = math.max(cell, 2) -- minimum 2px
	cell = math.min(cell, 12) -- maximum 12px

	-- Recalculate how many weeks we can fit
	local max_weeks = math.floor((available_w + gap) / (cell + gap))
	local start_week = 1
	if num_weeks > max_weeks then
		start_week = num_weeks - max_weeks + 1
	end

	-- Draw cells
	for wi = start_week, num_weeks do
		local week = state.weeks[wi]
		local col = wi - start_week
		local cx = px + col * (cell + gap)

		if week and week.contributionDays then
			for di = 1, #week.contributionDays do
				local day = week.contributionDays[di]
				local cy = grid_y + (di - 1) * (cell + gap)
				local color = count_to_color(day.contributionCount or 0)
				gfx:fill_rounded_rect(cx, cy, cell, cell, 0, color)
			end
		end
	end

	-- Legend
	local legend_y = grid_y + 7 * (cell + gap) + 4
	if legend_y + 10 < state.height - py then
		gfx:text(px, legend_y, "Less", th.text_muted, "small")
		local lx = px + 30
		local legend_colors = { "#161b22", "#0e4429", "#006d32", "#26a641", "#39d353" }
		for i = 1, #legend_colors do
			gfx:fill_rounded_rect(lx + (i - 1) * (cell + gap), legend_y, cell, cell, 0, legend_colors[i])
		end
		gfx:text(lx + 5 * (cell + gap) + 4, legend_y, "More", th.text_muted, "small")
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
