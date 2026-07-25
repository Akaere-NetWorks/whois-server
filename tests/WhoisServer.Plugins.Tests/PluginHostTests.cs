using System.Diagnostics;

namespace WhoisServer.Plugins.Tests;

public class PluginHostTests
{
    // A real AssemblyDependencyResolver needs *some* valid assembly path; these tests never
    // actually exercise loading/unloading through it, only PluginHost.ProcessAsync's own timeout
    // logic, so the test assembly's own location is a fine stand-in.
    private static PluginLoadContext DummyLoadContext() => new(typeof(PluginHostTests).Assembly.Location);

    [Fact]
    public async Task ProcessAsync_ReturnsPluginResult_WhenWithinTimeout()
    {
        var host = new PluginHost(TestManifests.Create("echo", "-ECHOTEST"), new EchoPlugin(), DummyLoadContext());

        var result = await host.ProcessAsync("hello");

        Assert.Equal("echo: hello", result);
    }

    [Fact]
    public async Task ProcessAsync_GivesUpAfterTimeout_WithoutHangingTheCaller()
    {
        var host = new PluginHost(
            TestManifests.Create("uncooperative", "-SLOWTEST", timeoutSeconds: 1),
            new UncooperativePlugin(),
            DummyLoadContext());

        var stopwatch = Stopwatch.StartNew();
        var result = await host.ProcessAsync("anything");
        stopwatch.Stop();

        Assert.Contains("timed out", result, StringComparison.OrdinalIgnoreCase);
        // Generous upper bound — the point is "didn't wait for the plugin's Timeout.InfiniteTimeSpan
        // delay", not a tight timing assertion that could flake under CI load.
        Assert.True(stopwatch.Elapsed < TimeSpan.FromSeconds(10), $"Took {stopwatch.Elapsed}, expected ~1s timeout");
    }
}
