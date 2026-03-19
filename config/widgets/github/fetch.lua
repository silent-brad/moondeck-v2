-- GitHub Widget: API fetching and response parsing

local colors = require("widgets.github.colors")

local M = {}

-- Parse two-digit string to number
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

local function two_digit(s, pos)
  local hi = digit_vals[string.sub(s, pos, pos)] or 0
  local lo = digit_vals[string.sub(s, pos + 1, pos + 1)] or 0
  return hi * 10 + lo
end

-- Format "2026-03-18T12:34:56Z" to "Mar 18"
function M.short_date(iso)
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

-- Extract first line from a commit message
local function first_line(msg)
  msg = tostring(msg or "")
  for ci = 1, #msg do
    local ch = string.sub(msg, ci, ci)
    if ch == "\n" or ch == "\r" then
      return string.sub(msg, 1, ci - 1)
    end
  end
  return msg
end

-- Build the GraphQL query string
-- stylua: ignore
local function build_query(username)
  return '{"query":"' ..
    'query { ' ..
      'user(login: \\\"' .. username .. '\\\") { ' ..
        'contributionsCollection { ' ..
          'contributionCalendar { ' ..
            'totalContributions ' ..
            'weeks { contributionDays { contributionCount date } } ' ..
          '} ' ..
        '} ' ..
        'repositories(first: 10, orderBy: {field: PUSHED_AT, direction: DESC}, ownerAffiliations: OWNER) { ' ..
          'nodes { ' ..
            'name ' ..
            'description ' ..
            'pushedAt ' ..
            'createdAt ' ..
            'visibility ' ..
            'primaryLanguage { name } ' ..
            'defaultBranchRef { ' ..
              'target { ' ..
                '... on Commit { ' ..
                  'history(first: 3) { nodes { message committedDate additions deletions } } ' ..
                '} ' ..
              '} ' ..
            '} ' ..
            'languages(first: 5, orderBy: {field: SIZE, direction: DESC}) { ' ..
              'edges { size node { name } } ' ..
            '} ' ..
          '} ' ..
        '} ' ..
      '} ' ..
    '}"}'
end

-- Parse commits from repo nodes into flat parallel arrays on state
local function parse_commits(state, repos)
  state.commit_repos = {}
  state.commit_msgs = {}
  state.commit_dates = {}
  state.commit_lines = {}
  state.commit_langs = {}
  state.commit_count = 0

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
        local n = state.commit_count + 1
        state.commit_count = n
        state.commit_repos[n] = rname
        state.commit_msgs[n] = first_line(c.message)
        state.commit_dates[n] = tostring(c.committedDate or "")
        state.commit_lines[n] = "+" .. tostring(c.additions or 0) .. " -" .. tostring(c.deletions or 0)
        state.commit_langs[n] = plang
      end
    end
  end
end

-- Insertion sort a table by a key in descending order
local function sort_desc(tbl, key_fn)
  for i = 2, #tbl do
    local item = tbl[i]
    local item_val = key_fn(item)
    local j = i - 1
    while j >= 1 and key_fn(tbl[j]) < item_val do
      tbl[j + 1] = tbl[j]
      j = j - 1
    end
    tbl[j + 1] = item
  end
end

-- Aggregate language sizes across repos into sorted parallel arrays on state
local function parse_languages(state, repos)
  local lang_totals = {}
  local lang_order = {}
  local total_size = 0

  for i = 1, #repos do
    local langs = repos[i].languages and repos[i].languages.edges or {}
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

  sort_desc(lang_order, function(name)
    return lang_totals[name]
  end)

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
end

-- Parse recent repos (max 3) with per-repo language breakdowns
local function parse_repos(state, repos)
  state.repo_names = {}
  state.repo_descs = {}
  state.repo_visibilities = {}
  state.repo_pushed = {}
  state.repo_lang_names = {}
  state.repo_lang_pcts = {}
  state.repo_lang_colors = {}
  state.repo_lang_counts = {}
  state.repo_count = 0

  local max_repos = 3
  for i = 1, math.min(max_repos, #repos) do
    local r = repos[i]
    local desc = tostring(r.description or "")
    if #desc > 60 then
      desc = string.sub(desc, 1, 57) .. "..."
    end

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

    sort_desc(r_langs, function(l)
      return l.size
    end)

    local ri = i
    local lcount = math.min(3, #r_langs)
    state.repo_lang_counts[ri] = lcount
    state.repo_lang_names[ri] = {}
    state.repo_lang_pcts[ri] = {}
    state.repo_lang_colors[ri] = {}
    for k = 1, lcount do
      local l = r_langs[k]
      local pct = r_total > 0 and math.floor(l.size * 100 / r_total) or 0
      local clr = colors.get_lang_color(l.name) or 0x888888
      state.repo_lang_names[ri][k] = l.name
      state.repo_lang_pcts[ri][k] = pct
      state.repo_lang_colors[ri][k] = clr
    end

    state.repo_count = state.repo_count + 1
    state.repo_names[state.repo_count] = tostring(r.name or "")
    state.repo_descs[state.repo_count] = desc
    state.repo_visibilities[state.repo_count] = tostring(r.visibility or "PUBLIC")
    state.repo_pushed[state.repo_count] = tostring(r.pushedAt or "")
  end
end

-- Fetch GitHub data via GraphQL and populate state
function M.fetch(state)
  local token = env.get("GITHUB_TOKEN")
  if not token then
    return false, "No GITHUB_TOKEN"
  end
  token = tostring(token)

  if state.username == "" then
    return false, "No username"
  end
  state.username = tostring(state.username)

  local query = build_query(state.username)
  local headers = {
    Authorization = "Bearer " .. token,
    ["User-Agent"] = "moondeck",
  }

  local response = net.http_post("https://api.github.com/graphql", query, "application/json", headers, 15000)

  if not (response and response.ok and response.body) then
    return false, response and response.error or "Network error"
  end

  local data = net.json_decode(response.body)
  if not (data and data.data and data.data.user) then
    return false, "User not found"
  end

  local user = data.data.user
  local cal = user.contributionsCollection.contributionCalendar
  state.total = cal.totalContributions or 0
  state.weeks = cal.weeks or {}

  local repos = user.repositories and user.repositories.nodes or {}
  parse_commits(state, repos)
  parse_languages(state, repos)
  parse_repos(state, repos)

  return true
end

return M
