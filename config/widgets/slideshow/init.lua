-- Slideshow Widget
-- Displays images from SPIFFS, cycling on a timer.
--
-- Options:
--   images         - table of image file paths (required)
--   interval       - seconds between slides (default: 5)
--   fetch_urls     - optional table of URLs to download images from on init
--   fetch_prefix   - path prefix for downloaded images (default: "/data/config/slide")

local M = {}

function M.init(ctx)
  local images = ctx.opts.images or {}
  local prefix = ctx.opts.fetch_prefix or "/data/config/slide"

  if ctx.opts.fetch_urls then
    for i, url in ipairs(ctx.opts.fetch_urls) do
      local path = prefix .. i .. ".jpg"
      local res = net.download(url, path)
      if res and res.ok then
        images[#images + 1] = path
      else
        local err = (res and res.error) or "unknown error"
        print("Slideshow: failed to download " .. url .. ": " .. tostring(err))
      end
    end
  end

  return {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    images = images,
    current = 1,
    timer = 0,
    interval = (ctx.opts.interval or 5) * 1000,
  }
end

function M.update(state, delta_ms)
  if not state.images or #state.images == 0 then
    return
  end
  state.timer = (state.timer or 0) + delta_ms
  if state.timer >= (state.interval or 5000) then
    state.timer = 0
    state.current = (state.current % #state.images) + 1
  end
end

function M.render(state, gfx)
  local th = theme:get()
  local w = state.width or 0
  local h = state.height or 0

  if not state.images or #state.images == 0 then
    gfx:clear(th.bg_primary)
    gfx:text(20, h / 2, "No images", th.text_muted, "inter", 16)
    return
  end

  local path = state.images[state.current or 1]
  -- Scale image to fill the widget
  gfx:draw_image(0, 0, w, h, path)

  local indicator = (state.current or 1) .. "/" .. #state.images
  gfx:text(w - 60, h - 25, indicator, th.text_muted, "inter", 14)
end

function M.on_event(state, event)
  if event.type == "tap" and state.images and #state.images > 0 then
    state.current = (state.current % #state.images) + 1
    state.timer = 0
    return true
  end
  return false
end

return M
