-- Minimal test pages config
-- Testing sysinfo and weather widgets

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
	},
}
