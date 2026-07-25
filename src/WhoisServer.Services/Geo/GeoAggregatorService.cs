using System.Net.Http.Json;
using System.Text;
using System.Text.Json;

namespace WhoisServer.Services.Geo;

/// <summary>
/// Ported from src/services/geo/services.rs + formatters.rs — the <c>-GEO</c> suffix. Queries 5
/// geolocation providers in parallel (RIPEstat/MaxMind, IPinfo Lite, IP-API, Bilibili, Meituan)
/// and reports whichever succeeded, "N/A" for whichever didn't.
///
/// The Rust formatter (945 lines) is not ported line-for-line — this is a new, simpler formatter
/// covering the same 5 providers' fields, consistent with the "not byte-for-byte" decision
/// (plan.md). IPinfo's token is read from configuration instead of the Rust version's hardcoded
/// literal (plan.md §4.5 — secrets externalization).
/// </summary>
public sealed class GeoAggregatorService(HttpClient httpClient, string ipinfoToken)
{
    public async Task<string> ProcessGeoAsync(string resource, CancellationToken ct = default)
    {
        var ripeTask = TryGetAsync(() => QueryRipeAsync(resource, ct));
        var ipinfoTask = TryGetAsync(() => QueryIpinfoAsync(resource, ct));
        var ipApiTask = TryGetAsync(() => QueryIpApiAsync(resource, ct));
        var bilibiliTask = TryGetAsync(() => QueryBilibiliAsync(resource, ct));
        var meituanTask = TryGetAsync(() => QueryMeituanAsync(resource, ct));

        await Task.WhenAll(ripeTask, ipinfoTask, ipApiTask, bilibiliTask, meituanTask);

        var sb = new StringBuilder();
        sb.Append($"% IP Geolocation Query (aggregated)\n% Query: {resource}\n\n");

        AppendRipeSection(sb, ripeTask.Result);
        AppendIpinfoSection(sb, ipinfoTask.Result);
        AppendIpApiSection(sb, ipApiTask.Result);
        AppendBilibiliSection(sb, bilibiliTask.Result);
        AppendMeituanSection(sb, meituanTask.Result);

        return sb.ToString();
    }

    public async Task<string> ProcessRirGeoAsync(string resource, CancellationToken ct = default)
    {
        try
        {
            var response = await httpClient.GetAsync(
                $"https://stat.ripe.net/data/rir-geo/data.json?resource={Uri.EscapeDataString(resource)}", ct);
            response.EnsureSuccessStatusCode();
            var rirGeo = await response.Content.ReadFromJsonAsync<RirGeoResponse>(cancellationToken: ct);

            if (rirGeo?.Status != "ok" || rirGeo.Data?.LocatedResources is not { Count: > 0 } resources)
            {
                return $"% RIR Geo Query\n% Query: {resource}\n\n% No RIR geolocation data found\n";
            }

            var sb = new StringBuilder();
            sb.Append($"% RIR Geo Query (RIPE NCC STAT)\n% Query: {resource}\n\n");
            foreach (var res in resources)
            {
                sb.Append($"resource:       {res.Resource}\n");
                sb.Append($"location:       {res.Location}\n\n");
            }

            return sb.ToString();
        }
        catch (Exception e)
        {
            return $"% RIR Geo Query Failed\n% Query: {resource}\n% Error: {e.Message}\n";
        }
    }

    public async Task<string> ProcessPrefixesAsync(string asn, CancellationToken ct = default)
    {
        try
        {
            var response = await httpClient.GetAsync(
                $"https://stat.ripe.net/data/announced-prefixes/data.json?resource={Uri.EscapeDataString(asn)}", ct);
            response.EnsureSuccessStatusCode();
            var prefixes = await response.Content.ReadFromJsonAsync<PrefixesResponse>(cancellationToken: ct);

            if (prefixes?.Status != "ok" || prefixes.Data?.Prefixes is not { Count: > 0 } list)
            {
                return $"% ASN Announced Prefixes Query\n% Data from RIPE NCC STAT\n% Query: {asn}\n\n% No announced prefixes found\n";
            }

            var sb = new StringBuilder();
            sb.Append($"% ASN Announced Prefixes Query\n% Data from RIPE NCC STAT\n% Query: {asn}\n\n");
            sb.Append($"% Total prefixes: {list.Count}\n\n");
            foreach (var prefix in list)
            {
                sb.Append($"route:          {prefix.Prefix}\n");
            }

            return sb.ToString();
        }
        catch (Exception e)
        {
            return $"% ASN Announced Prefixes Query\n% Data from RIPE NCC STAT\n% Query: {asn}\n\n% Error: {e.Message}\n";
        }
    }

    private static async Task<T?> TryGetAsync<T>(Func<Task<T>> fetch) where T : class
    {
        try
        {
            return await fetch();
        }
        catch
        {
            return null;
        }
    }

    private async Task<RipeStatResponse> QueryRipeAsync(string resource, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get,
            $"https://stat.ripe.net/data/maxmind-geo-lite/data.json?resource={Uri.EscapeDataString(resource)}");
        request.Headers.TryAddWithoutValidation("User-Agent", "akaere-whois-server/1.0");
        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var result = await response.Content.ReadFromJsonAsync<RipeStatResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty RIPE STAT response");
        if (result.Status != "ok") throw new InvalidOperationException($"RIPE STAT status: {result.Status}");
        return result;
    }

    private async Task<IpinfoResponse> QueryIpinfoAsync(string resource, CancellationToken ct)
    {
        var response = await httpClient.GetAsync($"https://api.ipinfo.io/lite/{resource}?token={ipinfoToken}", ct);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<IpinfoResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty IPinfo response");
    }

    private async Task<IpApiResponse> QueryIpApiAsync(string resource, CancellationToken ct)
    {
        var response = await httpClient.GetAsync(
            $"http://ip-api.com/json/{resource}?fields=status,message,country,countryCode,regionName,city,lat,lon,timezone,isp,org,as,query", ct);
        response.EnsureSuccessStatusCode();
        var body = await response.Content.ReadAsStringAsync(ct);
        var result = JsonSerializer.Deserialize<IpApiResponse>(body)
            ?? throw new InvalidOperationException("Empty IP-API response");
        if (result.Status != "success") throw new InvalidOperationException($"IP-API status: {result.Status}");
        return result;
    }

    private async Task<BilibiliIpData> QueryBilibiliAsync(string resource, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get,
            $"https://api.live.bilibili.com/client/v1/Ip/getInfoNew?ip={resource}");
        request.Headers.TryAddWithoutValidation("Referer", "https://www.bilibili.com/");
        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var result = await response.Content.ReadFromJsonAsync<BilibiliIpResponse>(cancellationToken: ct);
        if (result?.Code != 0 || result.Data is null)
        {
            throw new InvalidOperationException($"Bilibili API error: {result?.Code}");
        }

        return result.Data;
    }

    private async Task<MeituanIpData> QueryMeituanAsync(string resource, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get,
            $"https://apimobile.meituan.com/locate/v2/ip/loc?rgeo=true&ip={resource}");
        request.Headers.TryAddWithoutValidation("Referer", "https://www.meituan.com/");
        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var result = await response.Content.ReadFromJsonAsync<MeituanIpResponse>(cancellationToken: ct);
        return result?.Data ?? throw new InvalidOperationException("Meituan API returned no data");
    }

    private static void AppendRipeSection(StringBuilder sb, RipeStatResponse? ripe)
    {
        sb.Append("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
        var location = ripe?.Data?.LocatedResources?.FirstOrDefault()?.Locations?.FirstOrDefault();
        if (location is null)
        {
            sb.Append("N/A\n\n");
            return;
        }

        sb.Append($"country:        {location.Country ?? "N/A"}\n");
        sb.Append($"city:           {location.City ?? "N/A"}\n");
        if (location.Latitude is { } lat && location.Longitude is { } lon)
        {
            sb.Append($"coordinates:    {lat}, {lon}\n");
        }

        sb.Append('\n');
    }

    private static void AppendIpinfoSection(StringBuilder sb, IpinfoResponse? info)
    {
        sb.Append("=== IPinfo Lite ===\n");
        if (info is null)
        {
            sb.Append("N/A\n\n");
            return;
        }

        sb.Append($"asn:            {info.Asn ?? "N/A"}\n");
        sb.Append($"as-name:        {info.AsName ?? "N/A"}\n");
        sb.Append($"country:        {info.Country ?? "N/A"}\n");
        sb.Append($"continent:      {info.Continent ?? "N/A"}\n");
        sb.Append($"city:           {info.City ?? "N/A"}\n");
        sb.Append('\n');
    }

    private static void AppendIpApiSection(StringBuilder sb, IpApiResponse? info)
    {
        sb.Append("=== IP-API ===\n");
        if (info is null)
        {
            sb.Append("N/A\n\n");
            return;
        }

        sb.Append($"country:        {info.Country ?? "N/A"} ({info.CountryCode})\n");
        sb.Append($"region:         {info.RegionName ?? "N/A"}\n");
        sb.Append($"city:           {info.City ?? "N/A"}\n");
        sb.Append($"isp:            {info.Isp ?? "N/A"}\n");
        sb.Append($"org:            {info.Org ?? "N/A"}\n");
        sb.Append($"as:             {info.AsnInfo ?? "N/A"}\n");
        sb.Append('\n');
    }

    private static void AppendBilibiliSection(StringBuilder sb, BilibiliIpData? info)
    {
        sb.Append("=== Bilibili ===\n");
        if (info is null)
        {
            sb.Append("N/A\n\n");
            return;
        }

        sb.Append($"address:        {info.Addr}\n");
        sb.Append($"country:        {info.Country}\n");
        sb.Append($"province:       {info.Province}\n");
        sb.Append($"city:           {info.City}\n");
        sb.Append($"isp:            {info.Isp}\n");
        sb.Append('\n');
    }

    private static void AppendMeituanSection(StringBuilder sb, MeituanIpData? info)
    {
        sb.Append("=== Meituan ===\n");
        if (info is null)
        {
            sb.Append("N/A\n\n");
            return;
        }

        sb.Append($"country:        {info.Rgeo.Country}\n");
        sb.Append($"province:       {info.Rgeo.Province}\n");
        sb.Append($"city:           {info.Rgeo.City}\n");
        sb.Append($"district:       {info.Rgeo.District}\n");
        sb.Append($"coordinates:    {info.Lat}, {info.Lng}\n");
        sb.Append('\n');
    }
}
