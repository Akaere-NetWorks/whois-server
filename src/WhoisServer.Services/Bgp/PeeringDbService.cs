using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Bgp;

/// <summary>Ported from src/services/peeringdb.rs — the <c>-PEERINGDB</c> suffix. Queries
/// peeringdb.com for either ASN ("AS...") or Internet Exchange (bare numeric ID) info, cached
/// per-key for 1 day via <see cref="IKvStore"/>.</summary>
public sealed class PeeringDbService(HttpClient httpClient, IKvStore store)
{
    private static readonly TimeSpan CacheTtl = TimeSpan.FromDays(1);

    // PeeringDB's API 403s any request that doesn't look like a real browser — ported verbatim
    // from the Rust version's hardcoded User-Agent (services/peeringdb.rs's HTTP_CLIENT), found
    // to be load-bearing when this port initially 403'd without it.
    private const string BrowserUserAgent =
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

    private static HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", BrowserUserAgent);
        return request;
    }

    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        if (query.ToUpperInvariant().StartsWith("AS", StringComparison.Ordinal))
        {
            return await QueryAsnAsync(query, ct);
        }

        if (uint.TryParse(query, out var ixId))
        {
            return await QueryIxAsync(ixId, ct);
        }

        return "% Invalid PeeringDB query format. Use 'AS12345' for ASN or '1234' for IX ID\n";
    }

    private async Task<string> QueryAsnAsync(string query, CancellationToken ct)
    {
        var asnNumberStr = query.ToUpperInvariant().StartsWith("AS", StringComparison.Ordinal) ? query[2..] : query;
        if (!uint.TryParse(asnNumberStr, out var asnNumber))
        {
            return $"% Invalid ASN format: {query}\n";
        }

        var cacheKey = $"asn:{asnNumber}";
        var cached = await store.GetAsync(cacheKey, ct);
        if (cached is not null)
        {
            return cached;
        }

        try
        {
            using var request = CreateRequest($"https://www.peeringdb.com/api/net?asn={asnNumber}&depth=2");
            using var response = await httpClient.SendAsync(request, ct);
            response.EnsureSuccessStatusCode();

            var pdb = await response.Content.ReadFromJsonAsync<PeeringDbResponse<NetworkInfo>>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty PeeringDB response");

            if (pdb.Data.Count == 0)
            {
                var noData = $"% No network information found for ASN {asnNumber} in PeeringDB\n";
                await store.PutAsync(cacheKey, noData, TimeSpan.FromHours(1), ct);
                return noData;
            }

            var sb = new StringBuilder();
            sb.Append($"% PeeringDB Network Information for AS{asnNumber}\n");
            sb.Append("% Source: https://www.peeringdb.com/\n\n");
            foreach (var network in pdb.Data)
            {
                sb.Append(FormatNetwork(network));
                sb.Append('\n');
            }

            var result = sb.ToString();
            await store.PutAsync(cacheKey, result, CacheTtl, ct);
            return result;
        }
        catch (Exception e)
        {
            return $"% PeeringDB Query Failed\n% ASN: {asnNumber}\n% Error: {e.Message}\n";
        }
    }

    private async Task<string> QueryIxAsync(uint ixId, CancellationToken ct)
    {
        var cacheKey = $"ix:{ixId}";
        var cached = await store.GetAsync(cacheKey, ct);
        if (cached is not null)
        {
            return cached;
        }

        try
        {
            using var request = CreateRequest($"https://www.peeringdb.com/api/ix?id={ixId}&depth=2");
            using var response = await httpClient.SendAsync(request, ct);
            response.EnsureSuccessStatusCode();

            var pdb = await response.Content.ReadFromJsonAsync<PeeringDbResponse<ExchangeInfo>>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty PeeringDB response");

            if (pdb.Data.Count == 0)
            {
                var noData = $"% No Internet Exchange information found for ID {ixId} in PeeringDB\n";
                await store.PutAsync(cacheKey, noData, TimeSpan.FromHours(1), ct);
                return noData;
            }

            var sb = new StringBuilder();
            sb.Append($"% PeeringDB Internet Exchange Information for ID {ixId}\n");
            sb.Append("% Source: https://www.peeringdb.com/\n\n");
            foreach (var ix in pdb.Data)
            {
                sb.Append(FormatExchange(ix));
                sb.Append('\n');
            }

            var result = sb.ToString();
            await store.PutAsync(cacheKey, result, CacheTtl, ct);
            return result;
        }
        catch (Exception e)
        {
            return $"% PeeringDB Query Failed\n% IX ID: {ixId}\n% Error: {e.Message}\n";
        }
    }

    private static string FormatNetwork(NetworkInfo network)
    {
        var sb = new StringBuilder();
        sb.Append($"ASN:                AS{network.Asn}\n");
        sb.Append($"Network Name:       {network.Name}\n");
        if (network.NameLong is { } nameLong) sb.Append($"Full Name:          {nameLong}\n");
        sb.Append($"PeeringDB ID:       {network.Id}\n");
        sb.Append($"Organization ID:    {network.OrgId}\n");
        if (network.Website is { } website) sb.Append($"Website:            {website}\n");
        if (network.InfoType is { } infoType) sb.Append($"Network Type:       {infoType}\n");
        if (network.InfoTraffic is { } traffic) sb.Append($"Traffic Volume:     {traffic}\n");
        if (network.InfoPrefixes4 is { } p4) sb.Append($"IPv4 Prefixes:      {p4}\n");
        if (network.InfoPrefixes6 is { } p6) sb.Append($"IPv6 Prefixes:      {p6}\n");
        if (network.PolicyGeneral is { } policy) sb.Append($"Peering Policy:     {policy}\n");
        if (network.IxCount is { } ixCount) sb.Append($"Exchange Presence:  {ixCount} exchanges\n");
        if (network.FacCount is { } facCount) sb.Append($"Facility Presence:  {facCount} facilities\n");
        sb.Append($"Status:             {network.Status}\n");
        sb.Append($"Created:            {network.Created}\n");
        sb.Append($"Last Updated:       {network.Updated}\n");
        return sb.ToString();
    }

    private static string FormatExchange(ExchangeInfo ix)
    {
        var sb = new StringBuilder();
        sb.Append($"Exchange Name:      {ix.Name}\n");
        sb.Append($"PeeringDB ID:       {ix.Id}\n");
        sb.Append($"Organization ID:    {ix.OrgId}\n");
        sb.Append($"City:               {ix.City}\n");
        sb.Append($"Country:            {ix.Country}\n");
        sb.Append($"Continent:          {ix.RegionContinent}\n");
        if (ix.Website is { } website) sb.Append($"Website:            {website}\n");
        if (ix.NetCount is { } netCount) sb.Append($"Connected Networks: {netCount} networks\n");
        sb.Append($"Status:             {ix.Status}\n");
        sb.Append($"Created:            {ix.Created}\n");
        sb.Append($"Last Updated:       {ix.Updated}\n");
        return sb.ToString();
    }

    private sealed record PeeringDbResponse<T>([property: JsonPropertyName("data")] List<T> Data);

    private sealed record NetworkInfo(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("org_id")] uint OrgId,
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("name_long")] string? NameLong,
        [property: JsonPropertyName("asn")] uint Asn,
        [property: JsonPropertyName("website")] string? Website,
        [property: JsonPropertyName("info_type")] string? InfoType,
        [property: JsonPropertyName("info_traffic")] string? InfoTraffic,
        [property: JsonPropertyName("info_prefixes4")] uint? InfoPrefixes4,
        [property: JsonPropertyName("info_prefixes6")] uint? InfoPrefixes6,
        [property: JsonPropertyName("policy_general")] string? PolicyGeneral,
        [property: JsonPropertyName("ix_count")] uint? IxCount,
        [property: JsonPropertyName("fac_count")] uint? FacCount,
        [property: JsonPropertyName("status")] string Status,
        [property: JsonPropertyName("created")] string Created,
        [property: JsonPropertyName("updated")] string Updated);

    private sealed record ExchangeInfo(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("org_id")] uint OrgId,
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("city")] string City,
        [property: JsonPropertyName("country")] string Country,
        [property: JsonPropertyName("region_continent")] string RegionContinent,
        [property: JsonPropertyName("website")] string? Website,
        [property: JsonPropertyName("net_count")] uint? NetCount,
        [property: JsonPropertyName("status")] string Status,
        [property: JsonPropertyName("created")] string Created,
        [property: JsonPropertyName("updated")] string Updated);
}
