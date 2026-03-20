-- Quote Widget

local M = {}

-- Built-in quotes collection
local builtin_quotes = {
  { text = "An unexamined life is not worth living.", author = "Socrates" },
  { text = "Simplicity is the ultimate sophistication.", author = "Leonardo da Vinci" },
  { text = "The only true wisdom is in knowing you know nothing.", author = "Socrates" },
  { text = "The only way to do great work is to love what you do.", author = "Steve Jobs" },
  { text = "Innovation distinguishes between a leader and a follower.", author = "Steve Jobs" },
  { text = "Stay hungry, stay foolish.", author = "Steve Jobs" },
  { text = "It is during our darkest moments that we must focus to see the light.", author = "Aristotle" },
  { text = "In the middle of difficulty lies opportunity.", author = "Albert Einstein" },
  {
    text = "Many of life's failures are people who did not realize how close they were to success when they gave up.",
    author = "Thomas Edison",
  },
  { text = "Do what you can, with what you have, where you are.", author = "Theodore Roosevelt" },
  { text = "Everything you've ever wanted is on the other side of fear.", author = "George Addair" },
}

local function set_quote(state, index)
  local quote = builtin_quotes[index]
  state.quote_text = quote.text
  state.quote_author = quote.author
  state.quote_index = index
end

function M.init(ctx)
  local state = {
    x = ctx.x,
    y = ctx.y,
    width = ctx.width,
    height = ctx.height,
    quote_text = nil,
    quote_author = nil,
    quote_index = 1,
    last_change = 0,
    change_interval = ctx.opts.change_interval or 60000,
    use_api = ctx.opts.use_api or false,
    loading = false,
    error = nil,
  }
  set_quote(state, 1)
  return state
end

function M.update(state, delta_ms)
  state.last_change = state.last_change + delta_ms

  if state.last_change >= state.change_interval then
    state.last_change = 0
    local next_index = (state.quote_index % #builtin_quotes) + 1
    set_quote(state, next_index)
  end
end

function M.render(state, gfx)
  local th = theme:get()
  local px, py = 25, 20

  -- Draw card
  components.card(gfx, 0, 0, state.width, state.height)

  -- Title bar
  local title_h = components.title_bar(gfx, px, py, state.width - px * 2, "Quote", {
    accent = th.accent_primary,
  })

  local content_y = py + title_h + 15

  if state.loading then
    components.loading(gfx, px, state.height / 2 - 10)
    return
  end

  if not state.quote_text then
    gfx:text(px, state.height / 2 - 10, "No quote available", th.text_muted, "inter", 16)
    return
  end

  -- Opening quotation mark (decorative)
  gfx:text(px - 5, content_y, '"', th.accent_primary, "inter", 32)

  -- Calculate text layout
  local chars_per_line = math.floor((state.width - px * 2 - 20) / 8)
  local lines = util:word_wrap(state.quote_text, chars_per_line)

  local line_height = 22
  local text_start_y = content_y + 20
  local max_lines = math.floor((state.height - text_start_y - 50) / line_height)

  -- Draw quote text
  for i = 1, #lines do
    if i > max_lines then
      gfx:text(px + 15, text_start_y + (max_lines - 1) * line_height, "...", th.text_primary, "inter", 16)
      break
    end
    gfx:text(px + 15, text_start_y + (i - 1) * line_height, lines[i], th.text_primary, "inter", 16)
  end

  -- Author attribution
  if state.quote_author then
    local author_y = state.height - py - 15
    gfx:text(px + 15, author_y, "— " .. state.quote_author, th.text_accent, "inter", 16)
  end

  -- Subtle accent line
  -- gfx:line(px, state.height - py - 35, state.width - px, state.height - py - 35, th.accent_primary, 2)
end

function M.on_event(state, event)
  if event.type == "tap" then
    state.last_change = state.change_interval
    return true
  end
  return false
end

return M
