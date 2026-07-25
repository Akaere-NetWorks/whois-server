namespace WhoisServer.Core.Storage;

/// <summary>
/// Embedded string key-value store — the SQLite-backed replacement for the Rust version's
/// per-subsystem LMDB environments (plan.md §2/§3). One store per logical dataset (stats,
/// patches, IANA referral cache, ...), not one shared store, mirroring the original's
/// one-environment-per-directory layout.
/// </summary>
public interface IKvStore
{
    Task<string?> GetAsync(string key, CancellationToken ct = default);

    /// <summary>Upserts <paramref name="key"/>. A non-null <paramref name="ttl"/> makes the entry
    /// expire (checked lazily on read, like the Rust version's manual TTL checks) after that long.</summary>
    Task PutAsync(string key, string value, TimeSpan? ttl = null, CancellationToken ct = default);

    Task DeleteAsync(string key, CancellationToken ct = default);

    Task<bool> ExistsAsync(string key, CancellationToken ct = default);

    Task<IReadOnlyList<string>> ListKeysAsync(CancellationToken ct = default);

    Task<IReadOnlyList<string>> ListKeysWithPrefixAsync(string prefix, CancellationToken ct = default);

    /// <summary>Like <see cref="ListKeysWithPrefixAsync"/> but returns values too, in one scan —
    /// for callers that need to filter/search by value (e.g. PEN's fuzzy search over ~50k
    /// entries), avoiding an N+1 <see cref="GetAsync"/> per key.</summary>
    IAsyncEnumerable<(string Key, string Value)> IteratePrefixAsync(string prefix, CancellationToken ct = default);
}
