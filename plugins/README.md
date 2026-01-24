# WHOIS Server Plugin System

This directory contains plugins for the WHOIS server. Plugins allow you to extend the server with custom query suffixes and functionality using Lua scripts.

## Quick Start

1. Create a new directory in `plugins/` (e.g., `plugins/my-plugin/`)
2. Create a `meta.toml` file with plugin metadata
3. Create an `init.lua` file with your plugin code
4. Restart the server to load the plugin

## Plugin Structure

```
plugins/
└── my-plugin/
    ├── meta.toml       # Plugin metadata
    └── init.lua        # Plugin code
```

## Metadata Format (`meta.toml`)

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
suffix = "-CUSTOM"           # The suffix this plugin handles (must start with -)
author = "Your Name"
description = "What this plugin does"
enabled = true               # Set to false to disable
timeout = 10                 # Execution timeout in seconds (default: 5)

[permissions]
network = true               # Allow HTTP requests
allowed_domains = [          # Whitelist of allowed domains
    "api.example.com"
]
cache_read = true            # Allow reading from cache
cache_write = true           # Allow writing to cache
user_agent = "MyPlugin/1.0"  # Custom User-Agent for HTTP requests (optional)
env_vars = [                 # Environment variables from .plugins.env to access
    "API_KEY",
    "API_SECRET"
]
```

**Plugin Configuration Options:**
- `timeout` - Maximum execution time in seconds for `handle_query` (default: 5, minimum: 1)

**Permission Options:**
- `user_agent` - Custom User-Agent string for HTTP requests (optional, default: "whois-server-plugin/<version>")
- `env_vars` - List of environment variable names from `.plugins.env` that this plugin can access (optional)

## Lua Plugin API

### Required Functions

#### `handle_query(query: string) -> string`

This function is called when a query with your plugin's suffix is received.

**Parameters:**
- `query` - The query string without the suffix (e.g., "beijing" for "beijing-WEATHER")

**Returns:**
- A formatted response string (typically in WHOIS format with `%` prefixes)

**Example:**
```lua
function handle_query(query)
    if not query or query == "" then
        return "% Error: Query parameter required\n"
    end

    -- Process the query
    local result = process(query)

    -- Format response
    return "% Result: " .. result .. "\n"
end
```

### Optional Functions

#### `init()`

Called when the plugin is loaded at server startup. Use this for setup tasks.

```lua
function init()
    log_info("My plugin initialized")
end
```

#### `cleanup()`

Called when the server shuts down. Use this for cleanup tasks.

```lua
function cleanup()
    log_info("My plugin cleanup")
end
```

## Available APIs

### HTTP Client

**`http_get(url: string) -> string`**

Make an HTTP GET request. The URL domain must be in the `allowed_domains` whitelist.

**Returns:** JSON string `{"status": 200, "body": "response text"}`

**Example:**
```lua
local response = http_get("https://api.example.com/data")
local data = json.decode(response)
if data.status == 200 then
    local body = data.body
    -- Process response
end
```

### Cache API

**`cache_get(key: string) -> string | nil`**

Get a value from the shared LMDB cache.

**Example:**
```lua
local value = cache_get("my-plugin:key")
if value then
    return value
end
```

**`cache_set(key: string, value: string, ttl: number?)`**

Set a value in the shared LMDB cache. TTL is in seconds (default: 3600).

**Example:**
```lua
cache_set("my-plugin:key", "cached value", 1800)  -- Cache for 30 minutes
```

### Logging API

**`log_info(message: string)`**
**`log_warn(message: string)`**
**`log_error(message: string)`**

Log messages using the server's logging system.

**Example:**
```lua
log_info("Processing query for: " .. query)
log_warn("API rate limit approaching")
log_error("Failed to fetch data")
```

### Environment Variable API

**`env_get(key: string) -> string`**

Get an environment variable value from `.plugins.env`. Only variables listed in `env_vars` in `meta.toml` are accessible.

**Returns:** The environment variable value as a string

**Example:**
```lua
local api_key = env_get("API_KEY")
local url = "https://api.example.com/data?key=" .. api_key
local response = http_get(url)
```

**`env_list() -> table`**

Get a list of all available environment variable names for this plugin.

**Returns:** Array of variable names

**Example:**
```lua
local vars = env_list()
for i, var_name in ipairs(vars) do
    log_info("Available env var: " .. var_name)
end
```

### The `.plugins.env` File

The `.plugins.env` file in the server root directory stores environment variables that plugins can access. This is useful for API keys, tokens, and other sensitive configuration.

**File Format:**
```
# Comment lines start with #
API_KEY=your_api_key_here
API_SECRET=your_secret_here
BASE_URL=https://api.example.com

# Values can be quoted or unquoted
DB_HOST="database.example.com"
DB_PORT=5432
```

**Security Notes:**
- Each plugin can only access environment variables explicitly listed in its `env_vars` configuration
- Never commit `.plugins.env` to version control (add it to `.gitignore`)
- Use environment variables for sensitive data instead of hardcoding in plugin files

## Complete Example

Here's a complete example of a plugin using custom User-Agent and environment variables:

```toml
# meta.toml
[plugin]
name = "weather"
version = "1.0.0"
suffix = "-WEATHER"
author = "Your Name"
description = "Get weather information"
enabled = true
timeout = 15

[permissions]
network = true
allowed_domains = ["api.weather.com"]
cache_read = true
cache_write = true
user_agent = "MyWeatherPlugin/1.0 (contact@example.com)"
env_vars = ["WEATHER_API_KEY"]
```

```lua
-- init.lua
local function fetch_weather(location)
    -- Check cache first
    local cached = cache_get("weather:" .. location)
    if cached then
        return cached
    end

    -- Get API key from environment
    local api_key = env_get("WEATHER_API_KEY")

    -- Fetch from API with custom User-Agent
    local url = "https://api.weather.com/v1/current?location=" .. location .. "&apikey=" .. api_key
    local ok, result = pcall(http_get, url)

    if not ok then
        log_error("Weather API request failed: " .. result)
        return nil
    end

    -- Parse response
    local data = parse_json(result)
    if data.status ~= 200 then
        return nil
    end

    -- Cache for 30 minutes
    cache_set("weather:" .. location, data.body, 1800)
    return data.body
end

function handle_query(query)
    if not query or query == "" then
        return "% Error: Location required\n"
    end

    log_info("Weather query for: " .. query)

    local weather = fetch_weather(query)
    if not weather then
        return "% Error: Failed to fetch weather data\n"
    end

    return "% Weather: " .. weather .. "\n"
end

function init()
    log_info("Weather plugin initialized with custom User-Agent")
end
```

**Corresponding `.plugins.env` file:**
```
# Weather API credentials
WEATHER_API_KEY=your_actual_api_key_here
```

## Security Model

Plugins run in a secure sandbox with the following restrictions:

- **No file I/O** - The `io` library is removed
- **No shell execution** - The `os` library is removed
- **No dynamic code loading** - `load`, `loadfile`, `dofile` are removed
- **No C modules** - `package.loadlib` is removed
- **Network whitelist** - HTTP requests only allowed to whitelisted domains
- **Resource limits** - 10 MB memory limit per plugin
- **Execution timeout** - Configurable timeout per plugin (default: 5 seconds, set via `timeout` in meta.toml)

## Testing Your Plugin

1. Place your plugin in `plugins/my-plugin/`
2. Restart the WHOIS server
3. Check logs for successful loading: `"Registered plugin 'my-plugin' with suffix '-CUSTOM'"`
4. Test with: `echo "test-CUSTOM" | nc localhost 43`

## Best Practices

1. **Error Handling**
   - Use `pcall` to wrap potentially failing operations
   - Return user-friendly error messages

2. **Caching**
   - Cache API responses to reduce load
   - Use appropriate TTL values
   - Prefix cache keys to avoid collisions (e.g., "plugin-name:key")

3. **Logging**
   - Log important events for debugging
   - Don't log sensitive data
   - Use appropriate log levels

4. **Performance**
   - Keep queries fast (default timeout is 5 seconds, configurable via `timeout` in meta.toml)
   - Minimize HTTP requests
   - Use caching effectively
   - Increase timeout only when necessary for long-running operations

5. **Response Format**
   - Follow WHOIS format with `%` prefixes for metadata
   - Include clear error messages
   - Keep output concise

## Troubleshooting

### Plugin Not Loading

- Check server logs for error messages
- Verify `meta.toml` syntax is correct
- Ensure `enabled = true`
- Verify both `meta.toml` and `init.lua` exist

### Network Errors

- Verify domains are in `allowed_domains`
- Check network connectivity
- Ensure URL format is correct

### Cache Errors

- Verify `cache_read` and `cache_write` permissions
- Check cache key format (strings only)

### Timeout Errors

- Reduce work done in `handle_query`
- Use caching to avoid repeated API calls
- Consider async operations
- Increase the `timeout` value in `meta.toml` if your operations legitimately need more time (e.g., for slow external APIs)
