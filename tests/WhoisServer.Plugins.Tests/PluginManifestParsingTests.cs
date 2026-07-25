using System.Text.Json;

namespace WhoisServer.Plugins.Tests;

public class PluginManifestParsingTests
{
    private static readonly JsonSerializerOptions Options = new() { PropertyNameCaseInsensitive = true };

    [Fact]
    public void ParsesFullManifest()
    {
        const string json = """
            {
              "name": "weather",
              "version": "1.0.0",
              "suffix": "-WEATHER",
              "author": "Akaere Networks",
              "description": "demo",
              "enabled": true,
              "timeoutSeconds": 5,
              "entryPoint": "Weather.dll",
              "permissions": {
                "network": true,
                "allowedDomains": ["wttr.in"],
                "cacheRead": true,
                "cacheWrite": true,
                "userAgent": "custom-ua/1.0",
                "envVars": ["API_KEY"]
              }
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal("weather", manifest.Name);
        Assert.Equal("1.0.0", manifest.Version);
        Assert.Equal("-WEATHER", manifest.Suffix);
        Assert.True(manifest.Enabled);
        Assert.Equal(5, manifest.TimeoutSeconds);
        Assert.Equal("Weather.dll", manifest.EntryPoint);
        Assert.True(manifest.Permissions.Network);
        Assert.Equal(["wttr.in"], manifest.Permissions.AllowedDomains);
        Assert.True(manifest.Permissions.CacheRead);
        Assert.True(manifest.Permissions.CacheWrite);
        Assert.Equal("custom-ua/1.0", manifest.Permissions.UserAgent);
        Assert.Equal(["API_KEY"], manifest.Permissions.EnvVars);
    }

    [Fact]
    public void DefaultsApplyWhenFieldsOmitted()
    {
        const string json = """{"name": "minimal", "version": "0.1.0", "suffix": "-MIN"}""";

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.True(manifest.Enabled);
        Assert.Equal(5, manifest.TimeoutSeconds);
        Assert.Null(manifest.EntryPoint);
        Assert.False(manifest.Permissions.Network);
        Assert.Empty(manifest.Permissions.AllowedDomains);
        Assert.False(manifest.Permissions.CacheRead);
        Assert.False(manifest.Permissions.CacheWrite);
    }
}
