-- GitHub Widget: Language colors and heatmap color helpers

local M = {}

-- GitHub language colors (integer RGB values)
M.lang_colors = {
  ["Rust"] = 0xDEA584,
  ["JavaScript"] = 0xF1E05A,
  ["TypeScript"] = 0x3178C6,
  ["Python"] = 0x3572A5,
  ["Go"] = 0x00ADD8,
  ["Java"] = 0xB07219,
  ["C"] = 0x555555,
  ["C++"] = 0xF34B7D,
  ["C#"] = 0x178600,
  ["Ruby"] = 0x701516,
  ["PHP"] = 0x4F5D95,
  ["Swift"] = 0xF05138,
  ["Kotlin"] = 0xA97BFF,
  ["Dart"] = 0x00B4AB,
  ["Lua"] = 0x000080,
  ["Shell"] = 0x89E051,
  ["Bash"] = 0x89E051,
  ["Nu"] = 0xC5C5C5,
  ["HTML"] = 0xE34C26,
  ["CSS"] = 0x563D7C,
  ["SCSS"] = 0xC6538C,
  ["Vue"] = 0x41B883,
  ["Svelte"] = 0xFF3E00,
  ["Haskell"] = 0x5E5086,
  ["Elixir"] = 0x6E4A7E,
  ["Scala"] = 0xC22D40,
  ["Clojure"] = 0xDB5855,
  ["Scheme"] = 0x1E4AEB,
  ["Zig"] = 0xEC915C,
  ["Nix"] = 0x7E7EFF,
  ["OCaml"] = 0xEE6A1A,
  ["Vim Script"] = 0x199F4B,
  ["Dockerfile"] = 0x384D54,
  ["Makefile"] = 0x427819,
  ["Jupyter Notebook"] = 0xDA5B0B,
}

function M.get_lang_color(name)
  return M.lang_colors[name]
end

-- Hex color parsing and mixing

local hex_digits = {
  ["0"] = 0,
  ["1"] = 1,
  ["2"] = 2,
  ["3"] = 3,
  ["4"] = 4,
  ["5"] = 5,
  ["6"] = 6,
  ["7"] = 7,
  ["8"] = 8,
  ["9"] = 9,
  ["a"] = 10,
  ["b"] = 11,
  ["c"] = 12,
  ["d"] = 13,
  ["e"] = 14,
  ["f"] = 15,
  ["A"] = 10,
  ["B"] = 11,
  ["C"] = 12,
  ["D"] = 13,
  ["E"] = 14,
  ["F"] = 15,
}

local function hex2(s, pos)
  local hi = hex_digits[string.sub(s, pos, pos)] or 0
  local lo = hex_digits[string.sub(s, pos + 1, pos + 1)] or 0
  return hi * 16 + lo
end

local function hex_to_rgb(hex)
  return { hex2(hex, 2), hex2(hex, 4), hex2(hex, 6) }
end

local function lerp(a, b, t)
  return math.floor(a + (b - a) * t)
end

local function mix(c1, c2, t)
  local a = hex_to_rgb(c1)
  local b = hex_to_rgb(c2)
  return lerp(a[1], b[1], t) * 65536 + lerp(a[2], b[2], t) * 256 + lerp(a[3], b[3], t)
end

function M.make_heat_colors(th)
  local base = th.accent_secondary
  local target = th.accent_success
  return {
    mix(base, target, 0),
    mix(base, target, 0.5),
    mix(base, target, 0.75),
    mix(base, target, 0.9),
    mix(base, target, 1),
  }
end

function M.count_to_color(count, heat_colors)
  if count == 0 then
    return heat_colors[1]
  elseif count <= 3 then
    return heat_colors[2]
  elseif count <= 6 then
    return heat_colors[3]
  elseif count <= 9 then
    return heat_colors[4]
  else
    return heat_colors[5]
  end
end

return M
