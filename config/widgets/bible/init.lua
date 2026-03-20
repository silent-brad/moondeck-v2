-- Bible Verse Widget
-- Fetches random verses from bible-api.com

local M = {}

function M.init(ctx)
  local fetch_interval = ctx.opts.update_interval or 3600000 -- 1 hour
  local translation = ctx.opts.translation or "kjv" -- KJV default

  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    translation = translation,
    verse_text = nil,
    verse_ref = nil,
    last_fetch = fetch_interval, -- Trigger immediate fetch
    fetch_interval = fetch_interval,
    loading = true,
    error = nil,
  }
end

function M.update(state, delta_ms)
  state.last_fetch = state.last_fetch + delta_ms

  if state.last_fetch >= state.fetch_interval then
    state.last_fetch = 0

    -- Fetch random verse from bible-api.com
    local url = "https://bible-api.com/data/" .. state.translation .. "/random"

    local response = net.http_get(url, {}, 10000)

    if response and response.ok and response.body then
      local data = net.json_decode(response.body)

      if data and data.random_verse then
        local verse = data.random_verse

        -- Clean up text (remove leading/trailing whitespace)
        local text = verse.text or ""
        -- Simple trim: remove leading/trailing newlines
        local clean_text = ""
        local started = false
        for i = 1, #text do
          local c = string.sub(text, i, i)
          if c ~= "\n" and c ~= "\r" then
            started = true
          end
          if started then
            clean_text = clean_text .. c
          end
        end
        -- Trim trailing
        while
          #clean_text > 0
          and (
            string.sub(clean_text, #clean_text, #clean_text) == "\n"
            or string.sub(clean_text, #clean_text, #clean_text) == "\r"
          )
        do
          clean_text = string.sub(clean_text, 1, #clean_text - 1)
        end

        state.verse_text = clean_text
        state.verse_ref = verse.book .. " " .. verse.chapter .. ":" .. verse.verse
        state.loading = false
        state.error = nil
      else
        state.error = "No verse data"
        state.loading = false
      end
    else
      state.error = "Failed to fetch"
      state.loading = false
    end
  end
end

function M.render(state, gfx)
  local th = theme:get()
  local px, py = 20, 15

  -- Draw card
  components.card(gfx, 0, 0, state.width, state.height)

  local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Daily Verse", {
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

  if state.verse_text then
    -- Reserve space for the reference line at bottom
    local ref_h = 30
    local avail_h = state.height - content_y - py - ref_h

    -- Calculate characters per line based on width
    -- Use ~8.5px per char for Inter at size 16 to avoid horizontal overflow
    local chars_per_line = math.floor((state.width - px * 2) / 9)
    local lines = util.word_wrap(state.verse_text, chars_per_line)

    -- Calculate how many lines we can show
    local line_height = 18
    local max_lines = math.floor(avail_h / line_height)
    if max_lines < 1 then
      max_lines = 1
    end

    -- Draw verse text
    for i = 1, math.min(#lines, max_lines) do
      if i == max_lines and #lines > max_lines then
        gfx:text(px, content_y + (i - 1) * line_height, lines[i] .. "...", th.text_secondary, "inter", 16)
      else
        gfx:text(px, content_y + (i - 1) * line_height, lines[i], th.text_secondary, "inter", 16)
      end
    end

    -- Draw reference at bottom (within bounds)
    if state.verse_ref then
      gfx:text(px, state.height - py - 15, "— " .. state.verse_ref, th.text_accent, "inter", 16)
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
