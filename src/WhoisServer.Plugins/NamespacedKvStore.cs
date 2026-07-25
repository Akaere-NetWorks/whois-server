using WhoisServer.Core.Storage;

namespace WhoisServer.Plugins;

/// <summary>
/// Wraps the host's shared <see cref="IKvStore"/> with a <c>plugin:{name}:</c> key prefix (so
/// plugins can't collide with each other's cache keys or the host's own) and enforces the
/// manifest's <c>cacheRead</c>/<c>cacheWrite</c> permissions by throwing rather than silently
/// allowing an operation the plugin didn't declare — stricter than the Rust version's sandbox,
/// which only conditionally *registered* the Lua `cache_get`/`cache_set` globals in the first
/// place (a plugin that didn't request read access simply had no `cache_get` function to call);
/// since a native plugin's constructor always receives a concrete <see cref="IKvStore"/> instance
/// rather than an optionally-absent global function, permission has to be enforced per-call
/// instead.
/// </summary>
public sealed class NamespacedKvStore(IKvStore inner, string pluginName, bool canRead, bool canWrite) : IKvStore
{
    private readonly string _prefix = $"plugin:{pluginName}:";

    private string Key(string key) => _prefix + key;

    public Task<string?> GetAsync(string key, CancellationToken ct = default)
    {
        if (!canRead)
        {
            throw new InvalidOperationException($"Plugin '{pluginName}' does not have cacheRead permission");
        }

        return inner.GetAsync(Key(key), ct);
    }

    public Task PutAsync(string key, string value, TimeSpan? ttl = null, CancellationToken ct = default)
    {
        if (!canWrite)
        {
            throw new InvalidOperationException($"Plugin '{pluginName}' does not have cacheWrite permission");
        }

        return inner.PutAsync(Key(key), value, ttl, ct);
    }

    public Task DeleteAsync(string key, CancellationToken ct = default)
    {
        if (!canWrite)
        {
            throw new InvalidOperationException($"Plugin '{pluginName}' does not have cacheWrite permission");
        }

        return inner.DeleteAsync(Key(key), ct);
    }

    public async Task<bool> ExistsAsync(string key, CancellationToken ct = default) =>
        canRead && await inner.ExistsAsync(Key(key), ct);

    public async Task<IReadOnlyList<string>> ListKeysAsync(CancellationToken ct = default) =>
        await ListKeysWithPrefixAsync(string.Empty, ct);

    public async Task<IReadOnlyList<string>> ListKeysWithPrefixAsync(string prefix, CancellationToken ct = default)
    {
        if (!canRead)
        {
            throw new InvalidOperationException($"Plugin '{pluginName}' does not have cacheRead permission");
        }

        var keys = await inner.ListKeysWithPrefixAsync(_prefix + prefix, ct);
        return [.. keys.Select(k => k[_prefix.Length..])];
    }

    public async IAsyncEnumerable<(string Key, string Value)> IteratePrefixAsync(
        string prefix, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        if (!canRead)
        {
            throw new InvalidOperationException($"Plugin '{pluginName}' does not have cacheRead permission");
        }

        await foreach (var (key, value) in inner.IteratePrefixAsync(_prefix + prefix, ct))
        {
            yield return (key[_prefix.Length..], value);
        }
    }
}
