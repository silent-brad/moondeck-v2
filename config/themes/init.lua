-- Moondeck Theme System

local themes = {
  dark = require("themes.dark"),
  light = require("themes.light"),
  mint = require("themes.mint"),
  rose_pine = require("themes.rose_pine"),
}

-- Typography configuration
local typography = {
  font_display = "eb garamond",
  font_body = "inter",
  font_mono = "mono",
  size_xs = 10,
  size_sm = 12,
  size_md = 14,
  size_lg = 18,
  size_xl = 24,
  size_2xl = 32,
  size_3xl = 48,
  size_4xl = 64,
  line_tight = 1.1,
  line_normal = 1.4,
  line_relaxed = 1.6,
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

return {
  themes = themes,
  typography = typography,
  spacing = spacing,
  grid = grid,
  screen = screen,
  current = "rose_pine",

  get = function(self)
    return self.themes[self.current] or self.themes.dark
  end,

  set = function(self, name)
    if self.themes[name] then
      self.current = name
      return true
    end
    return false
  end,
}
