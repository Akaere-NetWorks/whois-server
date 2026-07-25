using System.Collections.Concurrent;

namespace WhoisServer.Plugins;

/// <summary>
/// Suffix -&gt; loaded-plugin map, mirroring the Rust <c>PluginRegistry</c>'s shape. Registered as
/// an empty DI singleton before the host's <c>WebApplicationBuilder.Build()</c> (so
/// <c>QueryProcessor</c> can take a dependency on it), then populated by <see cref="PluginLoader"/>
/// afterwards, once the DI container (and therefore <c>IHttpClientFactory</c>/
/// <c>ILoggerFactory</c>) actually exists — the same "singleton registered early, populated by an
/// explicit async step called from <c>Program.cs</c>" pattern this codebase already uses for
/// <c>PatchManager</c> (see <c>HostFactory.LoadPatchesAsync</c>).
/// </summary>
public sealed class PluginRegistry
{
    private readonly ConcurrentDictionary<string, PluginHost> _bySuffix = new(StringComparer.OrdinalIgnoreCase);

    /// <returns><see langword="false"/> if <paramref name="host"/>'s suffix is already claimed by
    /// another plugin — the caller is expected to log and skip, not throw, matching the Rust
    /// loader's "failed to register, skip this one, keep loading the rest" behavior.</returns>
    public bool Register(PluginHost host) => _bySuffix.TryAdd(host.Suffix, host);

    public bool IsSuffixTaken(string suffix) => _bySuffix.ContainsKey(suffix);

    public Task<string> ProcessAsync(string suffix, string baseQuery, CancellationToken ct = default) =>
        _bySuffix.TryGetValue(suffix, out var host)
            ? host.ProcessAsync(baseQuery, ct)
            : Task.FromResult($"% No plugin registered for suffix '{suffix}'\n");

    public IReadOnlyCollection<string> Suffixes => (IReadOnlyCollection<string>)_bySuffix.Keys;

    public int Count => _bySuffix.Count;

    /// <summary>Unloads and forgets every loaded plugin — used by tests to verify
    /// <see cref="PluginLoadContext"/> unloadability, and available for a future hot-reload
    /// feature (not wired up to any CLI/API surface yet).</summary>
    public void UnloadAll()
    {
        foreach (var host in _bySuffix.Values)
        {
            host.Unload();
        }

        _bySuffix.Clear();
    }

    /// <summary>Test-only hook (see AssemblyInfo.cs's <c>InternalsVisibleTo</c>) — must be called
    /// *before* <see cref="UnloadAll"/>, since the returned <see cref="WeakReference"/> is only
    /// useful if taken while the context is still reachable through this registry.</summary>
    internal WeakReference GetLoadContextWeakReferenceForTesting(string suffix) =>
        _bySuffix.TryGetValue(suffix, out var host)
            ? host.GetLoadContextWeakReferenceForTesting()
            : throw new KeyNotFoundException($"No plugin registered for suffix '{suffix}'");
}
