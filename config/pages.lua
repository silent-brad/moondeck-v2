-- Moondeck Pages Configuration
-- Define your display pages and widgets here
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

return {
	pages = {
		{
			id = "home",
			title = "Home",
			layout = "quad",
			widgets = {
				{
					module = "widgets.sysinfo",
					slot = 1,
					update_interval = 1000,
					opts = {},
				},
				{
					module = "widgets.weather",
					slot = 2,
					update_interval = 300000,
					opts = {},
				},
				{
					module = "widgets.quote",
					slot = 3,
					update_interval = 60000,
					opts = {},
				},
				{
					module = "widgets.crypto",
					slot = 4,
					update_interval = 60000,
					opts = {
						coins = { "bitcoin", "ethereum", "solana", "monero" },
					},
				},
			},
		},

		{
			id = "dashboard",
			title = "Dashboard",
			layout = "half_half",
			widgets = {
				{
					module = "widgets.clock",
					slot = 1,
					update_interval = 1000,
					opts = {
						timezone = -4, -- EDT (Eastern Daylight Time)
						show_seconds = true,
						show_date = true,
						format_24h = false,
					},
				},
				{
					module = "widgets.status",
					slot = 2,
					update_interval = 1000,
					opts = {},
				},
			},
		},

		{
			id = "reading",
			title = "Reading",
			layout = "half_half",
			widgets = {
				{
					module = "widgets.bible",
					slot = 1,
					update_interval = 3600000,
					opts = {},
				},
				{
					module = "widgets.rss",
					slot = 2,
					update_interval = 300000,
					opts = {},
				},
			},
		},

		{
			id = "stocks",
			title = "Stocks",
			layout = "full",
			widgets = {
				{
					module = "widgets.stocks",
					slot = 1,
					update_interval = 300000,
					opts = {
						symbols = { "AAPL", "GOOGL", "PLTR", "TSLA" },
					},
				},
			},
		},

    {
      id = "heatmap",
      title = "GitHub Heatmap",
      layout = "full",
      widgets = {
        {
          module = "widgets.github",
          slot = 1,
          update_interval = 3600000,
          opts = {},
        },
      },
    },

	},
}
