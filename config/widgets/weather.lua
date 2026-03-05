-- Weather Widget
-- Fetches and displays weather data from OpenWeatherMap API

local M = {}

function M.init(ctx)
    return {
        x = ctx.x,
        y = ctx.y,
        width = ctx.width,
        height = ctx.height,
        city = ctx.opts.city or "New York",
        units = ctx.opts.units or "metric",
        temperature = nil,
        description = "Loading...",
        humidity = nil,
        wind_speed = nil,
        last_fetch = 0,
        fetch_interval = 300000, -- 5 minutes
        error = nil,
    }
end

function M.update(state, delta_ms)
    state.last_fetch = state.last_fetch + delta_ms
    
    -- Fetch weather data periodically
    if state.last_fetch >= state.fetch_interval or state.temperature == nil then
        M.fetch_weather(state)
        state.last_fetch = 0
    end
end

function M.fetch_weather(state)
    local api_key = env.get("WEATHER_API_KEY")
    if not api_key then
        state.error = "No API key configured"
        state.description = "Set WEATHER_API_KEY in .env"
        return
    end
    
    local url = string.format(
        "https://api.openweathermap.org/data/2.5/weather?q=%s&units=%s&appid=%s",
        state.city,
        state.units,
        api_key
    )
    
    local response = net.http_get(url, nil, 5000)
    
    if response.ok then
        local data = net.json_decode(response.body)
        if data then
            state.temperature = data.main and data.main.temp
            state.humidity = data.main and data.main.humidity
            state.description = data.weather and data.weather[1] and data.weather[1].description or "Unknown"
            state.wind_speed = data.wind and data.wind.speed
            state.error = nil
        else
            state.error = "Failed to parse response"
        end
    else
        state.error = response.error or "Request failed"
    end
end

function M.render(state, gfx)
    -- Draw background panel
    gfx:fill_rounded_rect(0, 0, state.width, state.height, 16, "#1e3a5f")
    
    -- Title
    gfx:text(20, 40, "Weather - " .. state.city, "white", "large")
    gfx:line(20, 55, state.width - 20, 55, "#e94560", 2)
    
    if state.error then
        -- Show error
        gfx:text(20, 150, "Error:", "#ff6b6b", "medium")
        gfx:text(20, 180, state.error, "#888888", "small")
    else
        -- Temperature (large display)
        if state.temperature then
            local temp_str = string.format("%.1f°", state.temperature)
            local unit = state.units == "metric" and "C" or "F"
            gfx:text(40, 150, temp_str, "white", "xlarge")
            gfx:text(180, 150, unit, "#888888", "large")
        end
        
        -- Description
        gfx:text(40, 220, state.description:gsub("^%l", string.upper), "#00d9ff", "large")
        
        -- Additional info
        local y_offset = 280
        
        if state.humidity then
            gfx:text(40, y_offset, "Humidity: " .. state.humidity .. "%", "#888888", "medium")
            y_offset = y_offset + 35
        end
        
        if state.wind_speed then
            local wind_unit = state.units == "metric" and "m/s" or "mph"
            gfx:text(40, y_offset, "Wind: " .. state.wind_speed .. " " .. wind_unit, "#888888", "medium")
        end
    end
    
    -- Decorative border
    gfx:stroke_rounded_rect(5, 5, state.width - 10, state.height - 10, 12, "#e94560", 2)
    
    -- Weather icon placeholder (sun/cloud)
    local icon_x = state.width - 120
    local icon_y = 120
    gfx:fill_circle(icon_x, icon_y, 40, "#ffcc00")
    gfx:stroke_circle(icon_x, icon_y, 40, "#ff9500", 2)
end

function M.on_event(state, event)
    if event.type == "tap" then
        -- Force refresh on tap
        state.last_fetch = state.fetch_interval
        return true
    end
    return false
end

return M
