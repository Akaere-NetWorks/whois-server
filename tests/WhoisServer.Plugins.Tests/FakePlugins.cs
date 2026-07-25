namespace WhoisServer.Plugins.Tests;

/// <summary>Well-behaved fake plugin — echoes its input immediately.</summary>
public sealed class EchoPlugin : IWhoisPlugin
{
    public Task<string> ProcessAsync(string query, CancellationToken ct = default) =>
        Task.FromResult($"echo: {query}");
}

/// <summary>Uncooperative fake plugin — deliberately ignores the <see cref="CancellationToken"/>
/// it's given and never completes, simulating a plugin that doesn't observe cancellation. Used to
/// verify <see cref="PluginHost.ProcessAsync"/>'s timeout gives up *waiting* rather than actually
/// (and unsafely) killing the plugin's task.</summary>
public sealed class UncooperativePlugin : IWhoisPlugin
{
    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        await Task.Delay(Timeout.InfiniteTimeSpan, CancellationToken.None);
        return "unreachable";
    }
}

internal static class TestManifests
{
    public static PluginManifest Create(string name, string suffix, int timeoutSeconds = 5) => new()
    {
        Name = name,
        Version = "0.0.1",
        Suffix = suffix,
        TimeoutSeconds = timeoutSeconds,
    };
}
