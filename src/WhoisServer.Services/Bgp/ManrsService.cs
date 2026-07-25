using System.Net.Http.Json;
using System.Text.Json.Serialization;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Bgp;

/// <summary>
/// Ported from src/services/manrs.rs — the <c>-MANRS</c> suffix. Downloads the full MANRS member
/// ASN list and caches it (whole-list-refresh pattern, distinct from the per-query caches used
/// elsewhere) for 14 days via <see cref="IKvStore"/>, then does an O(1) membership check per query.
/// </summary>
public sealed class ManrsService(HttpClient httpClient, IKvStore store)
{
    private const string ApiUrl = "https://api.manrs.org/asns";
    private const string CacheKey = "manrs_asns";
    private const string CacheTimestampKey = "manrs_last_updated";
    private static readonly TimeSpan CacheDuration = TimeSpan.FromDays(14);

    /// <summary><paramref name="baseQuery"/> is already suffix-stripped by
    /// <see cref="WhoisServer.Core.Query.QueryAnalyzer"/> (unlike the Rust version, which strips
    /// "-MANRS" internally because it receives the whole original query).</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        var asn = ParseAsn(baseQuery);
        if (asn is null)
        {
            return $"""
                % MANRS Query Error: Invalid ASN format in query '{baseQuery}-MANRS'
                % Expected format: AS<number>-MANRS or <number>-MANRS
                % Example: AS64496-MANRS or 64496-MANRS

                """;
        }

        HashSet<ulong>? asns;
        try
        {
            asns = await GetAsnsAsync(ct);
        }
        catch
        {
            asns = null;
        }

        if (asns is null)
        {
            return """
                % MANRS Information: Unable to determine membership status
                %
                % This could be due to network connectivity issues or API unavailability.
                % Please try again later or check https://www.manrs.org/ directly.

                """;
        }

        var isMember = asns.Contains(asn.Value);
        var lastUpdatedRaw = await store.GetAsync(CacheTimestampKey, ct);
        var lastUpdated = long.TryParse(lastUpdatedRaw, out var ts)
            ? DateTimeOffset.FromUnixTimeSeconds(ts).UtcDateTime
            : (DateTime?)null;

        var status = isMember ? "MEMBER" : "NON-MEMBER";
        var updatedDisplay = lastUpdated?.ToString("yyyy-MM-dd HH:mm:ss") + " UTC" ?? "Unknown";

        return $"""
            % MANRS (Mutually Agreed Norms for Routing Security) Information
            %
            aut-num:            AS{asn}
            status:             {status}
            asn:                AS{asn}
            total-members:      {asns.Count}
            updated-time:       {updatedDisplay}
            %
            % MANRS is a global initiative that provides crucial fixes to reduce
            % the most common routing threats. The four actions of MANRS are:
            %   1. Filtering - Implement routing filters to prevent incorrect routing information
            %   2. Anti-spoofing - Enable anti-spoofing protection to prevent address spoofing
            %   3. Coordination - Facilitate coordination between network operators
            %   4. Global Validation - Facilitate global routing information validation
            %
            % For more information about MANRS, visit: https://www.manrs.org/
            %
            % Cache refresh interval: 14 days
            %

            """;
    }

    private async Task<HashSet<ulong>> GetAsnsAsync(CancellationToken ct)
    {
        var timestampRaw = await store.GetAsync(CacheTimestampKey, ct);
        var isExpired = !long.TryParse(timestampRaw, out var lastUpdated)
            || DateTimeOffset.UtcNow.ToUnixTimeSeconds() - lastUpdated > CacheDuration.TotalSeconds;

        if (!isExpired)
        {
            var cached = await GetCachedAsnsAsync(ct);
            if (cached is not null)
            {
                return cached;
            }
        }

        try
        {
            return await RefreshCacheAsync(ct);
        }
        catch
        {
            var cached = await GetCachedAsnsAsync(ct);
            if (cached is not null)
            {
                return cached;
            }

            throw;
        }
    }

    private async Task<HashSet<ulong>?> GetCachedAsnsAsync(CancellationToken ct)
    {
        var json = await store.GetAsync(CacheKey, ct);
        if (json is null)
        {
            return null;
        }

        var asns = System.Text.Json.JsonSerializer.Deserialize<List<ulong>>(json);
        return asns is null ? null : [.. asns];
    }

    private async Task<HashSet<ulong>> RefreshCacheAsync(CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, ApiUrl);
        request.Headers.TryAddWithoutValidation("User-Agent", "whois-server/0.1.0");

        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();

        var apiResponse = await response.Content.ReadFromJsonAsync<ManrsApiResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty MANRS API response");

        await store.PutAsync(CacheKey, System.Text.Json.JsonSerializer.Serialize(apiResponse.Asns), ct: ct);
        await store.PutAsync(CacheTimestampKey, DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString(), ct: ct);

        return [.. apiResponse.Asns];
    }

    private static ulong? ParseAsn(string baseQuery)
    {
        var asnStr = baseQuery.ToUpperInvariant().StartsWith("AS", StringComparison.Ordinal)
            ? baseQuery[2..]
            : baseQuery;

        return ulong.TryParse(asnStr, out var value) ? value : null;
    }

    private sealed record ManrsApiResponse([property: JsonPropertyName("asns")] List<ulong> Asns);
}
