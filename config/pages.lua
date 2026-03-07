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
		-- Page 1: Dashboard (main focus + sidebar)
		{
			id = "dashboard",
			title = "Dashboard",
			background = "#0a0a0f",
			layout = "main_sidebar",
			widgets = {
				-- Main area: Clock
				{
					module = "widgets.clock",
					slot = 1,
					update_interval = 1000,
					opts = {
						show_seconds = true,
						show_date = true,
						format_24h = false,
					},
				},
				-- Sidebar top: Weather
				{
					module = "widgets.weather",
					slot = 2,
					update_interval = 300000,
					opts = {},
				},
				-- Sidebar bottom: Crypto
				{
					module = "widgets.crypto",
					slot = 3,
					update_interval = 60000,
					opts = {},
				},
			},
		},

		-- Page 2: Finance
		{
			id = "finance",
			title = "Finance",
			background = "#0a0a0f",
			layout = "half_half",
			widgets = {
				-- Left: Stocks
				{
					module = "widgets.stocks",
					slot = 1,
					update_interval = 300000,
					opts = {},
				},
				-- Right: Crypto
				{
					module = "widgets.crypto",
					slot = 2,
					update_interval = 60000,
					opts = {},
				},
			},
		},

		-- Page 3: Reading
		{
			id = "reading",
			title = "Reading",
			background = "#0a0a0f",
			layout = "half_half",
			widgets = {
				-- Left: Bible Verse
				{
					module = "widgets.bible",
					slot = 1,
					update_interval = 3600000,
					opts = {},
				},
				-- Right: RSS Feed
				{
					module = "widgets.rss",
					slot = 2,
					update_interval = 300000,
					opts = {},
				},
			},
		},

		-- Page 4: Inspiration
		{
			id = "inspiration",
			title = "Inspiration",
			background = "#0a0a0f",
			layout = "full",
			widgets = {
				{
					module = "widgets.quote",
					slot = 1,
					update_interval = 60000,
					opts = {
						change_interval = 30000,
					},
				},
			},
		},

		-- Page 5: System
		{
			id = "system",
			title = "System",
			background = "#0a0a0f",
			layout = "full",
			widgets = {
				{
					module = "widgets.sysinfo",
					slot = 1,
					update_interval = 1000,
					opts = {},
				},
			},
		},
	},
}
