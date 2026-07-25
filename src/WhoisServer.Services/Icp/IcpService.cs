using System.Net.Http.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Icp;

/// <summary>
/// Ported from src/services/icp.rs — the <c>-ICP</c> suffix, looking up Chinese ICP filing
/// (beian) records for a domain via two third-party providers, tried in order (Baidu, then
/// DNSPod), first success wins. Whole-response cached for 1 day via <see cref="IKvStore"/>'s
/// native TTL instead of the Rust version's hand-rolled <c>cached_at</c> bookkeeping.
/// </summary>
public sealed partial class IcpService(HttpClient httpClient, IKvStore store)
{
    private static readonly TimeSpan CacheDuration = TimeSpan.FromDays(1);
    private const int MaxRetries = 3;

    // Both providers 403 a request with no User-Agent at all — same anti-bot behavior hit
    // elsewhere in this port (RIPE RDAP, PeeringDB, IANA's PEN registry); Rust's icp.rs set this
    // exact UA on its HTTP_CLIENT for the same reason.
    private const string BrowserUserAgent =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

    [GeneratedRegex(@"^[\w\-]+(\.[\w\-]+)+[\w\-]*$")]
    private static partial Regex DomainRegex();

    public async Task<string> ProcessAsync(string domain, CancellationToken ct = default)
    {
        string cleanDomain;
        try
        {
            cleanDomain = CleanDomain(domain);
        }
        catch (ArgumentException e)
        {
            return $"""
                % ICP Query Failed
                % Error: {e.Message}

                """;
        }

        var cacheKey = $"domain:{cleanDomain}";
        var cached = await store.GetAsync(cacheKey, ct);
        if (cached is not null)
        {
            return cached;
        }

        foreach (var query in new Func<string, CancellationToken, Task<string?>>[] { QueryBaiduAsync, QueryDnsPodAsync })
        {
            var result = await query(cleanDomain, ct);
            if (result is not null)
            {
                await store.PutAsync(cacheKey, result, CacheDuration, ct);
                return result;
            }
        }

        return $"""
            % ICP Query Failed for {cleanDomain}
            % Error: All providers returned no results

            """;
    }

    private static string CleanDomain(string domain)
    {
        var trimmed = domain.Trim().ToLowerInvariant();
        trimmed = trimmed.StartsWith("http://", StringComparison.Ordinal) ? trimmed[7..]
            : trimmed.StartsWith("https://", StringComparison.Ordinal) ? trimmed[8..]
            : trimmed.StartsWith("www.", StringComparison.Ordinal) ? trimmed[4..]
            : trimmed;
        trimmed = trimmed.Split(':')[0];

        if (!DomainRegex().IsMatch(trimmed))
        {
            throw new ArgumentException($"Invalid domain format: {trimmed}");
        }

        return trimmed;
    }

    private async Task<string?> QueryBaiduAsync(string domain, CancellationToken ct)
    {
        const string url = "https://cloud.baidu.com/api/sme/aladdin/icpquery";

        for (var attempt = 1; attempt <= MaxRetries; attempt++)
        {
            try
            {
                using var request = new HttpRequestMessage(HttpMethod.Post, url)
                {
                    Content = JsonContent.Create(new { host = domain, domain }),
                };
                request.Headers.TryAddWithoutValidation("Origin", "https://cloud.baidu.com");
                request.Headers.TryAddWithoutValidation("Referer", "https://cloud.baidu.com/product/bcd/toolPack.html?pageTitle=whois");
                request.Headers.TryAddWithoutValidation("User-Agent", BrowserUserAgent);

                using var response = await httpClient.SendAsync(request, ct);
                if (!response.IsSuccessStatusCode)
                {
                    continue;
                }

                var body = await response.Content.ReadFromJsonAsync<BaiduIcpResponse>(cancellationToken: ct);
                if (body is null || !body.Success)
                {
                    if (attempt < MaxRetries)
                    {
                        await Task.Delay(500, ct);
                    }
                    continue;
                }

                return FormatResult(
                    domain, "baidu", body.Result.Exists, body.Result.Number, body.Result.Company,
                    body.Result.SiteName, body.Result.AuditTime, body.Result.Type);
            }
            catch
            {
                if (attempt < MaxRetries)
                {
                    await Task.Delay(500, ct);
                }
            }
        }

        return null;
    }

    private async Task<string?> QueryDnsPodAsync(string domain, CancellationToken ct)
    {
        const string url = "https://www.dnspod.cn/cgi/dnsapi";

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Post, url)
            {
                Content = JsonContent.Create(new { ori_domain = domain, api = "Tools.Check.Website" }),
            };
            request.Headers.TryAddWithoutValidation("Origin", "https://tool.dnspod.cn");
            request.Headers.TryAddWithoutValidation("Referer", "https://tool.dnspod.cn");
            request.Headers.TryAddWithoutValidation("User-Agent", BrowserUserAgent);

            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                return null;
            }

            var body = await response.Content.ReadFromJsonAsync<DnsPodResponse>(cancellationToken: ct);
            if (body is null || body.Code != 0)
            {
                return null;
            }

            var exists = body.Data.IfBeian.Status == "1";
            return FormatResult(domain, "dnspod", exists, exists ? body.Data.IfBeian.Info : "", "", null, null, null);
        }
        catch
        {
            return null;
        }
    }

    private static string FormatResult(
        string domain, string provider, bool exists, string number, string company,
        string? siteName, string? auditTime, string? icpType)
    {
        var lines = new List<string>
        {
            $"% ICP Filing Information for {domain}",
            $"% Data source: {provider}",
            "",
        };

        if (exists)
        {
            if (!string.IsNullOrEmpty(number)) lines.Add($"ICP Number:      {number}");
            if (!string.IsNullOrEmpty(company)) lines.Add($"Company:          {company}");
            if (!string.IsNullOrEmpty(siteName)) lines.Add($"Site Name:        {siteName}");
            if (!string.IsNullOrEmpty(icpType)) lines.Add($"Type:             {icpType}");
            if (!string.IsNullOrEmpty(auditTime)) lines.Add($"Audit Time:       {auditTime}");
        }
        else
        {
            lines.Add("Status:           No ICP filing found");
        }

        lines.Add("");
        return string.Join('\n', lines);
    }

    private sealed record BaiduIcpResponse(
        [property: JsonPropertyName("result")] BaiduIcpResult Result,
        [property: JsonPropertyName("status")] int Status,
        [property: JsonPropertyName("success")] bool Success);

    private sealed record BaiduIcpResult(
        [property: JsonPropertyName("auditTime")] string AuditTime,
        [property: JsonPropertyName("company")] string Company,
        [property: JsonPropertyName("exists")] bool Exists,
        [property: JsonPropertyName("number")] string Number,
        [property: JsonPropertyName("siteName")] string SiteName,
        [property: JsonPropertyName("type")] string Type);

    private sealed record DnsPodResponse(
        [property: JsonPropertyName("code")] int Code,
        [property: JsonPropertyName("data")] DnsPodData Data);

    private sealed record DnsPodData([property: JsonPropertyName("if_beian")] DnsPodBeian IfBeian);

    private sealed record DnsPodBeian(
        [property: JsonPropertyName("status")] string Status,
        [property: JsonPropertyName("info")] string Info);
}
