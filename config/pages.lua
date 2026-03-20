-- Moondeck Pages Configuration
-- This is the main configuration file for your Moondeck dashboard.
-- Define your display pages and widgets here.
--
-- Layout Templates Available:
--   full          - Single full-screen widget
--   half_half     - Two equal columns
--   thirds        - Three equal columns
--   main_sidebar  - Large left (8 cols) + two stacked right (4 cols)
--   header_two_col - Header row + two columns below
--   quad          - 2x2 grid
--   dashboard     - Main area + sidebar widgets
--   cards_4       - Four equal cards in a row

local sysinfo = require("widgets.sysinfo")
local weather = require("widgets.weather")
local quote = require("widgets.quote")
local crypto = require("widgets.crypto")
local clock = require("widgets.clock")
local status = require("widgets.status")
local bible = require("widgets.bible")
local rss = require("widgets.rss")
local stocks = require("widgets.stocks")
local github = require("widgets.github")
local chess = require("widgets.chess")

return {
  pages = {
    {
      id = "home",
      title = "Home",
      layout = "quad",
      widgets = {
        {
          widget = chess,
          update_interval = 300000,
          opts = {
            username = env.get("CHESS_USERNAME"),
          },
        },
        {
          widget = quote,
          update_interval = 60000,
          opts = {},
        },
        {
          widget = crypto,
          update_interval = 60000,
          opts = {
            coins = { "bitcoin", "ethereum", "solana", "monero" },
          },
        },
        {
          widget = stocks,
          update_interval = 300000,
          opts = {
            symbols = { "AAPL", "GOOGL", "PLTR", "TSLA", "MO" },
          },
        },
      },
    },

    {
      id = "dashboard",
      title = "Dashboard",
      layout = "cards_4",
      widgets = {
        {
          widget = sysinfo,
          update_interval = 1000,
          opts = {},
        },
        {
          widget = status,
          update_interval = 1000,
          opts = {},
        },
        {
          widget = weather,
          update_interval = 300000,
          opts = {},
        },
        {
          widget = clock,
          update_interval = 1000,
          opts = {
            timezone = env.get("TIMEZONE"),
            show_seconds = true,
            show_date = true,
            format_24h = false,
          },
        },
      },
    },

    {
      id = "reading",
      title = "Reading",
      layout = "half_half",
      widgets = {
        {
          widget = bible,
          update_interval = 3600000,
          opts = {},
        },
        {
          widget = rss,
          update_interval = 300000,
          opts = {},
        },
      },
    },

    {
      id = "heatmap",
      title = "GitHub Heatmap",
      layout = "full",
      widgets = {
        {
          widget = github,
          update_interval = 300000,
          opts = {},
        },
      },
    },
  },
}
