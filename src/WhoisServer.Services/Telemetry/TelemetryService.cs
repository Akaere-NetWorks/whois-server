using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace WhoisServer.Services.Telemetry;

/// <summary>
/// Ported from src/core/telemetry.rs's <c>telemetry</c> feature. Deliberately NOT a genuine
/// OpenTelemetry SDK integration — that was this plan's original (aspirational, written before
/// actually reading the Rust source) idea of what "telemetry" meant here. What the Rust feature
/// actually does is much narrower: fire-and-forget POST one small JSON object per query to an
/// operator-configured webhook with bearer auth. Pulling in the full OTel SDK/exporter machinery
/// for that would be solving a problem this project doesn't have, so this is a faithful port of
/// the actual mechanism instead.
///
/// Config is 3 env vars, read once at construction (matching the Rust version's
/// <c>OnceLock&lt;TelemetryConfig&gt;</c>): <c>TELEMETRY_ENABLED</c> ("true"/"1"),
/// <c>TELEMETRY_ENDPOINT</c>, <c>TELEMETRY_BEARER_TOKEN</c>. All three must be present for
/// anything to actually send (<see cref="IsValid"/>).
/// </summary>
public sealed class TelemetryService
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<TelemetryService> _logger;
    private readonly bool _enabled;
    private readonly string? _endpoint;
    private readonly string? _bearerToken;

    public TelemetryService(HttpClient httpClient, ILogger<TelemetryService> logger)
    {
        _httpClient = httpClient;
        _logger = logger;

        var enabledRaw = Environment.GetEnvironmentVariable("TELEMETRY_ENABLED");
        _enabled = enabledRaw is "true" or "1";
        _endpoint = Environment.GetEnvironmentVariable("TELEMETRY_ENDPOINT");
        _bearerToken = Environment.GetEnvironmentVariable("TELEMETRY_BEARER_TOKEN");
    }

    private bool IsValid => _enabled && !string.IsNullOrEmpty(_endpoint) && !string.IsNullOrEmpty(_bearerToken);

    /// <summary>Fire-and-forget — returns immediately regardless of whether the actual send
    /// succeeds, times out, or is skipped entirely because telemetry isn't configured. Mirrors the
    /// Rust version's <c>send_telemetry</c>: the caller's <c>await</c> there only resolves the
    /// <c>tokio::spawn</c> calls themselves, never the underlying HTTP request, so a slow or
    /// unreachable telemetry endpoint can never add latency to (let alone fail) a real WHOIS
    /// query. No retry on failure, same as upstream.</summary>
    public void RecordQuery(string queryObject, string queryType, string clientIp, long responseTimeMs)
    {
        if (!IsValid)
        {
            return;
        }

        _ = SendAsync(new TelemetryData(queryObject, queryType, clientIp, responseTimeMs));
    }

    private async Task SendAsync(TelemetryData data)
    {
        try
        {
            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(10));
            using var request = new HttpRequestMessage(HttpMethod.Post, _endpoint)
            {
                Content = JsonContent.Create(data),
            };
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", _bearerToken);
            request.Headers.UserAgent.ParseAdd("Akaere-Networks-Whois");

            using var response = await _httpClient.SendAsync(request, cts.Token);
            if (!response.IsSuccessStatusCode)
            {
                _logger.LogWarning("Telemetry endpoint returned {Status}", response.StatusCode);
            }
        }
        catch (Exception e)
        {
            _logger.LogWarning(e, "Failed to send telemetry");
        }
    }

    private sealed record TelemetryData(
        [property: JsonPropertyName("query_object")] string QueryObject,
        [property: JsonPropertyName("query_type")] string QueryType,
        [property: JsonPropertyName("client_ip")] string ClientIp,
        [property: JsonPropertyName("response_time")] long ResponseTime);
}
