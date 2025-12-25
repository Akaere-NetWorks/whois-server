-- Example Weather Plugin
-- Demonstrates the plugin API for the WHOIS server
--
-- Usage: echo "beijing-WEATHER" | nc localhost 43

local function trim(s)
    return s:match("^%s*(.-)%s*$")
end

-- Simple JSON parsing (extract body field)
local function parse_json_response(json_str)
    -- Extract the "body" value from JSON like {"status": 200, "body": "..."}
    local body_start = json_str:find('"body":"')
    if not body_start then
        return nil
    end

    local body_value = json_str:sub(body_start + 9)  -- Skip "body":"
    -- Find closing quote (handling escaped quotes is complex, simplified here)
    local body_end = body_value:find('"}')

    if body_end then
        return body_value:sub(1, body_end - 1)
    end

    -- Fallback: look for closing quote with }
    body_end = body_value:find('"')
    if body_end then
        return body_value:sub(1, body_end - 1)
    end

    return nil
end

local function fetch_weather(location)
    -- Check cache first
    local cache_key = "weather:example:" .. location
    local cached = cache_get(cache_key)
    if cached then
        return cached
    end

    -- Build URL for wttr.in (simple weather API)
    local url = "https://wttr.in/" .. location .. "?format=3"

    -- Make HTTP request (whitelist-enforced)
    local ok, result = pcall(http_get, url)
    if not ok then
        return nil
    end

    -- Parse JSON response
    -- Result format: {"status": 200, "body": "Beijing: ☀️ +25°C"}
    local weather = parse_json_response(result)

    if not weather then
        return nil
    end

    weather = trim(weather)

    -- Cache for 30 minutes (1800 seconds)
    cache_set(cache_key, weather, 1800)

    return weather
end

-- Required: Handle incoming queries
function handle_query(query)
    -- Validate input
    if not query or query == "" then
        return [[
% Error: Location parameter required
%
% Usage: <location>-WEATHER
% Example: beijing-WEATHER
%          london-WEATHER
%          new-york-WEATHER

]]
    end

    -- Trim whitespace
    local location = trim(query)

    -- Log the query
    log_info("Weather query for location: " .. location)

    -- Fetch weather data
    local weather = fetch_weather(location)

    if not weather then
        return "% Error: Failed to fetch weather data for '" .. location .. "'\n"
    end

    -- Format response in WHOIS style
    local response = string.format([[
%% Weather Information
%% Location: %s
%% Condition: %s
%% Source: wttr.in
]], location, weather)

    return response
end

-- Optional: Called when plugin loads
function init()
    log_info("Weather plugin initialized")
end

-- Optional: Called when server shuts down
function cleanup()
    log_info("Weather plugin cleanup")
end
