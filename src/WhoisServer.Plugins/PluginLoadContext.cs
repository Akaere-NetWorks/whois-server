using System.Reflection;
using System.Runtime.Loader;

namespace WhoisServer.Plugins;

/// <summary>
/// One collectible <see cref="AssemblyLoadContext"/> per loaded plugin — the standard
/// Microsoft-documented .NET plugin-isolation pattern. Types the plugin needs from the host
/// (<c>WhoisServer.Plugins</c>, <c>WhoisServer.Core</c>, the BCL) resolve back to the host's own
/// <see cref="AssemblyLoadContext.Default"/> copies via <see cref="Load"/> returning
/// <see langword="null"/> for anything it can't find as one of the plugin's own private
/// dependencies — only the plugin's own assembly (and any private dependencies shipped alongside
/// it) actually get isolated into this context. <c>isCollectible: true</c> is what makes
/// <see cref="Unload"/> possible (plan.md's Phase 6 "unloadability" requirement) — actually
/// reclaiming the memory still requires the host to drop every last reference to types/instances
/// from this context and force a GC, same as any collectible ALC.
/// </summary>
public sealed class PluginLoadContext(string pluginDllPath) : AssemblyLoadContext(isCollectible: true)
{
    private readonly AssemblyDependencyResolver _resolver = new(pluginDllPath);

    protected override Assembly? Load(AssemblyName assemblyName)
    {
        var path = _resolver.ResolveAssemblyToPath(assemblyName);
        return path is not null ? LoadFromAssemblyPath(path) : null;
    }

    protected override nint LoadUnmanagedDll(string unmanagedDllName)
    {
        var path = _resolver.ResolveUnmanagedDllToPath(unmanagedDllName);
        return path is not null ? LoadUnmanagedDllFromPath(path) : nint.Zero;
    }
}
