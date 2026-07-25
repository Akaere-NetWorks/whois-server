using System.Net;
using System.Text.RegularExpressions;

namespace WhoisServer.Core.Query;

/// <summary>
/// Classifies a raw WHOIS query string into a <see cref="QueryType"/>.
///
/// This replaces the Rust version's single giant ordered if-chain (one branch per Cargo feature)
/// with a runtime suffix registry (plan.md §4.1): later phases call <see cref="RegisterSuffix"/>
/// once at startup for each service they add, instead of editing a shared function. Suffixes are
/// always tried longest-first so a newly registered short suffix can never accidentally shadow an
/// existing longer one (or vice versa) regardless of registration order.
/// </summary>
public static class QueryAnalyzer
{
    // Mirrors the Rust DOMAIN_REGEX in core/query.rs exactly: at least one label + dot, TLD >= 2 alpha chars.
    private static readonly Regex DomainRegex = new(
        @"^([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$",
        RegexOptions.Compiled);

    private static readonly object RegistryLock = new();
    private static readonly List<QuerySuffixDefinition> Suffixes = [];

    static QueryAnalyzer()
    {
        RegisterSuffix("-DESC", baseQuery => new QueryType.Desc(baseQuery));
        RegisterSuffix("-RDAP", baseQuery => new QueryType.Rdap(baseQuery));
    }

    /// <summary>
    /// Registers a query suffix (e.g. "-STEAM"). Idempotent-ish: registering the same suffix
    /// twice just adds two entries — callers (DI registration at startup) are expected to call
    /// this once per suffix, so no de-duplication is attempted here.
    /// </summary>
    public static void RegisterSuffix(string suffix, Func<string, QueryType> factory)
    {
        ArgumentException.ThrowIfNullOrEmpty(suffix);
        var upperSuffix = suffix.ToUpperInvariant();

        lock (RegistryLock)
        {
            var insertAt = Suffixes.FindIndex(s => s.Suffix.Length < upperSuffix.Length);
            var definition = new QuerySuffixDefinition(upperSuffix, factory);
            if (insertAt < 0)
            {
                Suffixes.Add(definition);
            }
            else
            {
                Suffixes.Insert(insertAt, definition);
            }
        }
    }

    /// <summary>Whether <paramref name="suffix"/> (case-insensitive) is already claimed by a
    /// registered suffix. Used by the Phase 6 plugin loader's collision check before registering a
    /// plugin's suffix — stricter than the Rust version's plugin registry, which only checked for
    /// collisions among plugins themselves, never against built-in suffixes. Does NOT catch the
    /// handful of suffixes special-cased directly in <see cref="Analyze"/> before the registry
    /// loop runs (<c>-RPKI</c>, <c>-MEAL</c>, <c>-MEAL-CN</c>) — those never go through
    /// <see cref="RegisterSuffix"/> in the first place, so there's nothing here to check them
    /// against; a plugin claiming one of those exact suffixes would still be shadowed at query time
    /// (documented limitation, not silently unsafe: the special-cased branches run first and always
    /// win).</summary>
    public static bool IsSuffixRegistered(string suffix)
    {
        var upperSuffix = suffix.ToUpperInvariant();
        lock (RegistryLock)
        {
            return Suffixes.Exists(s => s.Suffix == upperSuffix);
        }
    }

    public static QueryType Analyze(string query)
    {
        ArgumentNullException.ThrowIfNull(query);
        var upper = query.ToUpperInvariant();

        // Meal/MealCN have no base query to carry (trigger is an exact Chinese phrase OR a fixed
        // suffix with nothing meaningful before it), so — like RPKI below — they can't be simple
        // Func<string, QueryType> suffix-registry entries. Checked first, matching
        // core/query.rs's analyze_query ordering (before even HELP).
#if FEATURE_MEAL
        if (query == "今天吃什么中国" || upper.EndsWith("-MEAL-CN", StringComparison.Ordinal))
        {
            return new QueryType.MealCN();
        }

        if (query == "今天吃什么" || upper.EndsWith("-MEAL", StringComparison.Ordinal))
        {
            return new QueryType.Meal();
        }
#endif

        if (upper == "HELP")
        {
            return new QueryType.Help();
        }

        if (upper is "UPDATE-PATCH" or "-UPDATE-PATCH")
        {
            return new QueryType.UpdatePatch();
        }

        // "-RPKI" is a two-part suffix (prefix-asn-RPKI), not a simple fixed-suffix strip, so it's
        // parsed specially and checked before the generic registry — matching core/query.rs's
        // ordering. A malformed RPKI-suffixed query returns Unknown immediately rather than
        // falling through to the suffix registry (ported behavior, not a new decision).
        if (upper.EndsWith("-RPKI", StringComparison.Ordinal))
        {
            return (QueryType?)TryParseRpki(query) ?? new QueryType.Unknown(query);
        }

        QuerySuffixDefinition[] suffixSnapshot;
        lock (RegistryLock)
        {
            suffixSnapshot = [.. Suffixes];
        }

        foreach (var definition in suffixSnapshot)
        {
            if (upper.EndsWith(definition.Suffix, StringComparison.Ordinal))
            {
                // Slice the original-case query, not the upper-cased copy, so callers see the
                // base query in whatever case the client actually sent it.
                var baseQuery = query[..^definition.Suffix.Length];
                return definition.Factory(baseQuery);
            }
        }

        // .dn42 domains never match DomainRegex below (its TLD group requires letters only, and
        // "dn42" contains digits) — ported from core/query.rs, which needs the same explicit
        // check for the same reason. Checked here even when the DN42 feature isn't compiled in
        // (Core has no feature flags of its own): with no DN42 router registered, a .dn42 query
        // just classifies as an ordinary Domain and predictably fails RDAP/WHOIS resolution.
        if (upper.EndsWith(".DN42", StringComparison.Ordinal))
        {
            return new QueryType.Domain(query);
        }

        // "-DN42"/"-MNT" (DN42 person/maintainer handles) don't match any other shape here, so
        // they'd fall through to Unknown regardless — this explicit check just matches
        // core/query.rs's intent directly rather than relying on default fallthrough.
        if (upper.EndsWith("-DN42", StringComparison.Ordinal) || upper.EndsWith("-MNT", StringComparison.Ordinal))
        {
            return new QueryType.Unknown(query);
        }

        if (IPAddress.TryParse(query, out var ip))
        {
            if (ip.AddressFamily == System.Net.Sockets.AddressFamily.InterNetworkV6)
            {
                return new QueryType.IPv6(ip);
            }

            if (ip.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork && StrictIpText.IsStrictIpv4(query))
            {
                return new QueryType.IPv4(ip);
            }

            // Legacy inet_aton-style match (e.g. a bare integer) — not a real IP, fall through.
        }

        if (IPNetwork.TryParse(query, out var network))
        {
            var isV6 = network.BaseAddress.AddressFamily == System.Net.Sockets.AddressFamily.InterNetworkV6;
            if (isV6 || StrictIpText.IsStrictIpv4(network.BaseAddress.ToString()))
            {
                return isV6 ? new QueryType.IPv6(network.BaseAddress) : new QueryType.IPv4(network.BaseAddress);
            }
        }

        if (upper.StartsWith("AS", StringComparison.Ordinal)
            && query.Length > 2
            && query[2..].All(char.IsAsciiDigit))
        {
            return new QueryType.Asn(query);
        }

        if (query.Length > 0 && query.All(char.IsAsciiDigit))
        {
            return new QueryType.BareAsn(query);
        }

        if (DomainRegex.IsMatch(query))
        {
            return new QueryType.Domain(query);
        }

        return new QueryType.Unknown(query);
    }

    /// <summary>Parses the "prefix-asn" payload preceding a "-RPKI" suffix. Ported from
    /// core/query.rs: strip "-RPKI", split on the *last* '-', require the ASN half to be all
    /// digits, and accept the prefix half as an IPv4/IPv6 CIDR or a bare IP (auto-appending
    /// /32 or /128). Returns null on any parse failure.</summary>
    private static QueryType.Rpki? TryParseRpki(string query)
    {
        var baseQuery = query[..^"-RPKI".Length];
        var dashIndex = baseQuery.LastIndexOf('-');
        if (dashIndex < 0)
        {
            return null;
        }

        var prefixPart = baseQuery[..dashIndex];
        var asnPart = baseQuery[(dashIndex + 1)..];

        if (asnPart.Length == 0 || !asnPart.All(char.IsAsciiDigit))
        {
            return null;
        }

        if (IPNetwork.TryParse(prefixPart, out _))
        {
            return new QueryType.Rpki(prefixPart, asnPart);
        }

        if (IPAddress.TryParse(prefixPart, out var ip))
        {
            var withMask = ip.AddressFamily == System.Net.Sockets.AddressFamily.InterNetworkV6
                ? $"{prefixPart}/128"
                : $"{prefixPart}/32";
            return new QueryType.Rpki(withMask, asnPart);
        }

        return null;
    }
}
