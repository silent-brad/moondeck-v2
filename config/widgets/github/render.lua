-- GitHub Widget: Rendering

local colors = require("widgets.github.colors")
local fetch_mod = require("widgets.github.fetch")

local M = {}

local function render_heatmap(gfx, state, th, px, grid_y, available_w, available_h, heat_colors)
  local num_weeks = #state.weeks
  if num_weeks == 0 then
    return grid_y
  end

  local gap = 2
  local cell_w = math.floor((available_w - (num_weeks - 1) * gap) / num_weeks)
  local cell_h = math.floor((available_h - 6 * gap) / 7)
  local cell = math.min(cell_w, cell_h)
  cell = math.max(cell, 2)
  cell = math.min(cell, 10)

  local max_weeks = math.floor((available_w + gap) / (cell + gap))
  local start_week = 1
  if num_weeks > max_weeks then
    start_week = num_weeks - max_weeks + 1
  end

  for wi = start_week, num_weeks do
    local week = state.weeks[wi]
    local col = wi - start_week
    local cx = px + col * (cell + gap)

    if week and week.contributionDays then
      for di = 1, #week.contributionDays do
        local day = week.contributionDays[di]
        local dy = grid_y + (di - 1) * (cell + gap)
        local color = colors.count_to_color(day.contributionCount or 0, heat_colors)
        gfx:fill_rounded_rect(cx, dy, cell, cell, 0, color)
      end
    end
  end

  -- Legend
  local legend_y = grid_y + 7 * (cell + gap) + 4
  local py = 15
  if legend_y + 10 < state.height - py then
    gfx:text(px, legend_y, "Less", th.text_muted, "small")
    local lx = px + 30
    for i = 1, #heat_colors do
      gfx:fill_rounded_rect(lx + (i - 1) * (cell + gap), legend_y, cell, cell, 0, heat_colors[i])
    end
    gfx:text(lx + 5 * (cell + gap) + 4, legend_y, "More", th.text_muted, "small")
  end

  return legend_y + 20
end

local function render_repos(gfx, state, th, px, repo_y, repo_w)
  if state.repo_count == 0 then
    return
  end

  local py = 15
  local repo_h = 58

  for i = 1, state.repo_count do
    if repo_y + repo_h > state.height - py then
      break
    end

    local rname = state.repo_names[i] or ""
    local rdesc = state.repo_descs[i] or ""
    local rvis = state.repo_visibilities[i] or "PUBLIC"
    local rdate = state.repo_pushed[i] or ""

    if #rname > 28 then
      rname = string.sub(rname, 1, 25) .. "..."
    end

    -- Name and visibility badge
    gfx:text(px, repo_y, rname, th.text_accent, "small")
    local vis_label = rvis == "PRIVATE" and "Private" or "Public"
    local vis_color = rvis == "PRIVATE" and th.accent_error or th.accent_success
    local vis_w = #vis_label * 6
    gfx:text(px + repo_w - vis_w, repo_y, vis_label, vis_color, "small")

    -- Description
    if #rdesc > 50 then
      rdesc = string.sub(rdesc, 1, 47) .. "..."
    end
    gfx:text(px, repo_y + 14, rdesc, th.text_muted, "small")

    -- Pushed date
    local date_str = fetch_mod.short_date(rdate)
    if date_str ~= "" then
      gfx:text(px + repo_w - (#date_str * 6), repo_y + 14, date_str, th.text_muted, "small")
    end

    -- Language bar and labels
    local lcount = state.repo_lang_counts[i] or 0
    if lcount > 0 then
      local bar_y = repo_y + 34
      local bar_h = 5
      local bar_w = repo_w - 10

      local cx = px
      local label_y = bar_y + bar_h + 4
      local lx = px
      local labels_drawn = 0

      local ln_names = state.repo_lang_names[i] or {}
      local ln_pcts = state.repo_lang_pcts[i] or {}
      local ln_colors = state.repo_lang_colors[i] or {}

      for k = 1, lcount do
        local ln = ln_names[k] or ""
        local lpct = ln_pcts[k] or 0
        local lclr = ln_colors[k] or th.text_muted

        local seg_w = math.floor(bar_w * lpct / 100)
        if seg_w > 0 then
          gfx:fill_rounded_rect(cx, bar_y, seg_w - 1, bar_h, 1, lclr)

          if seg_w > 30 and labels_drawn < 2 and lx < px + bar_w - 30 then
            local lbl = ln .. " " .. tostring(lpct) .. "%"
            if #lbl > 14 then
              lbl = string.sub(ln, 1, 10) .. ".."
            end
            gfx:text(lx, label_y, lbl, lclr, "small")
            lx = lx + #lbl * 6 + 10
            labels_drawn = labels_drawn + 1
          end

          cx = cx + seg_w
        end
      end
    end

    repo_y = repo_y + repo_h
  end
end

local function render_languages(gfx, state, th, right_x, right_w, section_y)
  if state.lang_count == 0 then
    return section_y
  end

  gfx:text(right_x, section_y, "Languages", th.text_muted, "small")
  section_y = section_y + 16

  local max_langs = math.min(state.lang_count, 5)
  for i = 1, max_langs do
    local lname = state.lang_names[i] or ""
    local lpct = state.lang_pcts[i] or 0
    local dot_color = colors.get_lang_color(lname) or th.text_muted

    gfx:fill_circle(right_x + 4, section_y + 5, 3, dot_color)
    gfx:text(right_x + 14, section_y, lname, th.text_primary, "small")

    local pct_str = tostring(lpct) .. "%"
    local pct_w = #pct_str * 7
    gfx:text(right_x + right_w - pct_w, section_y, pct_str, th.text_muted, "small")

    section_y = section_y + 16
  end

  return section_y + 6
end

local function render_commits(gfx, state, th, right_x, right_w, section_y)
  if state.commit_count == 0 then
    return
  end

  gfx:text(right_x, section_y, "Recent Commits", th.text_muted, "small")
  section_y = section_y + 16

  local py = 15
  local commit_row_h = 38
  local max_commits = math.floor((state.height - section_y - py) / commit_row_h)

  for i = 1, state.commit_count do
    if i > max_commits then
      break
    end

    local r = state.commit_repos[i] or ""
    local m = state.commit_msgs[i] or ""
    local d = state.commit_dates[i] or ""
    local l = state.commit_lines[i] or ""
    local lang = state.commit_langs[i] or ""

    -- Language dot + repo · date
    local lang_clr = colors.get_lang_color(lang) or th.text_muted
    gfx:fill_circle(right_x + 4, section_y + 5, 3, lang_clr)

    local header = r .. " · " .. fetch_mod.short_date(d)
    gfx:text(right_x + 14, section_y, header, th.text_muted, "small")

    -- Commit message
    if #m > 35 then
      m = string.sub(m, 1, 32) .. "..."
    end
    gfx:text(right_x + 14, section_y + 13, m, th.text_primary, "small")

    -- Lines changed + language name
    local detail = l
    if #lang > 0 then
      detail = l .. " · " .. lang
    end
    gfx:text(right_x + 14, section_y + 25, detail, th.text_muted, "small")

    section_y = section_y + commit_row_h
  end
end

function M.render(state, gfx)
  local th = theme:get()
  local px, py = 20, 15

  local heat_colors = colors.make_heat_colors(th)

  -- Draw card
  components.card(gfx, 0, 0, state.width, state.height)

  -- Title bar
  local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "GitHub", {
    accent = th.accent_success,
  })

  local cy = py + title_h + 25

  if state.loading then
    components.loading(gfx, px, cy + 20)
    return
  end

  if state.error then
    components.error(gfx, px, cy + 10, state.width - px * 2, state.error)
    return
  end

  -- Layout: left column = heatmap + repos, right column = languages + commits
  local col_gap = 20
  local left_w = math.floor(state.width * 0.55)
  local right_x = left_w + col_gap
  local right_w = state.width - right_x - px

  -- Left column: username + heatmap
  gfx:text(px, cy, "@" .. state.username, th.text_accent, "medium")
  gfx:text(px, cy + 18, tostring(state.total) .. " contributions", th.text_muted, "small")

  local grid_y = cy + 38
  local available_w = left_w - px - 5
  local available_h = state.height - grid_y - py - 20

  local heatmap_bottom = render_heatmap(gfx, state, th, px, grid_y, available_w, available_h, heat_colors)

  -- Bottom left: recent repositories
  render_repos(gfx, state, th, px, heatmap_bottom, left_w - px - 5)

  -- Right column: languages
  local section_y = cy
  section_y = render_languages(gfx, state, th, right_x, right_w, section_y)

  -- Divider
  gfx:line(right_x, section_y, right_x + right_w, section_y, th.border_primary, 1)
  section_y = section_y + 10

  -- Right column: commits
  render_commits(gfx, state, th, right_x, right_w, section_y)
end

return M
