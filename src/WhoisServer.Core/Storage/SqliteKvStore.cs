using System.Runtime.CompilerServices;
using Microsoft.Data.Sqlite;

namespace WhoisServer.Core.Storage;

/// <summary>
/// SQLite-backed <see cref="IKvStore"/>. One file per logical store, WAL mode for concurrent
/// readers, TTL enforced lazily on read (expired rows are deleted the next time they're touched,
/// same pattern the Rust version used with LMDB — see plan.md §3).
/// </summary>
public sealed class SqliteKvStore : IKvStore, IDisposable
{
    private readonly string _connectionString;
    private readonly SemaphoreSlim _writeLock = new(1, 1);

    public SqliteKvStore(string dbPath)
    {
        var directory = Path.GetDirectoryName(dbPath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        _connectionString = new SqliteConnectionStringBuilder
        {
            DataSource = dbPath,
            Cache = SqliteCacheMode.Shared,
        }.ToString();

        using var connection = OpenConnection();
        using var pragma = connection.CreateCommand();
        pragma.CommandText = "PRAGMA journal_mode=WAL;";
        pragma.ExecuteNonQuery();

        using var create = connection.CreateCommand();
        create.CommandText =
            """
            CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                expires_at INTEGER NULL
            );
            """;
        create.ExecuteNonQuery();
    }

    private SqliteConnection OpenConnection()
    {
        var connection = new SqliteConnection(_connectionString);
        connection.Open();
        return connection;
    }

    private static long NowUnixSeconds() => DateTimeOffset.UtcNow.ToUnixTimeSeconds();

    public async Task<string?> GetAsync(string key, CancellationToken ct = default)
    {
        await using var connection = OpenConnection();
        await using var select = connection.CreateCommand();
        select.CommandText = "SELECT value, expires_at FROM kv WHERE key = $key;";
        select.Parameters.AddWithValue("$key", key);

        await using var reader = await select.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        var value = reader.GetString(0);
        if (!reader.IsDBNull(1))
        {
            var expiresAt = reader.GetInt64(1);
            if (expiresAt <= NowUnixSeconds())
            {
                await reader.DisposeAsync();
                await connection.CloseAsync();
                await DeleteAsync(key, ct);
                return null;
            }
        }

        return value;
    }

    public async Task PutAsync(string key, string value, TimeSpan? ttl = null, CancellationToken ct = default)
    {
        await _writeLock.WaitAsync(ct);
        try
        {
            await using var connection = OpenConnection();
            await using var upsert = connection.CreateCommand();
            upsert.CommandText =
                """
                INSERT INTO kv (key, value, expires_at) VALUES ($key, $value, $expiresAt)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, expires_at = excluded.expires_at;
                """;
            upsert.Parameters.AddWithValue("$key", key);
            upsert.Parameters.AddWithValue("$value", value);
            upsert.Parameters.AddWithValue("$expiresAt", ttl.HasValue ? NowUnixSeconds() + (long)ttl.Value.TotalSeconds : DBNull.Value);
            await upsert.ExecuteNonQueryAsync(ct);
        }
        finally
        {
            _writeLock.Release();
        }
    }

    public async Task DeleteAsync(string key, CancellationToken ct = default)
    {
        await _writeLock.WaitAsync(ct);
        try
        {
            await using var connection = OpenConnection();
            await using var delete = connection.CreateCommand();
            delete.CommandText = "DELETE FROM kv WHERE key = $key;";
            delete.Parameters.AddWithValue("$key", key);
            await delete.ExecuteNonQueryAsync(ct);
        }
        finally
        {
            _writeLock.Release();
        }
    }

    public async Task<bool> ExistsAsync(string key, CancellationToken ct = default) =>
        await GetAsync(key, ct) is not null;

    public async Task<IReadOnlyList<string>> ListKeysAsync(CancellationToken ct = default) =>
        await ListKeysWithPrefixAsync(string.Empty, ct);

    public async Task<IReadOnlyList<string>> ListKeysWithPrefixAsync(string prefix, CancellationToken ct = default)
    {
        var now = NowUnixSeconds();
        await using var connection = OpenConnection();
        await using var select = connection.CreateCommand();
        select.CommandText =
            "SELECT key FROM kv WHERE key LIKE $pattern ESCAPE '\\' AND (expires_at IS NULL OR expires_at > $now);";
        select.Parameters.AddWithValue("$pattern", EscapeLikePattern(prefix) + "%");
        select.Parameters.AddWithValue("$now", now);

        var keys = new List<string>();
        await using var reader = await select.ExecuteReaderAsync(ct);
        while (await reader.ReadAsync(ct))
        {
            keys.Add(reader.GetString(0));
        }

        return keys;
    }

    public async IAsyncEnumerable<(string Key, string Value)> IteratePrefixAsync(
        string prefix, [EnumeratorCancellation] CancellationToken ct = default)
    {
        var now = NowUnixSeconds();
        await using var connection = OpenConnection();
        await using var select = connection.CreateCommand();
        select.CommandText =
            "SELECT key, value FROM kv WHERE key LIKE $pattern ESCAPE '\\' AND (expires_at IS NULL OR expires_at > $now);";
        select.Parameters.AddWithValue("$pattern", EscapeLikePattern(prefix) + "%");
        select.Parameters.AddWithValue("$now", now);

        await using var reader = await select.ExecuteReaderAsync(ct);
        while (await reader.ReadAsync(ct))
        {
            yield return (reader.GetString(0), reader.GetString(1));
        }
    }

    private static string EscapeLikePattern(string value) =>
        value.Replace("\\", "\\\\").Replace("%", "\\%").Replace("_", "\\_");

    public void Dispose() => _writeLock.Dispose();
}
