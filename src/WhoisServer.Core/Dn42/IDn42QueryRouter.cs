using WhoisServer.Core.Query;

namespace WhoisServer.Core.Dn42;

/// <summary>
/// Lives in Core (not the feature-gated WhoisServer.Services.Dn42 project) so
/// <c>ResolutionOrchestrator</c> can depend on it unconditionally: when the DN42 feature is
/// compiled out, nothing implements this interface, DI resolves the optional constructor
/// parameter as null, and every check below is skipped — no <c>#if</c> needed in
/// WhoisServer.Services at all. See plan.md's Phase 3 notes for the full routing rationale.
/// </summary>
public interface IDn42QueryRouter
{
    /// <summary>True for cases where DN42 is the *only* correct backend — private/reserved IPs,
    /// .dn42 domains, and ASNs in the AS4242420000-AS4242429999 block — mirroring
    /// core/query_processor.rs's early-return arms for Domain/IPv4/IPv6/ASN.</summary>
    bool IsDefinitivelyDn42(QueryType queryType);

    /// <summary>True for the "-DN42"/"-MNT" suffix markers that route an otherwise-Unknown query
    /// straight to DN42 without ever attempting RDAP/WHOIS.</summary>
    bool IsExplicitDn42Marker(string query);

    /// <summary>Resolves <paramref name="query"/> against the DN42 registry. Always returns a
    /// (possibly "% 404 Not Found") string, never null — callers decide what "not found" means
    /// for their own fallback logic (see ResolutionOrchestrator).</summary>
    Task<string> ResolveAsync(string query, CancellationToken ct = default);
}
