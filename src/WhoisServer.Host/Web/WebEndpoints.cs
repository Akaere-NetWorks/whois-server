using System.Diagnostics;
using Microsoft.AspNetCore.Mvc;
using WhoisServer.Core.Query;
using WhoisServer.Core.Stats;

namespace WhoisServer.Host.Web;

/// <summary>
/// The Phase 4 REST/JSON API — ported from src/web/dashboard.rs's route table, minus the Pixiv
/// endpoints (this port never implemented Pixiv; see plan.md — nothing to proxy or format) and
/// minus a hand-maintained openapi.json (built-in `MapOpenApi()` generates one from the endpoint
/// metadata below instead, so the spec can't silently drift from the actual routes).
/// </summary>
public static class WebEndpoints
{
    public static void MapWhoisApi(this WebApplication app)
    {
        app.MapGet("/api/stats", (StatsManager stats) => Results.Json(BuildStatsResponse(stats)))
            .WithName("GetStats");

        app.MapGet("/api/whois", async (string q, QueryProcessor processor, CancellationToken ct) =>
                Results.Json(await RunQueryAsync(q, processor, ct)))
            .WithName("GetWhois");

        app.MapPost("/api/whois", async ([FromBody] WhoisQueryRequest body, QueryProcessor processor, CancellationToken ct) =>
                Results.Json(await RunQueryAsync(body.Q, processor, ct)))
            .WithName("PostWhois");

        app.MapGet("/raw/{query}", async (string query, QueryProcessor processor, CancellationToken ct) =>
        {
            var decoded = Uri.UnescapeDataString(query);
            var queryType = QueryAnalyzer.Analyze(decoded);
            var raw = await processor.ProcessAsync(decoded, queryType, colorScheme: null, ct);
            return Results.Text(raw, "text/plain");
        }).WithName("GetRaw");
    }

    private static async Task<WhoisApiResponse> RunQueryAsync(string query, QueryProcessor processor, CancellationToken ct)
    {
        var stopwatch = Stopwatch.StartNew();
        var queryType = QueryAnalyzer.Analyze(query);
        var typeName = queryType.GetType().Name;

        try
        {
            var raw = await processor.ProcessAsync(query, queryType, colorScheme: null, ct);
            return JsonFormatter.FormatResponse(query, raw, typeName, stopwatch.Elapsed.TotalMilliseconds);
        }
        catch (Exception e)
        {
            return JsonFormatter.FormatError(query, typeName, e.Message, stopwatch.Elapsed.TotalMilliseconds);
        }
    }

    private static StatsApiResponse BuildStatsResponse(StatsManager statsManager)
    {
        var snapshot = statsManager.GetSnapshot();
        var now = DateTimeOffset.UtcNow;

        var hourly = new List<StatsSeriesPoint>();
        for (var i = 0; i < 24; i++)
        {
            var hourTime = now.AddHours(-23 + i);
            var key = hourTime.ToString("yyyy-MM-dd HH");
            var stat = snapshot.Hourly.GetValueOrDefault(key, new DailyStats(0, 0));
            hourly.Add(new StatsSeriesPoint(hourTime.ToString("HH:00"), stat.Requests, stat.BytesServed));
        }

        var daily = new List<StatsSeriesPoint>();
        for (var i = 0; i < 30; i++)
        {
            var day = now.AddDays(-29 + i);
            var key = day.ToString("yyyy-MM-dd");
            var stat = snapshot.Daily.GetValueOrDefault(key, new DailyStats(0, 0));
            daily.Add(new StatsSeriesPoint(key, stat.Requests, stat.BytesServed));
        }

        return new StatsApiResponse(snapshot.TotalRequests, snapshot.TotalBytesServed, hourly, daily);
    }
}

public sealed record WhoisQueryRequest(string Q);

public sealed record StatsSeriesPoint(string Label, long Requests, long BytesServed);

public sealed record StatsApiResponse(
    long TotalRequests,
    long TotalBytesServed,
    List<StatsSeriesPoint> Hourly24h,
    List<StatsSeriesPoint> Daily30d);
