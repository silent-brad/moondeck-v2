-- GitHub Widget
-- Displays contribution heatmap, latest commits, and language breakdown

local fetch = require("widgets.github.fetch")
local render = require("widgets.github.render")

local M = {}

function M.init(ctx)
  local fetch_interval = ctx.opts.update_interval or 3600000

  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    username = ctx.opts.username or env.get("GITHUB_USERNAME") or "",
    weeks = {},
    total = 0,
    commit_repos = {},
    commit_msgs = {},
    commit_dates = {},
    commit_lines = {},
    commit_langs = {},
    commit_count = 0,
    lang_names = {},
    lang_pcts = {},
    lang_count = 0,
    repo_names = {},
    repo_descs = {},
    repo_visibilities = {},
    repo_pushed = {},
    repo_lang_names = {},
    repo_lang_pcts = {},
    repo_lang_colors = {},
    repo_lang_counts = {},
    repo_count = 0,
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
    local ok, err = fetch.fetch(state)
    if ok then
      state.error = nil
    else
      state.error = err
    end
    state.loading = false
  end
end

function M.render(state, gfx)
  render.render(state, gfx)
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
