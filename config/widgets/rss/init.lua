-- RSS Widget
-- Fetches feed entries from Miniflux API

local M = {}

function M.init(ctx)
  local fetch_interval = ctx.opts.update_interval or 300000 -- 5 minutes

  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    entries = {},
    current_index = 1,
    limit = ctx.opts.limit or 10,
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

    local api_url = env.get("MINIFLUX_URL")
    local api_key = env.get("MINIFLUX_API_KEY")

    if not api_url then
      state.error = "No MINIFLUX_URL"
      state.loading = false
      return
    end

    if not api_key then
      state.error = "No API key"
      state.loading = false
      return
    end

    -- Miniflux API: GET /v1/entries?status=unread&limit=N
    local url = api_url .. "/v1/entries?status=unread&limit=" .. state.limit .. "&direction=desc&order=published_at"

    local headers = {
      ["X-Auth-Token"] = api_key,
    }

    local response = net.http_get(url, headers, 15000)

    if response and response.ok and response.body then
      local data = net.json_decode(response.body)

      if data and data.entries then
        state.entries = {}
        local idx = 1
        for i = 1, #data.entries do
          local entry = data.entries[i]
          if entry then
            local feed_title = ""
            if entry.feed and entry.feed.title then
              feed_title = entry.feed.title
            end
            state.entries[idx] = {
              id = entry.id,
              title = entry.title or "Untitled",
              feed = feed_title,
              url = entry.url,
            }
            idx = idx + 1
          end
        end
        state.loading = false
        state.error = nil
      else
        state.error = "Invalid response"
        state.loading = false
      end
    else
      state.error = response and response.error or "Network error"
      state.loading = false
    end
  end
end

-- Truncate text with ellipsis
local function truncate(text, max_len)
  if not text then
    return ""
  end
  if #text <= max_len then
    return text
  end
  local truncated = ""
  for i = 1, max_len - 3 do
    truncated = truncated .. string.sub(text, i, i)
  end
  return truncated .. "..."
end

function M.render(state, gfx)
  local th = theme:get()
  local px, py = 20, 15

  -- Draw card
  components.card(gfx, 0, 0, state.width, state.height)

  -- Title bar with entry count
  local title = "RSS Feed"
  if #state.entries > 0 then
    title = title .. " (" .. #state.entries .. ")"
  end

  local title_h = components.title_bar(gfx, px, py, state.width - px * 2, title, {
    accent = th.accent_primary,
  })

  local content_y = py + title_h + 25

  if state.loading then
    components.loading(gfx, px, content_y + 20)
    return
  end

  if state.error then
    components.error(gfx, px, content_y, state.width - px * 2, state.error)
    return
  end

  if #state.entries == 0 then
    gfx:text(px, content_y + 20, "No unread entries", th.text_muted, "medium")
    return
  end

  -- Display entries as list
  local row_height = 45
  local max_rows = math.floor((state.height - content_y - py - 20) / row_height)
  local title_max_chars = math.floor((state.width - px * 2) / 7)

  for i = 1, #state.entries do
    if i > max_rows then
      break
    end

    local entry = state.entries[i]
    local y = content_y + (i - 1) * row_height

    -- Entry indicator
    gfx:fill_circle(px + 4, y + 8, 3, th.accent_primary)

    -- Title
    local title_text = truncate(entry.title, title_max_chars)
    gfx:text(px + 15, y, title_text, th.text_primary, "medium")

    -- Feed name
    gfx:text(px + 15, y + 18, entry.feed, th.text_muted, "small")
  end

  -- Navigation hint at bottom
  if #state.entries > max_rows then
    local more = #state.entries - max_rows
    gfx:text(px, state.height - py - 5, "+" .. more .. " more", th.text_muted, "small")
  end
end

function M.on_event(state, event)
  if event.type == "tap" then
    -- Cycle through entries or refresh
    if #state.entries > 0 then
      state.current_index = (state.current_index % #state.entries) + 1
    end
    return true
  elseif event.type == "long_press" then
    -- Refresh on long press
    state.last_fetch = state.fetch_interval
    state.loading = true
    return true
  end
  return false
end

return M
