-- Moondeck Theme Configuration
-- Inspired by TRMNL's clean, focused design philosophy

local themes = {}

-- Dark theme (default)
themes.dark = {
	name = "dark",

	-- Background colors
	bg_primary = "#0a0a0f",
	bg_secondary = "#12121a",
	bg_tertiary = "#1a1a2e",
	bg_card = "#16162a",

	-- Text colors
	text_primary = "#ffffff",
	text_secondary = "#a0a0b0",
	text_muted = "#606070",
	text_accent = "#00d4ff",

	-- Accent colors
	accent_primary = "#00d4ff",
	accent_secondary = "#e94560",
	accent_success = "#00ff88",
	accent_warning = "#ffaa00",
	accent_error = "#ff4466",

	-- Border colors
	border_primary = "#2a2a3e",
	border_accent = "#00d4ff",

	-- Component specific
	card_radius = 12,
	border_width = 1,
}

-- Light theme
themes.light = {
	name = "light",

	-- Background colors
	bg_primary = "#ffffff",
	bg_secondary = "#f5f5f7",
	bg_tertiary = "#e8e8ec",
	bg_card = "#ffffff",

	-- Text colors
	text_primary = "#1a1a1a",
	text_secondary = "#4a4a4a",
	text_muted = "#8a8a8a",
	text_accent = "#0066cc",

	-- Accent colors
	accent_primary = "#0066cc",
	accent_secondary = "#cc3366",
	accent_success = "#00aa55",
	accent_warning = "#cc8800",
	accent_error = "#cc2244",

	-- Border colors
	border_primary = "#d0d0d8",
	border_accent = "#0066cc",

	-- Component specific
	card_radius = 12,
	border_width = 1,
}

-- Mint theme (dark variant)
themes.mint = {
	name = "mint",

	-- Background colors
	bg_primary = "#0d1f1a",
	bg_secondary = "#122a22",
	bg_tertiary = "#1a3830",
	bg_card = "#15302a",

	-- Text colors
	text_primary = "#e8fff4",
	text_secondary = "#a0d4c0",
	text_muted = "#608070",
	text_accent = "#00ffaa",

	-- Accent colors
	accent_primary = "#00ffaa",
	accent_secondary = "#00d4ff",
	accent_success = "#00ff88",
	accent_warning = "#ffcc00",
	accent_error = "#ff6666",

	-- Border colors
	border_primary = "#2a4a40",
	border_accent = "#00ffaa",

	-- Component specific
	card_radius = 12,
	border_width = 1,
}

-- Typography configuration
local typography = {
	-- Font families (referenced by name, resolved by renderer)
	font_display = "garamond", -- EB Garamond for headings
	font_body = "inter", -- Inter for body text
	font_mono = "mono", -- Monospace for data

	-- Font sizes (in pixels)
	size_xs = 10,
	size_sm = 12,
	size_md = 14,
	size_lg = 18,
	size_xl = 24,
	size_2xl = 32,
	size_3xl = 48,
	size_4xl = 64,

	-- Line heights
	line_tight = 1.1,
	line_normal = 1.4,
	line_relaxed = 1.6,

	-- Font weights
	weight_normal = 400,
	weight_medium = 500,
	weight_bold = 700,
}

-- Spacing scale (8px base)
local spacing = {
	xs = 4,
	sm = 8,
	md = 16,
	lg = 24,
	xl = 32,
	xxl = 48,
}

-- Grid configuration for layout system
local grid = {
	columns = 12,
	gutter = 16,
	margin = 20,
}

-- Screen dimensions
local screen = {
	width = 800,
	height = 480,
}

-- Export the theme module
return {
	themes = themes,
	typography = typography,
	spacing = spacing,
	grid = grid,
	screen = screen,

	-- Current active theme (can be changed)
	current = "dark",

	-- Helper to get current theme
	get = function(self)
		return self.themes[self.current] or self.themes.dark
	end,

	-- Helper to set theme
	set = function(self, name)
		if self.themes[name] then
			self.current = name
			return true
		end
		return false
	end,
}
