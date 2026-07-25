using Microsoft.Extensions.Logging.Abstractions;
using WhoisServer.Core.Query;
using WhoisServer.Core.Storage;

namespace WhoisServer.Plugins.Tests;

/// <summary>
/// Exercises <see cref="PluginLoader"/> against the real, compiled <c>weather</c> plugin under
/// the repo's <c>plugins/</c> directory (built by <c>plugins-src/Weather/Weather.csproj</c> as
/// part of the same solution) — not a hand-rolled fake, so this also proves the
/// shared-assembly-isolation fix in Weather.csproj (Private=false/ExcludeAssets=runtime on its
/// WhoisServer.Plugins/WhoisServer.Core references) actually works end-to-end: if it didn't, the
/// loader's <c>typeof(IWhoisPlugin).IsAssignableFrom(...)</c> check would fail and every test here
/// would report "0 plugins loaded".
/// </summary>
public sealed class PluginLoaderEndToEndTests : IDisposable
{
    private readonly string _tempCacheDb = Path.Combine(Path.GetTempPath(), $"plugin-tests-{Guid.NewGuid():N}.db");

    private static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !File.Exists(Path.Combine(dir.FullName, "whois-server.slnx")))
        {
            dir = dir.Parent;
        }

        return dir?.FullName ?? throw new InvalidOperationException("Could not locate repo root from test bin output");
    }

    private IKvStore SharedCache() => new SqliteKvStore(_tempCacheDb);

    [Fact]
    public async Task LoadAllAsync_LoadsTheRealWeatherPlugin_AndItAnswersQueries()
    {
        var pluginsDir = Path.Combine(RepoRoot(), "plugins");
        Assert.True(
            File.Exists(Path.Combine(pluginsDir, "weather", "plugin.json")),
            $"plugins/weather/plugin.json not found under {pluginsDir} — build plugins-src/Weather first " +
            "(it's part of the solution, so `dotnet build`/`dotnet test` on the whole .slnx builds it too).");

        var registry = new PluginRegistry();
        var loaded = await PluginLoader.LoadAllAsync(registry, pluginsDir, SharedCache(), NullLoggerFactory.Instance);

        Assert.True(loaded >= 1);
        Assert.True(QueryAnalyzer.IsSuffixRegistered("-WEATHER"));
        Assert.Equal(1, registry.Suffixes.Count(s => s == "-WEATHER"));

        var result = await registry.ProcessAsync("-WEATHER", "London");
        Assert.Contains("Weather Information", result);
    }

    [Fact]
    public async Task LoadAllAsync_SkipsAPluginWhoseSuffixIsAlreadyRegistered()
    {
        // Self-contained: this test project never runs WhoisServer.Services' registration code
        // (NetworkServicesRegistration etc.), so a *real* built-in suffix like "-BGPTOOL" is NOT
        // actually registered in QueryAnalyzer here — using one would make this test pass for the
        // wrong reason (or fail, depending on process-wide test ordering). Registering a synthetic
        // suffix directly is what actually proves the loader's collision check consults
        // QueryAnalyzer's real shared state, not just its own registry.
        const string suffix = "-ZZZPRECOLLISIONTEST";
        QueryAnalyzer.RegisterSuffix(suffix, baseQuery => new QueryType.Unknown(baseQuery));

        var tempDir = Path.Combine(Path.GetTempPath(), $"plugin-collision-{Guid.NewGuid():N}");
        var pluginDir = Path.Combine(tempDir, "fakeplugin");
        Directory.CreateDirectory(pluginDir);
        try
        {
            // Reusing the real Weather.dll as the entry point (any assembly implementing
            // IWhoisPlugin would do) — its constructor needs network + cache permissions, granted
            // here so the plugin actually loads successfully into a PluginHost and is rejected
            // SPECIFICALLY by the suffix-collision check, not incidentally by a permissions
            // mismatch (which would produce the same "0 loaded" outcome for the wrong reason).
            var weatherDll = Path.Combine(RepoRoot(), "plugins", "weather", "Weather.dll");
            File.Copy(weatherDll, Path.Combine(pluginDir, "Weather.dll"));
            await File.WriteAllTextAsync(Path.Combine(pluginDir, "plugin.json"), $$"""
                {
                  "name": "fake-plugin", "version": "1.0.0", "suffix": "{{suffix}}", "entryPoint": "Weather.dll",
                  "permissions": { "network": true, "cacheRead": true, "cacheWrite": true }
                }
                """);

            var registry = new PluginRegistry();
            var loaded = await PluginLoader.LoadAllAsync(registry, tempDir, SharedCache(), NullLoggerFactory.Instance);

            Assert.Equal(0, loaded);
            Assert.Equal(0, registry.Count);
            Assert.False(registry.IsSuffixTaken(suffix));
        }
        finally
        {
            Directory.Delete(tempDir, recursive: true);
        }
    }

    [Fact]
    public async Task Unload_ActuallyReclaimsTheLoadContext()
    {
        var tempDir = Path.Combine(Path.GetTempPath(), $"plugin-unload-{Guid.NewGuid():N}");
        var pluginDir = Path.Combine(tempDir, "unloadme");
        Directory.CreateDirectory(pluginDir);
        try
        {
            var weatherDll = Path.Combine(RepoRoot(), "plugins", "weather", "Weather.dll");
            File.Copy(weatherDll, Path.Combine(pluginDir, "Weather.dll"));
            await File.WriteAllTextAsync(Path.Combine(pluginDir, "plugin.json"), """
                {
                  "name": "unload-me", "version": "1.0.0", "suffix": "-ZZZUNLOADTEST", "entryPoint": "Weather.dll",
                  "permissions": { "network": true, "cacheRead": true, "cacheWrite": true }
                }
                """);

            var registry = new PluginRegistry();
            await PluginLoader.LoadAllAsync(registry, tempDir, SharedCache(), NullLoggerFactory.Instance);
            Assert.Equal(1, registry.Count);

            // Isolated into its own non-inlined method so the JIT can't keep the PluginHost (and
            // therefore the PluginLoadContext) reachable via a lingering stack slot in this method
            // across the GC.Collect() calls below — the standard shape for this verification
            // pattern (grab the weak reference, unload, THEN drop every strong reference).
            var weakRef = UnloadAndGetWeakReference(registry);

            // A single GC.Collect()+WaitForPendingFinalizers() round can leave the context alive
            // for one more pass even in a from-scratch repro outside any test runner (confirmed
            // empirically) — a second collect plus a short delay reliably finishes the job.
            for (var i = 0; i < 20 && weakRef.IsAlive; i++)
            {
                GC.Collect();
                GC.WaitForPendingFinalizers();
                GC.Collect();
                if (weakRef.IsAlive)
                {
                    await Task.Delay(50);
                }
            }

            Assert.False(weakRef.IsAlive, "PluginLoadContext was still alive after Unload() + forced GC");
        }
        finally
        {
            Directory.Delete(tempDir, recursive: true);
        }
    }

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    private static WeakReference UnloadAndGetWeakReference(PluginRegistry registry)
    {
        var weakRef = registry.GetLoadContextWeakReferenceForTesting("-ZZZUNLOADTEST");
        registry.UnloadAll();
        return weakRef;
    }

    public void Dispose()
    {
        try
        {
            File.Delete(_tempCacheDb);
        }
        catch
        {
            // Best-effort cleanup.
        }
    }
}
