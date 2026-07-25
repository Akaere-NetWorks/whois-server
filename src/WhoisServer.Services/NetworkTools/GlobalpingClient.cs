using System.Net.Http.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/utils/globalping.rs — client for the public Globalping API
/// (globalping.io), used for both ping and traceroute measurements from real-world vantage points.</summary>
public sealed class GlobalpingClient(HttpClient httpClient)
{
    private const string ApiBase = "https://api.globalping.io/v1/measurements";
    private const int MaxPollAttempts = 60;
    private static readonly TimeSpan PollInterval = TimeSpan.FromSeconds(1);

    public async Task<string> SubmitMeasurementAsync(GlobalpingRequest request, CancellationToken ct = default)
    {
        using var response = await httpClient.PostAsJsonAsync(ApiBase, request, ct);
        if (!response.IsSuccessStatusCode)
        {
            var body = await response.Content.ReadAsStringAsync(ct);
            throw new InvalidOperationException($"Globalping API returned error: {response.StatusCode} - {body}");
        }

        var result = await response.Content.ReadFromJsonAsync<GlobalpingSubmitResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty Globalping submit response");
        return result.Id;
    }

    public async Task<GlobalpingResult> GetResultsAsync(string id, CancellationToken ct = default)
    {
        using var response = await httpClient.GetAsync($"{ApiBase}/{id}", ct);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<GlobalpingResult>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty Globalping result response");
    }

    public async Task<GlobalpingResult> WaitForResultsAsync(string id, int timeoutSeconds, CancellationToken ct = default)
    {
        var maxAttempts = Math.Min(timeoutSeconds, MaxPollAttempts);
        for (var attempt = 0; attempt < maxAttempts; attempt++)
        {
            var result = await GetResultsAsync(id, ct);
            if (result.Status == "finished")
            {
                return result;
            }

            if (result.Status == "failed")
            {
                throw new InvalidOperationException($"Measurement {id} failed");
            }

            await Task.Delay(PollInterval, ct);
        }

        throw new TimeoutException($"Measurement timed out after {timeoutSeconds} seconds");
    }
}

public sealed record GlobalpingRequest(
    [property: JsonPropertyName("type")] string MeasurementType,
    [property: JsonPropertyName("target")] string Target,
    [property: JsonPropertyName("limit")] int? Limit,
    [property: JsonPropertyName("measurementOptions")] object? MeasurementOptions,
    [property: JsonPropertyName("locations")] List<MeasurementLocation>? Locations,
    [property: JsonPropertyName("inProgressUpdates")] bool? InProgressUpdates);

public sealed record MeasurementLocation([property: JsonPropertyName("magic")] string Magic);

public sealed record PingOptions(
    [property: JsonPropertyName("packets")] int Packets,
    [property: JsonPropertyName("protocol")] string Protocol);

public sealed record TracerouteOptions([property: JsonPropertyName("protocol")] string Protocol);

public sealed record GlobalpingSubmitResponse([property: JsonPropertyName("id")] string Id);

public sealed record GlobalpingResult(
    [property: JsonPropertyName("results")] List<ProbeResult>? ResultsOrNull,
    [property: JsonPropertyName("status")] string Status)
{
    [JsonIgnore]
    public List<ProbeResult> Results => ResultsOrNull ?? [];
}

public sealed record ProbeResult(
    [property: JsonPropertyName("probe")] ProbeInfo Probe,
    [property: JsonPropertyName("result")] TestResult Result);

public sealed record ProbeInfo(
    [property: JsonPropertyName("country")] string Country,
    [property: JsonPropertyName("state")] string? State,
    [property: JsonPropertyName("city")] string? City,
    [property: JsonPropertyName("network")] string Network);

public sealed record TestResult(
    [property: JsonPropertyName("resolvedAddress")] string? ResolvedAddress,
    [property: JsonPropertyName("timings")] List<Timing>? Timings,
    [property: JsonPropertyName("stats")] Stats? Stats,
    [property: JsonPropertyName("hops")] List<HopResult>? Hops);

public sealed record Timing([property: JsonPropertyName("rtt")] double Rtt);

public sealed record Stats(
    [property: JsonPropertyName("min")] double Min,
    [property: JsonPropertyName("max")] double Max,
    [property: JsonPropertyName("avg")] double Avg,
    [property: JsonPropertyName("total")] int Total,
    [property: JsonPropertyName("loss")] int Loss,
    [property: JsonPropertyName("rcv")] int Received);

public sealed record HopResult(
    [property: JsonPropertyName("resolvedAddress")] string? ResolvedAddress,
    [property: JsonPropertyName("timings")] List<Timing>? Timings);
