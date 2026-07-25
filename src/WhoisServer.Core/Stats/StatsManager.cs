using System.Text.Json;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Storage;

namespace WhoisServer.Core.Stats;

public sealed record DailyStats(long Requests, long BytesServed);

/// <summary>
/// Query-count/byte-count tracking, backed by <see cref="IKvStore"/>. Unlike the Rust version's
/// every-10th/every-100th-request modulo counters (an artifact of not having graceful shutdown —
/// see plan.md §4.4/§4.6/§7.8), persistence and cleanup here are driven by a periodic background
/// flush (see WhoisServer.Host's StatsFlushService) plus a flush on host shutdown. This class only
/// tracks in-memory state and knows how to load/save/prune itself; it does not schedule anything.
/// </summary>
public sealed class StatsManager(IKvStore store, ILogger<StatsManager> logger)
{
    private const string TotalKey = "stats:total";
    private const string DailyPrefix = "stats:daily:";
    private const string HourlyPrefix = "stats:hourly:";

    private readonly Lock _lock = new();
    private long _totalRequests;
    private long _totalBytesServed;
    private readonly Dictionary<string, DailyStats> _daily = [];
    private readonly Dictionary<string, DailyStats> _hourly = [];

    public async Task LoadAsync(CancellationToken ct = default)
    {
        var totalJson = await store.GetAsync(TotalKey, ct);
        long totalRequests = 0, totalBytes = 0;
        if (totalJson is not null)
        {
            var pair = JsonSerializer.Deserialize<long[]>(totalJson);
            if (pair is { Length: 2 })
            {
                totalRequests = pair[0];
                totalBytes = pair[1];
            }
        }

        var daily = new Dictionary<string, DailyStats>();
        foreach (var key in await store.ListKeysWithPrefixAsync(DailyPrefix, ct))
        {
            var date = key[DailyPrefix.Length..];
            var json = await store.GetAsync(key, ct);
            if (json is not null)
            {
                var stat = JsonSerializer.Deserialize<DailyStats>(json);
                if (stat is not null)
                {
                    daily[date] = stat;
                }
            }
        }

        var hourly = new Dictionary<string, DailyStats>();
        foreach (var key in await store.ListKeysWithPrefixAsync(HourlyPrefix, ct))
        {
            var datetime = key[HourlyPrefix.Length..];
            var json = await store.GetAsync(key, ct);
            if (json is not null)
            {
                var stat = JsonSerializer.Deserialize<DailyStats>(json);
                if (stat is not null)
                {
                    hourly[datetime] = stat;
                }
            }
        }

        lock (_lock)
        {
            _totalRequests = totalRequests;
            _totalBytesServed = totalBytes;
            _daily.Clear();
            foreach (var (k, v) in daily) _daily[k] = v;
            _hourly.Clear();
            foreach (var (k, v) in hourly) _hourly[k] = v;
        }

        logger.LogInformation(
            "Loaded statistics: {TotalRequests} total requests, {DailyCount} daily entries, {HourlyCount} hourly entries",
            totalRequests, daily.Count, hourly.Count);
    }

    public void RecordRequest(long responseBytes)
    {
        var now = DateTimeOffset.UtcNow;
        var today = now.ToString("yyyy-MM-dd");
        var currentHour = now.ToString("yyyy-MM-dd HH");

        lock (_lock)
        {
            _totalRequests++;
            _totalBytesServed += responseBytes;

            var day = _daily.GetValueOrDefault(today, new DailyStats(0, 0));
            _daily[today] = day with { Requests = day.Requests + 1, BytesServed = day.BytesServed + responseBytes };

            var hour = _hourly.GetValueOrDefault(currentHour, new DailyStats(0, 0));
            _hourly[currentHour] = hour with { Requests = hour.Requests + 1, BytesServed = hour.BytesServed + responseBytes };
        }
    }

    public void CleanupOld()
    {
        var now = DateTimeOffset.UtcNow;
        var oneMonthAgo = now.AddDays(-31).ToString("yyyy-MM-dd");
        var oneDayAgo = now.AddHours(-25).ToString("yyyy-MM-dd HH");

        lock (_lock)
        {
            foreach (var date in _daily.Keys.Where(d => string.CompareOrdinal(d, oneMonthAgo) < 0).ToList())
            {
                _daily.Remove(date);
            }

            foreach (var datetime in _hourly.Keys.Where(d => string.CompareOrdinal(d, oneDayAgo) < 0).ToList())
            {
                _hourly.Remove(datetime);
            }
        }
    }

    public async Task SaveAsync(CancellationToken ct = default)
    {
        long totalRequests, totalBytes;
        Dictionary<string, DailyStats> daily, hourly;
        lock (_lock)
        {
            totalRequests = _totalRequests;
            totalBytes = _totalBytesServed;
            daily = new Dictionary<string, DailyStats>(_daily);
            hourly = new Dictionary<string, DailyStats>(_hourly);
        }

        await store.PutAsync(TotalKey, JsonSerializer.Serialize(new[] { totalRequests, totalBytes }), ct: ct);

        foreach (var (date, stat) in daily)
        {
            await store.PutAsync($"{DailyPrefix}{date}", JsonSerializer.Serialize(stat), ct: ct);
        }

        foreach (var (datetime, stat) in hourly)
        {
            await store.PutAsync($"{HourlyPrefix}{datetime}", JsonSerializer.Serialize(stat), ct: ct);
        }
    }

    public (long TotalRequests, long TotalBytesServed) GetTotals()
    {
        lock (_lock)
        {
            return (_totalRequests, _totalBytesServed);
        }
    }

    /// <summary>Point-in-time copy of everything the web dashboard's stats API needs — keyed by
    /// the same "yyyy-MM-dd"/"yyyy-MM-dd HH" strings used internally, so the caller (not this
    /// class) decides how to turn that into a 24h/30d display series.</summary>
    public StatsSnapshot GetSnapshot()
    {
        lock (_lock)
        {
            return new StatsSnapshot(
                _totalRequests,
                _totalBytesServed,
                new Dictionary<string, DailyStats>(_daily),
                new Dictionary<string, DailyStats>(_hourly));
        }
    }
}

public sealed record StatsSnapshot(
    long TotalRequests,
    long TotalBytesServed,
    IReadOnlyDictionary<string, DailyStats> Daily,
    IReadOnlyDictionary<string, DailyStats> Hourly);
