using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Whois;

/// <summary>
/// Resolves which authoritative WHOIS server to query for a given domain/IP/ASN, by asking
/// whois.iana.org for a referral and caching the result.
///
/// This is a deliberately simplified port of src/services/iana_cache.rs (724 lines): the Rust
/// version parses <c>as-block:</c>/<c>inetnum:</c>/<c>inet6num:</c> ranges out of the IANA
/// response so one lookup can answer future queries for *any* resource in the same block: this
/// port instead caches per-exact-query only. Functionally correct (every query still gets a
/// referral, just via its own IANA round trip on first sight per query rather than per block),
/// but less efficient across many queries in the same block — a known, deliberate scope
/// reduction, not an oversight; see plan.md's Phase 1 TODO for revisiting block-level caching.
///
/// One targeted exception (Phase 7): for domain queries specifically, if the exact domain has no
/// referral (e.g. IANA's own reserved test domains), <see cref="GetWhoisServerAsync"/> retries at
/// the TLD level, since IANA's referral data is genuinely organized per-TLD — this both fixes a
/// real bug (a cold cache misrouting such domains to RIPE/RADB and returning unrelated network
/// data) and incidentally caches the TLD for later domains under it.
/// </summary>
public sealed class IanaReferralCache(WhoisClient whoisClient, IKvStore store, ILogger<IanaReferralCache> logger)
{
    private static readonly TimeSpan CacheTtl = TimeSpan.FromDays(7);
    private static readonly Regex ReferLineRegex = new(@"^refer:\s*(\S+)", RegexOptions.Multiline | RegexOptions.IgnoreCase);
    private static readonly Regex WhoisLineRegex = new(@"^whois:\s*(\S+)", RegexOptions.Multiline | RegexOptions.IgnoreCase);

    public async Task<string?> GetWhoisServerAsync(string query, CancellationToken ct = default)
    {
        var cacheKey = $"referral:{query.ToUpperInvariant()}";
        var cached = await store.GetAsync(cacheKey, ct);
        if (cached is not null)
        {
            return cached;
        }

        var server = await LookupServerAsync(query, ct);

        // IANA's referral data is organized per-TLD, not per-domain: a domain query with no
        // direct referral (e.g. IANA's own reserved test domains like example.com) still belongs
        // to a real TLD zone that DOES have one. Retry at the TLD level so a domain that IANA
        // itself won't answer for still resolves via its zone's registry, rather than silently
        // falling through to the RIPE/RADB fallback path meant for network resources. This is a
        // targeted fix for the exact gap the side-by-side diff against whois.akae.re surfaced
        // (see plan.md Phase 7) — it does not attempt Rust's full inetnum/as-block range caching,
        // which remains the deliberately deferred larger item.
        if (server is null && query.Contains('.'))
        {
            var tld = query[(query.LastIndexOf('.') + 1)..];
            if (!string.Equals(tld, query, StringComparison.OrdinalIgnoreCase))
            {
                server = await LookupServerAsync(tld, ct);
            }
        }

        if (server is null)
        {
            return null;
        }

        await store.PutAsync(cacheKey, server, CacheTtl, ct);
        return server;
    }

    private async Task<string?> LookupServerAsync(string query, CancellationToken ct)
    {
        string ianaResponse;
        try
        {
            ianaResponse = await whoisClient.QueryAsync(
                query, WhoisServerConstants.IanaWhoisServer, WhoisServerConstants.IanaWhoisPort,
                WhoisServerConstants.DefaultTimeout, ct);
        }
        catch (Exception e)
        {
            logger.LogDebug(e, "IANA referral lookup failed for {Query}", query);
            return null;
        }

        var referMatch = ReferLineRegex.Match(ianaResponse);
        if (referMatch.Success)
        {
            return referMatch.Groups[1].Value;
        }

        var whoisMatch = WhoisLineRegex.Match(ianaResponse);
        return whoisMatch.Success ? whoisMatch.Groups[1].Value : null;
    }
}
