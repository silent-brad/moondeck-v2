-- Moondeck Layout System
-- Grid-based layout inspired by TRMNL's framework

local theme = require("theme")

local Layout = {}

-- Calculate column width based on grid settings
function Layout.col_width(cols, gutter, margin, total_cols)
	gutter = gutter or theme.grid.gutter
	margin = margin or theme.grid.margin
	total_cols = total_cols or theme.grid.columns

	local available_width = theme.screen.width - (margin * 2) - (gutter * (total_cols - 1))
	local single_col = available_width / total_cols
	return (single_col * cols) + (gutter * (cols - 1))
end

-- Calculate x position for a column start
function Layout.col_x(start_col, gutter, margin, total_cols)
	gutter = gutter or theme.grid.gutter
	margin = margin or theme.grid.margin
	total_cols = total_cols or theme.grid.columns

	local available_width = theme.screen.width - (margin * 2) - (gutter * (total_cols - 1))
	local single_col = available_width / total_cols
	return margin + ((start_col - 1) * (single_col + gutter))
end

-- Predefined layout templates
Layout.templates = {
	-- Full screen single widget
	full = {
		{ col = 1, span = 12, row = 1, row_span = 1 },
	},

	-- Two equal columns
	half_half = {
		{ col = 1, span = 6, row = 1, row_span = 1 },
		{ col = 7, span = 6, row = 1, row_span = 1 },
	},

	-- Three equal columns
	thirds = {
		{ col = 1, span = 4, row = 1, row_span = 1 },
		{ col = 5, span = 4, row = 1, row_span = 1 },
		{ col = 9, span = 4, row = 1, row_span = 1 },
	},

	-- Large left, small right stack
	main_sidebar = {
		{ col = 1, span = 8, row = 1, row_span = 2 },
		{ col = 9, span = 4, row = 1, row_span = 1 },
		{ col = 9, span = 4, row = 2, row_span = 1 },
	},

	-- Header + two columns below
	header_two_col = {
		{ col = 1, span = 12, row = 1, row_span = 1, height_ratio = 0.35 },
		{ col = 1, span = 6, row = 2, row_span = 1, height_ratio = 0.65 },
		{ col = 7, span = 6, row = 2, row_span = 1, height_ratio = 0.65 },
	},

	-- Two rows, two columns (quad)
	quad = {
		{ col = 1, span = 6, row = 1, row_span = 1 },
		{ col = 7, span = 6, row = 1, row_span = 1 },
		{ col = 1, span = 6, row = 2, row_span = 1 },
		{ col = 7, span = 6, row = 2, row_span = 1 },
	},

	-- Dashboard: main + 3 small
	dashboard = {
		{ col = 1, span = 8, row = 1, row_span = 2 },
		{ col = 9, span = 4, row = 1, row_span = 1 },
		{ col = 9, span = 4, row = 2, row_span = 1 },
	},

	-- Info cards: 4 equal cards
	cards_4 = {
		{ col = 1, span = 3, row = 1, row_span = 1 },
		{ col = 4, span = 3, row = 1, row_span = 1 },
		{ col = 7, span = 3, row = 1, row_span = 1 },
		{ col = 10, span = 3, row = 1, row_span = 1 },
	},
}

-- Calculate widget bounds from grid position
function Layout.calculate_bounds(slot, rows, gutter, margin)
	rows = rows or 2
	gutter = gutter or theme.grid.gutter
	margin = margin or theme.grid.margin

	local x = Layout.col_x(slot.col, gutter, margin)
	local w = Layout.col_width(slot.span, gutter, margin)

	-- Calculate row heights
	local available_height = theme.screen.height - (margin * 2) - (gutter * (rows - 1))
	local row_height = available_height / rows

	-- Apply height ratio if specified
	local h
	if slot.height_ratio then
		h = (theme.screen.height - (margin * 2)) * slot.height_ratio - (gutter / 2)
	else
		h = (row_height * (slot.row_span or 1)) + (gutter * ((slot.row_span or 1) - 1))
	end

	local y = margin + ((slot.row - 1) * (row_height + gutter))

	return {
		x = math.floor(x),
		y = math.floor(y),
		w = math.floor(w),
		h = math.floor(h),
	}
end

-- Apply a layout template to generate widget positions
function Layout.apply_template(template_name, rows)
	local template = Layout.templates[template_name]
	if not template then
		return nil
	end

	rows = rows or 2
	local positions = {}

	for i, slot in ipairs(template) do
		positions[i] = Layout.calculate_bounds(slot, rows)
	end

	return positions
end

return Layout
