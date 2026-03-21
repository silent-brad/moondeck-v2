-- GitHub Widget: Language colors and heatmap color helpers

local color_util = require("utils.color")

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
  ["Nim"] = 0xFFDF00,
  ["Jinja"] = 0xFF0000,
  ["Vim Script"] = 0x199F4B,
  ["Dockerfile"] = 0x384D54,
  ["Makefile"] = 0x427819,
  ["Jupyter Notebook"] = 0xDA5B0B,
  ["Markdown"] = 0x000000,
  ["ORG"] = 0x990000,
  ["JSON"] = 0x292929,
  ["YAML"] = 0x000000,
  ["TOML"] = 0x000000,
  ["XML"] = 0x0060AC,
  ["GraphQL"] = 0xE10098,
  ["SQL"] = 0x0099D6,
  ["PostgreSQL"] = 0x336791,
  ["MySQL"] = 0x4479A1,
  ["MSSQL"] = 0xCC2927,
  ["PL/pgSQL"] = 0x336791,
  ["SCALA"] = 0xCC2927,
}

function M.get_lang_color(name)
  return M.lang_colors[name]
end

function M.make_heat_colors(th)
  local base = th.accent_secondary
  local target = th.accent_success
  return {
    color_util.mix(base, target, 0),
    color_util.mix(base, target, 0.5),
    color_util.mix(base, target, 0.75),
    color_util.mix(base, target, 0.9),
    color_util.mix(base, target, 1),
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
