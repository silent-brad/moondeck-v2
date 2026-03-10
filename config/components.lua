-- Moondeck Component Library
-- Reusable UI components inspired by TRMNL's design system

local theme = require("theme")

local Components = {}

-- Safe math functions (avoids upvalue issues with piccolo)
local function floor(n)
	if math and math.floor then
		return math.floor(n)
	end
	local i = n - (n % 1)
	if n < 0 and i ~= n then
		return i - 1
	end
	return i
end

local function max(a, b)
	if math and math.max then
		return math.max(a, b)
	end
	return a > b and a or b
end

local function min(a, b)
	if math and math.min then
		return math.min(a, b)
	end
	return a < b and a or b
end

-- Default theme colors fallback
local default_colors = {
	bg_primary = "#0a0a0f",
	bg_secondary = "#12121a",
	bg_tertiary = "#1a1a2e",
	bg_card = "#16162a",
	text_primary = "#ffffff",
	text_secondary = "#a0a0b0",
	text_muted = "#606070",
	text_accent = "#00d4ff",
	accent_primary = "#00d4ff",
	accent_secondary = "#e94560",
	accent_success = "#00ff88",
	accent_warning = "#ffaa00",
	accent_error = "#ff4466",
	border_primary = "#2a2a3e",
	border_accent = "#00d4ff",
	card_radius = 12,
	border_width = 1,
}

-- Get current theme with fallback
local function t()
	if theme and theme.get then
		local result = theme:get()
		if result then
			return result
		end
	end
	return default_colors
end

-- Card component: rounded rectangle with border (uses theme defaults)
function Components.card(gfx, x, y, w, h)
	if not gfx then
		return
	end

	local th = t()

	local bg = th.bg_card or "#16162a"
	local radius = th.card_radius or 12
	local border = th.border_primary or "#2a2a3e"
	local border_width = th.border_width or 1

	-- Draw background
	if gfx.fill_rounded_rect then
		gfx:fill_rounded_rect(x, y, w, h, radius, bg)
	end

	-- Draw border
	if gfx.stroke_rounded_rect then
		gfx:stroke_rounded_rect(x, y, w, h, radius, border, border_width)
	end
end

-- Title bar component
function Components.title_bar(gfx, x, y, w, title, opts)
	if not gfx then
		return 35
	end
	opts = opts or {}
	local th = t()

	local color = opts.color or th.text_primary or "#ffffff"
	local accent = opts.accent or th.accent_primary or "#00d4ff"
	local font = opts.font or "large"
	local show_line = opts.show_line ~= false

	-- Draw title
	if gfx.text then
		gfx:text(x, y + 20, title, color, font)
	end

	-- Draw accent line
	if show_line and gfx.line then
		local line_y = y + 48
		gfx:line(x, line_y, x + w, line_y, accent, 2)
	end

	return 35 -- Return height consumed
end

-- Value display: large number with label
function Components.value_display(gfx, x, y, value, label, opts)
	opts = opts or {}
	local th = t()

	local value_color = opts.value_color or th.text_primary
	local label_color = opts.label_color or th.text_muted
	local value_font = opts.value_font or "xlarge"
	local label_font = opts.label_font or "small"
	local unit = opts.unit or ""

	-- Draw value
	gfx:text(x, y, tostring(value) .. unit, value_color, value_font)

	-- Draw label below
	if label then
		gfx:text(x, y + 28, label, label_color, label_font)
	end

	return 45 -- Return height consumed
end

-- Item row: icon/indicator + label + value
function Components.item_row(gfx, x, y, w, label, value, opts)
	opts = opts or {}
	local th = t()

	local label_color = opts.label_color or th.text_secondary
	local value_color = opts.value_color or th.text_primary
	local indicator_color = opts.indicator or nil
	local font = opts.font or "medium"

	local text_x = x

	-- Draw indicator dot if specified
	if indicator_color then
		gfx:fill_circle(x + 4, y + 6, 4, indicator_color)
		text_x = x + 16
	end

	-- Draw label
	gfx:text(text_x, y, label, label_color, font)

	-- Draw value (right-aligned)
	local value_str = tostring(value)
	local value_width = #value_str * 8 -- Approximate
	gfx:text(x + w - value_width, y, value_str, value_color, font)

	return 20 -- Return height consumed
end

-- Divider line
function Components.divider(gfx, x, y, w, opts)
	opts = opts or {}
	local th = t()

	local color = opts.color or th.border_primary
	local thickness = opts.thickness or 1

	gfx:line(x, y, x + w, y, color, thickness)

	return thickness + 8 -- Return height consumed with padding
end

-- Progress bar
function Components.progress_bar(gfx, x, y, w, h, progress, opts)
	opts = opts or {}
	local th = t()

	local bg = opts.bg or th.bg_tertiary
	local fg = opts.fg or th.accent_primary
	local radius = opts.radius or (h / 2)

	-- Clamp progress to 0-1
	progress = max(0, min(1, progress))

	-- Draw background
	gfx:fill_rounded_rect(x, y, w, h, radius, bg)

	-- Draw fill
	if progress > 0 then
		local fill_w = max(h, w * progress)
		gfx:fill_rounded_rect(x, y, fill_w, h, radius, fg)
	end

	return h + 8 -- Return height consumed
end

-- Status indicator (dot + text)
function Components.status(gfx, x, y, text, status, opts)
	opts = opts or {}
	local th = t()

	local colors = {
		ok = th.accent_success,
		warning = th.accent_warning,
		error = th.accent_error,
		info = th.accent_primary,
	}

	local dot_color = colors[status] or th.text_muted
	local text_color = opts.text_color or th.text_secondary
	local font = opts.font or "small"

	-- Draw status dot
	gfx:fill_circle(x + 4, y + 5, 4, dot_color)

	-- Draw text
	gfx:text(x + 14, y, text, text_color, font)

	return 16 -- Return height consumed
end

-- Mini chart (sparkline)
function Components.sparkline(gfx, x, y, w, h, data, opts)
	opts = opts or {}
	local th = t()

	local color = opts.color or th.accent_primary
	local thickness = opts.thickness or 2

	if not data or #data < 2 then
		return h
	end

	-- Find min/max for scaling
	local min_val, max_val = data[1], data[1]
	for _, v in ipairs(data) do
		if v < min_val then
			min_val = v
		end
		if v > max_val then
			max_val = v
		end
	end

	local range = max_val - min_val
	if range == 0 then
		range = 1
	end

	-- Draw lines
	local step = w / (#data - 1)
	for i = 1, #data - 1 do
		local x1 = x + (i - 1) * step
		local y1 = y + h - ((data[i] - min_val) / range * h)
		local x2 = x + i * step
		local y2 = y + h - ((data[i + 1] - min_val) / range * h)

		gfx:line(floor(x1), floor(y1), floor(x2), floor(y2), color, thickness)
	end

	return h
end

-- Icon placeholder (circle with letter)
function Components.icon(gfx, x, y, size, letter, opts)
	opts = opts or {}
	local th = t()

	local bg = opts.bg or th.accent_primary
	local fg = opts.fg or th.bg_primary

	local radius = size / 2
	gfx:fill_circle(x + radius, y + radius, radius, bg)
	gfx:text(x + radius - 4, y + radius - 6, letter, fg, "medium")

	return size
end

-- Loading indicator
function Components.loading(gfx, x, y, text)
	local th = t()
	text = text or "Loading..."
	gfx:text(x, y, text, th.text_muted, "medium")
	return 20
end

-- Error display
function Components.error(gfx, x, y, w, message)
	local th = t()
	gfx:text(x, y, "Error", th.accent_error, "medium")
	gfx:text(x, y + 18, message or "Unknown error", th.text_muted, "small")
	return 40
end

return Components
