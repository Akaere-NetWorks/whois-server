using System.Net.Http.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/utils/ip_info.rs — optional ASN/geolocation enrichment for
/// ping/traceroute hops, via ipinfo.io/lite. Requires an API token; ping/traceroute degrade
/// gracefully (no enrichment, not a hard failure) when none is configured, matching the Rust
/// version's <c>Result&lt;IpInfoClient&gt;</c> being treated as "may fail if token not set".</summary>
public sealed class IpInfoClient(HttpClient httpClient, string? apiToken)
{
    public bool IsConfigured => !string.IsNullOrEmpty(apiToken);

    public async Task<IpInfo?> GetIpInfoAsync(string ip, CancellationToken ct = default)
    {
        if (!IsConfigured)
        {
            return null;
        }

        try
        {
            using var response = await httpClient.GetAsync($"https://api.ipinfo.io/lite/{ip}?token={apiToken}", ct);
            response.EnsureSuccessStatusCode();
            return await response.Content.ReadFromJsonAsync<IpInfo>(cancellationToken: ct);
        }
        catch
        {
            return null;
        }
    }
}

public sealed record IpInfo(
    [property: JsonPropertyName("asn")] string Asn,
    [property: JsonPropertyName("as_name")] string AsName,
    [property: JsonPropertyName("as_domain")] string AsDomain,
    [property: JsonPropertyName("country_code")] string CountryCode,
    [property: JsonPropertyName("country")] string Country,
    [property: JsonPropertyName("continent_code")] string ContinentCode,
    [property: JsonPropertyName("continent")] string Continent);
