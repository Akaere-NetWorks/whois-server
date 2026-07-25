using Microsoft.Extensions.Logging;

namespace WhoisServer.Services.Whois;

/// <summary>
/// Ported from src/services/whois.rs's <c>query_with_iana_referral</c> — resolves the
/// authoritative server via <see cref="IanaReferralCache"/>, queries it, and falls back to RADB
/// when the response looks like a "this resource was transferred elsewhere" notice.
/// </summary>
public sealed class WhoisResolutionService(
    WhoisClient whoisClient, IanaReferralCache ianaCache, ILogger<WhoisResolutionService> logger)
{
    private static readonly string[] TransferIndicators =
    [
        "not managed by the ripe ncc", "not managed by ripe ncc", "managed by arin", "managed by apnic",
        "managed by lacnic", "managed by afrinic", "transferred", "no entries found", "not found",
        "no match found", "no data found", "% no entries found", "% not found",
        "asn block not managed", "ip block not managed", "for registration information",
        "you can find the whois server to query",
    ];

    private static string PrepareRipeQuery(string query, string server) =>
        server == WhoisServerConstants.RipeWhoisServer ? $"{query} --no-referenced" : query;

    public async Task<string> QueryWithIanaReferralAsync(string query, CancellationToken ct = default)
    {
        var whoisServer = await ianaCache.GetWhoisServerAsync(query, ct) ?? WhoisServerConstants.DefaultWhoisServer;

        try
        {
            var response = await whoisClient.QueryAsync(
                PrepareRipeQuery(query, whoisServer), whoisServer, WhoisServerConstants.DefaultWhoisPort,
                WhoisServerConstants.DefaultTimeout, ct);

            if (!ShouldTryRadbFallback(response, query))
            {
                return response;
            }

            logger.LogDebug("Primary response suggests transferred resource, trying RADB fallback for {Query}", query);
            try
            {
                var radbResponse = await whoisClient.QueryAsync(
                    query, WhoisServerConstants.RadbWhoisServer, WhoisServerConstants.RadbWhoisPort,
                    WhoisServerConstants.DefaultTimeout, ct);

                return IsMeaningfulResponse(radbResponse, query)
                    ? radbResponse
                    : $"{response.Trim()}\n\n% Additional query attempted via RADB:\n{radbResponse.Trim()}\n% End of RADB response\n";
            }
            catch (Exception e)
            {
                return $"{response.Trim()}\n\n% Note: RADB fallback attempted but failed: {e.Message}\n";
            }
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "Query failed on {Server}, trying RADB as fallback for {Query}", whoisServer, query);
            try
            {
                return await whoisClient.QueryAsync(
                    query, WhoisServerConstants.RadbWhoisServer, WhoisServerConstants.RadbWhoisPort,
                    WhoisServerConstants.DefaultTimeout, ct);
            }
            catch
            {
                var prepared = PrepareRipeQuery(query, WhoisServerConstants.DefaultWhoisServer);
                return await whoisClient.QueryAsync(
                    prepared, WhoisServerConstants.DefaultWhoisServer, WhoisServerConstants.DefaultWhoisPort,
                    WhoisServerConstants.DefaultTimeout, ct);
            }
        }
    }

    private static bool ShouldTryRadbFallback(string response, string query)
    {
        var responseLower = response.ToLowerInvariant();

        var meaningfulLines = response
            .Split('\n')
            .Where(line => !string.IsNullOrWhiteSpace(line) && !line.TrimStart().StartsWith('%'))
            .ToList();

        if (meaningfulLines.Count < 3)
        {
            return true;
        }

        var hasRoutingInfo = responseLower.Contains("route:") || responseLower.Contains("descr:")
            || responseLower.Contains("origin:") || responseLower.Contains("as-path:")
            || responseLower.Contains("source:") || responseLower.Contains("remarks:");

        if ((query.Contains('/') || query.Contains('-')) && !hasRoutingInfo)
        {
            return true;
        }

        return TransferIndicators.Any(responseLower.Contains);
    }

    private static bool IsMeaningfulResponse(string response, string query)
    {
        var meaningfulLines = response
            .Split('\n')
            .Select(l => l.Trim())
            .Where(l => l.Length > 0 && !l.StartsWith('%') && !l.StartsWith('#')
                && !l.Contains("Please report any issues") && !l.Contains("The objects are in RPSL format"))
            .ToList();

        return meaningfulLines.Count >= 5 && response.Length > 200 && !ShouldTryRadbFallback(response, query);
    }
}
