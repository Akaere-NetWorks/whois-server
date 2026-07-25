namespace WhoisServer.Plugins;

/// <summary>
/// Parsed <c>plugin.json</c> — the C#/JSON replacement for the Rust version's <c>meta.toml</c>
/// (plain <c>System.Text.Json</c> is enough here; <c>Tomlyn</c> is no longer needed since nothing
/// else in this port reads TOML, per plan.md's Phase 6 notes).
/// </summary>
public sealed class PluginManifest
{
    public required string Name { get; init; }
    public required string Version { get; init; }
    public required string Suffix { get; init; }
    public string? Author { get; init; }
    public string? Description { get; init; }
    public bool Enabled { get; init; } = true;

    /// <summary>Per-query execution budget. Enforced via <see cref="Task.WaitAsync(TimeSpan)"/>
    /// around the plugin's <see cref="IWhoisPlugin.ProcessAsync"/> call — this stops the host from
    /// *waiting* on a runaway plugin past this many seconds, but (same caveat as the Rust
    /// version's <c>CancellationTokenSource</c>-based timeout) cannot forcibly kill a plugin that
    /// ignores its <see cref="CancellationToken"/> and keeps running in the background. There is
    /// no safe way to hard-kill arbitrary managed code sharing this process without a separate-
    /// process sandbox, which is out of scope here.</summary>
    public int TimeoutSeconds { get; init; } = 5;

    /// <summary>The plugin's compiled assembly, relative to its manifest's directory. Defaults to
    /// <c>&lt;plugin-folder-name&gt;.dll</c> when omitted (the common case for a plugin built from
    /// a single companion project) — Rust's Lua plugins never needed this field since they always
    /// loaded a hardcoded <c>init.lua</c>, but compiled .NET assemblies can be named anything.</summary>
    public string? EntryPoint { get; init; }

    public PluginPermissions Permissions { get; init; } = new();
}

public sealed class PluginPermissions
{
    /// <summary>Whether the plugin gets an <see cref="HttpClient"/> injected at all. A plugin
    /// whose constructor requests <see cref="HttpClient"/> while this is <c>false</c> fails to
    /// load with a clear error, rather than silently receiving a client that will 403/hang.</summary>
    public bool Network { get; init; }

    /// <summary>When non-empty, the injected <see cref="HttpClient"/> rejects (via
    /// <see cref="DomainWhitelistHttpHandler"/>) any request whose host isn't in this list —
    /// same intent as the Rust sandbox's <c>http_get</c> domain whitelist, just enforced through a
    /// <see cref="DelegatingHandler"/> instead of a Lua-callable wrapper function, since there's no
    /// VM boundary here to hang the check off of.</summary>
    public List<string> AllowedDomains { get; init; } = [];

    public string? UserAgent { get; init; }

    /// <summary>Whether the plugin gets an <see cref="Core.Storage.IKvStore"/> injected at all
    /// (namespaced under <c>plugin:{name}:</c>, see <see cref="NamespacedKvStore"/>).</summary>
    public bool CacheRead { get; init; }

    public bool CacheWrite { get; init; }

    /// <summary>Names of keys to pull from <c>plugins/.plugins.env</c> (same
    /// <c>KEY=VALUE</c>-per-line format as the Rust version) into the
    /// <c>IReadOnlyDictionary&lt;string, string&gt;</c> a plugin's constructor may request —
    /// explicit allowlist per plugin, not blanket access to the whole file.</summary>
    public List<string> EnvVars { get; init; } = [];
}
