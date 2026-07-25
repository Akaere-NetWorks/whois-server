using Microsoft.Extensions.Logging;
using WhoisServer.Core.Storage;
using WhoisServer.Plugins;

namespace Weather;

/// <summary>
/// Native C# reimplementation of plugins/weather/init.lua — demonstrates the plugin API (HTTP with
/// a domain whitelist, a namespaced cache, structured logging) by looking up weather via wttr.in.
/// Not a line-by-line port: the Lua version hand-rolled JSON string-searching to pull a "body"
/// field out of a raw JSON response because Lua has no JSON library available in the sandbox; this
/// version just uses <see cref="System.Net.Http.Json.HttpClientJsonExtensions"/> since normal
/// .NET has one.
/// </summary>
public sealed class WeatherPlugin(HttpClient httpClient, IKvStore cache, ILogger logger) : IWhoisPlugin
{
    private static readonly TimeSpan CacheDuration = TimeSpan.FromMinutes(30);

    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        var location = query.Trim();
        if (location.Length == 0)
        {
            return """
                % Error: Location parameter required
                %
                % Usage: <location>-WEATHER
                % Example: beijing-WEATHER
                %          london-WEATHER
                %          new-york-WEATHER

                """;
        }

        logger.LogInformation("Weather query for location: {Location}", location);

        var weather = await FetchWeatherAsync(location, ct);
        if (weather is null)
        {
            return $"% Error: Failed to fetch weather data for '{location}'\n";
        }

        return $"""
            % Weather Information
            % Location: {location}
            % Condition: {weather}
            % Source: wttr.in
            """;
    }

    private async Task<string?> FetchWeatherAsync(string location, CancellationToken ct)
    {
        var cacheKey = $"weather:example:{location}";
        var cached = await cache.GetAsync(cacheKey, ct);
        if (cached is not null)
        {
            return cached;
        }

        try
        {
            var url = $"https://wttr.in/{Uri.EscapeDataString(location)}?format=3";
            var weather = (await httpClient.GetStringAsync(url, ct)).Trim();
            if (weather.Length == 0)
            {
                return null;
            }

            await cache.PutAsync(cacheKey, weather, CacheDuration, ct);
            return weather;
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "wttr.in request failed for {Location}", location);
            return null;
        }
    }
}
