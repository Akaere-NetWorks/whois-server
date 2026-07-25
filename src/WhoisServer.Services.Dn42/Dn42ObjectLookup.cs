#if FEATURE_DN42
namespace WhoisServer.Services.Dn42;

/// <summary>
/// Ported from src/dn42/git_backend.rs's <c>handle_object_query</c>/<c>handle_object_query_raw</c>
/// (the two were identical ~100-line copies in Rust; collapsed into one here — plan.md flagged
/// this duplication as a simplification opportunity). Each candidate is tried in the exact order
/// below; the first one whose storage key actually exists wins. Note this is genuinely
/// order-and-existence-dependent, not "classify once then look up": a query ending in "-MNT" with
/// no matching mntner object keeps falling through to later checks (as-set, then the unconditional
/// DNS fallback) rather than failing immediately — <see cref="GetCandidates"/> yields candidates
/// lazily so the caller can do exactly that.
/// </summary>
public static class Dn42ObjectLookup
{
    /// <summary>Zero-pads a short numeric ASN (1-4 digits) into DN42's AS4242420000-AS4242429999
    /// block; passes longer numbers/already-AS-prefixed values through with just an "AS" prefix.
    /// Returns null if <paramref name="query"/> isn't ASN-shaped at all.</summary>
    public static string? ParseAsn(string query)
    {
        var normalized = query.ToUpperInvariant();

        if (normalized.Length > 0 && normalized.All(char.IsAsciiDigit))
        {
            return PadAsn(normalized);
        }

        if (normalized.StartsWith("AS", StringComparison.Ordinal))
        {
            var digits = normalized[2..];
            if (digits.Length > 0 && digits.All(char.IsAsciiDigit))
            {
                return PadAsn(digits);
            }
        }

        return null;
    }

    private static string PadAsn(string digits) => digits.Length switch
    {
        1 => $"AS424242000{digits}",
        2 => $"AS42424200{digits}",
        3 => $"AS4242420{digits}",
        4 => $"AS424242{digits}",
        _ => $"AS{digits}",
    };

    public static IEnumerable<(string ObjectType, string Key)> GetCandidates(string query)
    {
        var normalized = query.ToUpperInvariant();

        var asn = ParseAsn(query);
        if (asn is not null)
        {
            yield return ("aut-num", asn);
        }

        if (normalized.EndsWith("-DN42", StringComparison.Ordinal)
            || normalized.EndsWith("-NEONETWORK", StringComparison.Ordinal)
            || normalized.EndsWith("-CRXN", StringComparison.Ordinal))
        {
            yield return ("person", normalized);
        }

        if (normalized.EndsWith("-MNT", StringComparison.Ordinal))
        {
            yield return ("mntner", normalized);
        }

        if (normalized.EndsWith("-SCHEMA", StringComparison.Ordinal))
        {
            yield return ("schema", normalized);
        }

        if (normalized.StartsWith("ORG-", StringComparison.Ordinal))
        {
            yield return ("organisation", normalized);
        }

        if (normalized.StartsWith("SET-", StringComparison.Ordinal) && normalized.EndsWith("-TINC", StringComparison.Ordinal))
        {
            yield return ("tinc-keyset", normalized);
        }

        if (normalized.EndsWith("-TINC", StringComparison.Ordinal) && !normalized.StartsWith("SET-", StringComparison.Ordinal))
        {
            yield return ("tinc-key", normalized);
        }

        if (normalized.StartsWith("RS-", StringComparison.Ordinal))
        {
            yield return ("route-set", normalized);
        }

        if (normalized.StartsWith("AS", StringComparison.Ordinal) && normalized.Contains("-AS", StringComparison.Ordinal))
        {
            yield return ("as-block", normalized);
        }

        if (normalized.StartsWith("AS", StringComparison.Ordinal) && !normalized[2..].All(char.IsAsciiDigit))
        {
            yield return ("as-set", normalized);
        }

        // Unconditional fallback — tried regardless of whether the query "looked like" anything
        // above, matching the Rust version exactly.
        yield return ("dns", query.ToLowerInvariant());
    }
}
#endif
