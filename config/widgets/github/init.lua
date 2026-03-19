-- GitHub Widget
-- Displays contribution heatmap, latest commits, and language breakdown

local M = {}

-- GitHub language colors (integer RGB values)
local lang_color_map = {
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

local function get_lang_color(name)
  return lang_color_map[name]
end

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

    local token = env.get("GITHUB_TOKEN")
    if not token then
      state.error = "No GITHUB_TOKEN"
      state.loading = false
      return
    end
    token = tostring(token)

    if state.username == "" then
      state.error = "No username"
      state.loading = false
      return
    end
    state.username = tostring(state.username)

    -- GraphQL query: contributions + recent commits + languages + repo details
    local query = '{"query":"query{user(login:\\"'
      .. state.username
      .. '\\"){contributionsCollection{contributionCalendar{totalContributions weeks{contributionDays{contributionCount date}}}}'
      .. " repositories(first:10,orderBy:{field:PUSHED_AT,direction:DESC},ownerAffiliations:OWNER){"
      .. "nodes{name description pushedAt createdAt visibility primaryLanguage{name}"
      .. " defaultBranchRef{target{... on Commit{history(first:3){nodes{message committedDate additions deletions}}}}}"
      .. " languages(first:5,orderBy:{field:SIZE,direction:DESC}){edges{size node{name}}}"
      .. '}}}}"}'

    local headers = {
      Authorization = "Bearer " .. token,
      ["User-Agent"] = "moondeck",
    }

    local response = net.http_post("https://api.github.com/graphql", query, "application/json", headers, 15000)

    if response and response.ok and response.body then
      local data = net.json_decode(response.body)

      if data and data.data and data.data.user then
        local user = data.data.user

        -- Contribution calendar
        local cal = user.contributionsCollection.contributionCalendar
        state.total = cal.totalContributions or 0
        state.weeks = cal.weeks or {}

        -- Extract recent commits across repos (flat parallel arrays)
        state.commit_repos = {}
        state.commit_msgs = {}
        state.commit_dates = {}
        state.commit_lines = {}
        state.commit_langs = {}
        state.commit_count = 0
        local repos = user.repositories and user.repositories.nodes or {}
        for i = 1, #repos do
          local repo = repos[i]
          local ref = repo.defaultBranchRef
          if ref and ref.target and ref.target.history then
            local rname = tostring(repo.name or "")
            local plang = ""
            if repo.primaryLanguage and repo.primaryLanguage.name then
              plang = tostring(repo.primaryLanguage.name)
            end
            local history = ref.target.history.nodes or {}
            for j = 1, #history do
              local c = history[j]
              local msg = tostring(c.message or "")
              for ci = 1, #msg do
                local ch = string.sub(msg, ci, ci)
                if ch == "\n" or ch == "\r" then
                  msg = string.sub(msg, 1, ci - 1)
                  break
                end
              end
              local n = state.commit_count + 1
              state.commit_count = n
              state.commit_repos[n] = rname
              state.commit_msgs[n] = msg
              state.commit_dates[n] = tostring(c.committedDate or "")
              state.commit_lines[n] = "+" .. tostring(c.additions or 0) .. " -" .. tostring(c.deletions or 0)
              state.commit_langs[n] = plang
            end
          end
        end

        -- Aggregate language sizes across repos
        local lang_totals = {}
        local lang_order = {}
        local total_size = 0
        for i = 1, #repos do
          local repo = repos[i]
          local langs = repo.languages and repo.languages.edges or {}
          for k = 1, #langs do
            local edge = langs[k]
            local name = tostring(edge.node.name or "")
            local size = edge.size or 0
            if not lang_totals[name] then
              lang_totals[name] = 0
              lang_order[#lang_order + 1] = name
            end
            lang_totals[name] = lang_totals[name] + size
            total_size = total_size + size
          end
        end

        -- Build sorted language list (flat parallel arrays)
        -- Sort lang_order by size descending via insertion sort
        for i = 2, #lang_order do
          local key = lang_order[i]
          local key_size = lang_totals[key]
          local j = i - 1
          while j >= 1 and lang_totals[lang_order[j]] < key_size do
            lang_order[j + 1] = lang_order[j]
            j = j - 1
          end
          lang_order[j + 1] = key
        end

        state.lang_names = {}
        state.lang_pcts = {}
        state.lang_count = #lang_order
        for i = 1, #lang_order do
          local name = lang_order[i]
          state.lang_names[i] = name
          if total_size > 0 then
            state.lang_pcts[i] = math.floor(lang_totals[name] * 1000 / total_size) / 10
          else
            state.lang_pcts[i] = 0
          end
        end

        -- Parse recent repos for bottom left display (max 3)
        state.repo_names = {}
        state.repo_descs = {}
        state.repo_visibilities = {}
        state.repo_pushed = {}
        state.repo_lang_names = {}
        state.repo_lang_pcts = {}
        state.repo_lang_colors = {}
        state.repo_lang_counts = {}
        state.repo_count = 0

        local max_repos_display = 3
        for i = 1, math.min(max_repos_display, #repos) do
          local r = repos[i]
          local name = tostring(r.name or "")
          local desc = tostring(r.description or "")
          if #desc > 60 then
            desc = string.sub(desc, 1, 57) .. "..."
          end
          local vis = tostring(r.visibility or "PUBLIC")
          local pushed = tostring(r.pushedAt or "")

          -- Language breakdown for this repo
          local r_langs = {}
          local r_total = 0
          local edges = r.languages and r.languages.edges or {}
          for k = 1, #edges do
            local e = edges[k]
            local lname = tostring(e.node.name or "")
            local size = e.size or 0
            r_total = r_total + size
            r_langs[#r_langs + 1] = { name = lname, size = size }
          end

          -- Sort descending by size (insertion sort)
          for a = 2, #r_langs do
            local key = r_langs[a]
            local b = a - 1
            while b >= 1 and r_langs[b].size < key.size do
              r_langs[b + 1] = r_langs[b]
              b = b - 1
            end
            r_langs[b + 1] = key
          end

          -- Store language data as flat parallel arrays (max 3 per repo)
          local ri = i
          local lcount = math.min(3, #r_langs)
          state.repo_lang_counts[ri] = lcount
          state.repo_lang_names[ri] = {}
          state.repo_lang_pcts[ri] = {}
          state.repo_lang_colors[ri] = {}
          for k = 1, lcount do
            local l = r_langs[k]
            local pct = r_total > 0 and math.floor(l.size * 100 / r_total) or 0
            local clr = get_lang_color(l.name) or 0x888888
            state.repo_lang_names[ri][k] = l.name
            state.repo_lang_pcts[ri][k] = pct
            state.repo_lang_colors[ri][k] = clr
          end

          state.repo_count = state.repo_count + 1
          state.repo_names[state.repo_count] = name
          state.repo_descs[state.repo_count] = desc
          state.repo_visibilities[state.repo_count] = vis
          state.repo_pushed[state.repo_count] = pushed
        end

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

-- Parse two-digit string to number
local function two_digit(s, pos)
  local digit_vals = {
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
  }
  local hi = digit_vals[string.sub(s, pos, pos)] or 0
  local lo = digit_vals[string.sub(s, pos + 1, pos + 1)] or 0
  return hi * 10 + lo
end

-- Format "2026-03-18T12:34:56Z" to "Mar 18"
local function short_date(iso)
  if not iso then
    return ""
  end
  iso = tostring(iso)
  if #iso < 10 then
    return ""
  end
  local months = {
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  }
  local m = two_digit(iso, 6)
  if m < 1 or m > 12 then
    m = 1
  end
  local d = string.sub(iso, 9, 10)
  if string.sub(d, 1, 1) == "0" then
    d = string.sub(d, 2, 2)
  end
  return months[m] .. " " .. d
end

function M.render(state, gfx)
  local th = theme:get()
  local px, py = 20, 15

  -- Color helpers
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

  local base = th.accent_secondary
  local target = th.accent_success
  local heat_colors = {
    mix(base, target, 0),
    mix(base, target, 0.5),
    mix(base, target, 0.75),
    mix(base, target, 0.9),
    mix(base, target, 1),
  }

  local function count_to_color(count)
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

  -- Layout: left column = heatmap, right column = commits + languages
  local col_gap = 20
  local left_w = math.floor(state.width * 0.55)
  local right_x = left_w + col_gap
  local right_w = state.width - right_x - px

  -- ── LEFT COLUMN: Username + Heatmap ──
  gfx:text(px, cy, "@" .. state.username, th.text_accent, "medium")
  local total_str = tostring(state.total) .. " contributions"
  gfx:text(px, cy + 18, total_str, th.text_muted, "small")

  local grid_y = cy + 38
  local available_w = left_w - px - 5
  local available_h = state.height - grid_y - py - 20

  local num_weeks = #state.weeks
  local heatmap_bottom = grid_y

  if num_weeks > 0 then
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
          local color = count_to_color(day.contributionCount or 0)
          gfx:fill_rounded_rect(cx, dy, cell, cell, 0, color)
        end
      end
    end

    -- Legend
    local legend_y = grid_y + 7 * (cell + gap) + 4
    if legend_y + 10 < state.height - py then
      gfx:text(px, legend_y, "Less", th.text_muted, "small")
      local lx = px + 30
      for i = 1, #heat_colors do
        gfx:fill_rounded_rect(lx + (i - 1) * (cell + gap), legend_y, cell, cell, 0, heat_colors[i])
      end
      gfx:text(lx + 5 * (cell + gap) + 4, legend_y, "More", th.text_muted, "small")
    end

    heatmap_bottom = legend_y + 20
  end

  -- ── BOTTOM LEFT: Recent Repositories ──
  if state.repo_count > 0 then
    local repo_y = heatmap_bottom
    local repo_w = left_w - px - 5
    local repo_h = 58

    for i = 1, state.repo_count do
      if repo_y + repo_h > state.height - py then
        break
      end

      local rname = state.repo_names[i] or ""
      local rdesc = state.repo_descs[i] or ""
      local rvis = state.repo_visibilities[i] or "PUBLIC"
      local rdate = state.repo_pushed[i] or ""

      -- Truncate name if needed
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

      -- Date (pushed date)
      local date_str = short_date(rdate)
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

  -- ── RIGHT COLUMN: Languages + Commits ──
  local section_y = cy

  -- Languages section
  if state.lang_count > 0 then
    gfx:text(right_x, section_y, "Languages", th.text_muted, "small")
    section_y = section_y + 16

    local max_langs = math.min(state.lang_count, 5)
    for i = 1, max_langs do
      local lname = state.lang_names[i] or ""
      local lpct = state.lang_pcts[i] or 0
      local dot_color = get_lang_color(lname) or th.text_muted

      -- Colored dot + name + percentage
      gfx:fill_circle(right_x + 4, section_y + 5, 3, dot_color)
      gfx:text(right_x + 14, section_y, lname, th.text_primary, "small")

      local pct_str = tostring(lpct) .. "%"
      local pct_w = #pct_str * 7
      gfx:text(right_x + right_w - pct_w, section_y, pct_str, th.text_muted, "small")

      section_y = section_y + 16
    end

    section_y = section_y + 6
  end

  -- Divider
  gfx:line(right_x, section_y, right_x + right_w, section_y, th.border_primary, 1)
  section_y = section_y + 10

  -- Commits section
  if state.commit_count > 0 then
    gfx:text(right_x, section_y, "Recent Commits", th.text_muted, "small")
    section_y = section_y + 16

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
      local lang_clr = get_lang_color(lang) or th.text_muted
      gfx:fill_circle(right_x + 4, section_y + 5, 3, lang_clr)

      local header = r .. " · " .. short_date(d)
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
