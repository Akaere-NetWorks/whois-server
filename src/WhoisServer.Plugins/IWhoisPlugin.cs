namespace WhoisServer.Plugins;

/// <summary>
/// Contract every native plugin implements — the C# replacement for the Rust version's Lua
/// <c>handle_query(query) -&gt; string</c> global function (see plan.md's locked-in decision to
/// delete the Lua/mlua sandbox entirely rather than port it).
///
/// All plugin metadata (name, suffix, version, timeout, permissions) lives in <c>plugin.json</c>
/// (see <see cref="PluginManifest"/>), not on this interface — the loader needs that information
/// before it can safely decide whether to even instantiate the plugin (suffix-collision checks
/// happen first), so duplicating it as interface members would just be two sources of truth for
/// the same data.
/// </summary>
public interface IWhoisPlugin
{
    /// <summary><paramref name="query"/> is already suffix-stripped, exactly like every built-in
    /// service's <c>ProcessAsync(string baseQuery, ...)</c> convention elsewhere in this
    /// codebase.</summary>
    Task<string> ProcessAsync(string query, CancellationToken ct = default);
}
