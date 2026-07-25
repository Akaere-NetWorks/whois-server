namespace WhoisServer.Plugins.Tests;

public class PluginRegistryTests
{
    private static PluginLoadContext DummyLoadContext() => new(typeof(PluginRegistryTests).Assembly.Location);

    [Fact]
    public async Task RoutesToTheCorrectPluginBySuffix()
    {
        var registry = new PluginRegistry();
        registry.Register(new PluginHost(TestManifests.Create("a", "-AAA"), new EchoPlugin(), DummyLoadContext()));
        registry.Register(new PluginHost(TestManifests.Create("b", "-BBB"), new EchoPlugin(), DummyLoadContext()));

        Assert.Equal("echo: 1", await registry.ProcessAsync("-AAA", "1"));
        Assert.Equal("echo: 2", await registry.ProcessAsync("-BBB", "2"));
    }

    [Fact]
    public void Register_RejectsASecondPluginClaimingTheSameSuffix()
    {
        var registry = new PluginRegistry();
        var first = registry.Register(new PluginHost(TestManifests.Create("a", "-DUP"), new EchoPlugin(), DummyLoadContext()));
        var second = registry.Register(new PluginHost(TestManifests.Create("b", "-DUP"), new EchoPlugin(), DummyLoadContext()));

        Assert.True(first);
        Assert.False(second);
        Assert.Equal(1, registry.Count);
    }

    [Fact]
    public async Task ProcessAsync_ReturnsAMessage_ForAnUnregisteredSuffix()
    {
        var registry = new PluginRegistry();

        var result = await registry.ProcessAsync("-NOPE", "query");

        Assert.Contains("No plugin registered", result);
    }
}
