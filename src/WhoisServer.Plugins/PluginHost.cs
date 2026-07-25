namespace WhoisServer.Plugins;

/// <summary>
/// A successfully loaded plugin: its manifest, its instance, and the
/// <see cref="PluginLoadContext"/> it was loaded into. Enforces the manifest's per-query
/// <c>timeoutSeconds</c> budget around every call.
/// </summary>
public sealed class PluginHost(PluginManifest manifest, IWhoisPlugin instance, PluginLoadContext loadContext)
{
    public PluginManifest Manifest { get; } = manifest;

    public string Suffix => Manifest.Suffix;

    public string Name => Manifest.Name;

    /// <summary>Runs the plugin, giving up on *waiting* for it (not cancelling it — see
    /// <see cref="PluginManifest.TimeoutSeconds"/>'s doc comment) once its timeout elapses.</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        var timeout = TimeSpan.FromSeconds(Manifest.TimeoutSeconds);

        try
        {
            return await instance.ProcessAsync(baseQuery, cts.Token).WaitAsync(timeout, ct);
        }
        catch (TimeoutException)
        {
            return $"""
                % Plugin '{Name}' timed out after {Manifest.TimeoutSeconds}s
                % The plugin may still be running in the background; it was only given up on.

                """;
        }
        catch (Exception e)
        {
            return $"""
                % Plugin '{Name}' failed: {e.Message}

                """;
        }
    }

    /// <summary>Requests the plugin's <see cref="PluginLoadContext"/> unload. Actually reclaiming
    /// the memory requires the caller to also drop this <see cref="PluginHost"/> (and anything
    /// else referencing the plugin instance or its types) and force a GC — see plan.md's Phase 6
    /// notes and <c>PluginLoaderEndToEndTests.Unload_ActuallyReclaimsTheLoadContext</c> for the
    /// verification pattern.</summary>
    public void Unload() => loadContext.Unload();

    /// <summary>Test-only hook (see <see cref="System.Runtime.CompilerServices.InternalsVisibleToAttribute"/>
    /// in AssemblyInfo.cs) — the standard collectible-<see cref="System.Runtime.Loader.AssemblyLoadContext"/>
    /// verification pattern needs a <see cref="WeakReference"/> to the context itself, taken
    /// *before* every strong reference (including this <see cref="PluginHost"/>) is dropped.</summary>
    internal WeakReference GetLoadContextWeakReferenceForTesting() => new(loadContext);
}
