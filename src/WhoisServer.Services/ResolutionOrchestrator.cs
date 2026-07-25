using Microsoft.Extensions.Logging;
using WhoisServer.Core.Dn42;
using WhoisServer.Core.Query;
using WhoisServer.Services.Rdap;
using WhoisServer.Services.Whois;

namespace WhoisServer.Services;

/// <summary>
/// Resolves a Domain/IPv4/IPv6/Asn/BareAsn/Unknown query to WHOIS-style text.
///
/// Default (matching the Rust version's actual default build — <c>rdap-backend</c> is NOT part of
/// its <c>default = ["full"]</c> feature set, confirmed by reading services/rdap.rs's own doc
/// comment: "This is experimental... When rdap-backend is NOT enabled, all queries use the default
/// WHOIS backend"): query the authoritative RIR WHOIS server directly via the IANA referral chain,
/// never touching RDAP at all. An earlier version of this port made RDAP-first-with-WHOIS-fallback
/// the unconditional default here — that was a real behavioral bug, not the deliberate
/// simplification it was originally believed to be, since it diverged from what a WHOIS client
/// actually gets by default against the real Rust server. Fixed by gating the RDAP-first path
/// behind <c>Feature_RdapBackend</c> (default <c>false</c>, mirroring Rust's own default), matching
/// Rust's own <c>#[cfg(feature = "rdap-backend")]</c>/<c>#[cfg(not(...))]</c> branch in
/// query_processor.rs exactly.
///
/// The explicit <c>-RDAP</c> suffix (<see cref="WhoisServer.Core.Query.QueryType.Rdap"/>, handled
/// separately in QueryProcessor) still works regardless of this flag — a deliberate improvement
/// over Rust, where <c>QueryType::Rdap</c> itself doesn't even exist without the feature compiled
/// in, so there's no way to request RDAP output at all without a recompile. Here, an explicit
/// per-query request is treated as intentional user intent independent of what the *default*,
/// unsuffixed resolution path does.
///
/// <paramref name="dn42Router"/> is null whenever the DN42 feature isn't compiled in (see
/// <see cref="IDn42QueryRouter"/>'s doc comment) — every DN42-specific branch below is a no-op in
/// that case. Where present, its per-QueryType routing mirrors core/query_processor.rs's DN42
/// arms: Domain/IPv4/IPv6/Asn "definitely DN42" cases return the DN42 result directly (no
/// RDAP/WHOIS attempted at all); BareAsn tries DN42 first and only falls back to RDAP/WHOIS if
/// DN42 didn't have it; Unknown checks the explicit "-DN42"/"-MNT" markers first, and — if the
/// normal RDAP/WHOIS path comes back empty — tries DN42 as a last resort. Unlike the Rust version,
/// DN42 responses still go through the same colorize+patch pipeline as everything else (see
/// QueryProcessor) rather than bypassing it — a deliberate simplification, not a Rust behavior
/// carried over.
/// </summary>
public sealed class ResolutionOrchestrator(
#if FEATURE_RDAP_BACKEND
    RdapClient rdapClient,
#endif
    WhoisResolutionService whoisResolution,
#if FEATURE_RDAP_BACKEND
    ILogger<ResolutionOrchestrator> logger,
#endif
    IDn42QueryRouter? dn42Router = null)
{
    public async Task<string> ResolveAsync(string query, QueryType queryType, CancellationToken ct = default)
    {
        if (dn42Router is not null)
        {
            if (queryType is QueryType.Unknown && dn42Router.IsExplicitDn42Marker(query))
            {
                return await dn42Router.ResolveAsync(query, ct);
            }

            if (dn42Router.IsDefinitivelyDn42(queryType))
            {
                return await dn42Router.ResolveAsync(query, ct);
            }

            if (queryType is QueryType.BareAsn)
            {
                var dn42Result = await dn42Router.ResolveAsync(query, ct);
                if (IsMeaningfulDn42Result(dn42Result))
                {
                    return dn42Result;
                }
            }
        }

        var result = await ResolveViaRdapOrWhoisAsync(query, queryType, ct);

        if (dn42Router is not null && queryType is QueryType.Unknown && !IsMeaningfulDn42Result(result))
        {
            return await dn42Router.ResolveAsync(query, ct);
        }

        return result;
    }

    private async Task<string> ResolveViaRdapOrWhoisAsync(string query, QueryType queryType, CancellationToken ct)
    {
#if FEATURE_RDAP_BACKEND
        if (RdapClient.BuildRequestPath(queryType) is not null)
        {
            try
            {
                var rdapResult = await rdapClient.QueryAsync(query, queryType, ct);
                logger.LogDebug("RDAP query succeeded for {Query}", query);
                return rdapResult;
            }
            catch (Exception e)
            {
                logger.LogDebug(e, "RDAP query failed for {Query}, falling back to WHOIS", query);
            }
        }
#endif

        return await whoisResolution.QueryWithIanaReferralAsync(query, ct);
    }

    /// <summary>Mirrors the Rust version's ad-hoc "does this look like a real hit" check
    /// (<c>!response.contains("404 Not Found") && !response.is_empty()</c>) used to decide
    /// whether a DN42 lookup (or lack thereof) should end the fallback chain.</summary>
    private static bool IsMeaningfulDn42Result(string response) =>
        !string.IsNullOrWhiteSpace(response)
        && !response.Contains("404 Not Found", StringComparison.Ordinal)
        && !response.Contains("No entries found", StringComparison.OrdinalIgnoreCase)
        && !response.Contains("Not found", StringComparison.OrdinalIgnoreCase);
}
