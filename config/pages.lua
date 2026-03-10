-- Minimal test pages config
-- Testing sysinfo and weather widgets

return {
	pages = {
		{
			id = "test",
			title = "Test",
			layout = "half_half",
			--layout = "main_sidebar",
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

				--[[
        {
					module = "widgets.quote",
					slot = 3,
					update_interval = 60000,
					opts = {},
				},
        --]]
			},
		},
	},
}
