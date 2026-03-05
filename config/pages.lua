-- Moondeck Pages Configuration
-- Define your display pages and widgets here

return {
	pages = {
		{
			id = "home",
			title = "Home",
			background = "#1a1a2e",
			widgets = {
				{
					module = "widgets.clock",
					x = 20,
					y = 20,
					w = 760,
					h = 180,
					update_interval = 1000,
					opts = {
						show_seconds = true,
						show_date = true,
						timezone = "local",
					},
				},
				{
					module = "widgets.status",
					x = 20,
					y = 220,
					w = 370,
					h = 200,
					update_interval = 5000,
					opts = {},
				},
				{
					module = "widgets.quote",
					x = 410,
					y = 220,
					w = 370,
					h = 200,
					update_interval = 60000,
					opts = {},
				},
			},
		},
		{
			id = "weather",
			title = "Weather",
			background = "#16213e",
			widgets = {
				{
					module = "widgets.weather",
					x = 20,
					y = 20,
					w = 760,
					h = 420,
					update_interval = 300000, -- 5 minutes
					opts = {
						city = "New York",
						units = "imperial",
					},
				},
			},
		},
		{
			id = "system",
			title = "System",
			background = "#0f0f23",
			widgets = {
				{
					module = "widgets.sysinfo",
					x = 20,
					y = 20,
					w = 760,
					h = 420,
					update_interval = 2000,
					opts = {},
				},
			},
		},
	},
}
