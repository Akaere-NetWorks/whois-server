using WhoisServer.Core.Storage;
using Xunit;

namespace WhoisServer.Core.Tests;

public class SqliteKvStoreTests : IDisposable
{
    private readonly string _dbPath;
    private readonly SqliteKvStore _store;

    public SqliteKvStoreTests()
    {
        _dbPath = Path.Combine(Path.GetTempPath(), $"whois-kv-test-{Guid.NewGuid():N}.db");
        _store = new SqliteKvStore(_dbPath);
    }

    public void Dispose()
    {
        _store.Dispose();
        File.Delete(_dbPath);
        foreach (var extra in new[] { "-wal", "-shm" })
        {
            var path = _dbPath + extra;
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }

        GC.SuppressFinalize(this);
    }

    [Fact]
    public async Task PutThenGet_RoundTrips()
    {
        await _store.PutAsync("key1", "value1");
        Assert.Equal("value1", await _store.GetAsync("key1"));
    }

    [Fact]
    public async Task Get_ReturnsNullForMissingKey()
    {
        Assert.Null(await _store.GetAsync("does-not-exist"));
    }

    [Fact]
    public async Task Put_UpsertsExistingKey()
    {
        await _store.PutAsync("key1", "first");
        await _store.PutAsync("key1", "second");
        Assert.Equal("second", await _store.GetAsync("key1"));
    }

    [Fact]
    public async Task Delete_RemovesKey()
    {
        await _store.PutAsync("key1", "value1");
        await _store.DeleteAsync("key1");
        Assert.Null(await _store.GetAsync("key1"));
    }

    [Fact]
    public async Task ExpiredEntry_IsTreatedAsMissing()
    {
        await _store.PutAsync("key1", "value1", TimeSpan.FromSeconds(-1));
        Assert.Null(await _store.GetAsync("key1"));
        Assert.False(await _store.ExistsAsync("key1"));
    }

    [Fact]
    public async Task UnexpiredEntry_IsReturned()
    {
        await _store.PutAsync("key1", "value1", TimeSpan.FromHours(1));
        Assert.Equal("value1", await _store.GetAsync("key1"));
    }

    [Fact]
    public async Task ListKeysWithPrefix_FiltersCorrectly()
    {
        await _store.PutAsync("meta:a", "1");
        await _store.PutAsync("meta:b", "2");
        await _store.PutAsync("patch:a", "3");

        var keys = await _store.ListKeysWithPrefixAsync("meta:");

        Assert.Equal(2, keys.Count);
        Assert.Contains("meta:a", keys);
        Assert.Contains("meta:b", keys);
    }

    [Fact]
    public async Task ListKeysWithPrefix_DoesNotTreatUnderscoreAsWildcard()
    {
        // LIKE treats '_'/'%' as wildcards unless escaped — a prefix containing them (e.g. a
        // literal underscore in a cache key) must still be matched literally.
        await _store.PutAsync("meta_x:a", "1");
        await _store.PutAsync("metaYx:a", "2");

        var keys = await _store.ListKeysWithPrefixAsync("meta_x:");

        Assert.Single(keys);
        Assert.Contains("meta_x:a", keys);
    }
}
