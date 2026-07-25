using System.Net;

namespace WhoisServer.Core.Query;

/// <summary>
/// Classification result for a raw query string. Sealed record hierarchy standing in for the
/// Rust <c>QueryType</c> enum — pattern-match on the concrete type instead of a discriminant.
/// New variants are added here (and to the suffix registry in <see cref="QueryAnalyzer"/>) as
/// later phases implement the corresponding service, not gated behind compile-time flags the way
/// the Rust enum's variants were.
/// </summary>
public abstract record QueryType
{
    private QueryType() { }

    public sealed record Domain(string Name) : QueryType;

    public sealed record IPv4(IPAddress Address) : QueryType;

    public sealed record IPv6(IPAddress Address) : QueryType;

    public sealed record Asn(string Value) : QueryType;

    public sealed record BareAsn(string Value) : QueryType;

    /// <summary>The <c>-DESC</c> suffix: <paramref name="BaseQuery"/> is the query with the suffix stripped.</summary>
    public sealed record Desc(string BaseQuery) : QueryType;

    /// <summary>The explicit <c>-RDAP</c> suffix.</summary>
    public sealed record Rdap(string BaseQuery) : QueryType;

    public sealed record Help : QueryType;

    public sealed record UpdatePatch : QueryType;

    public sealed record Unknown(string Raw) : QueryType;

    // ── Phase 2: network intelligence services ────────────────────────────

    public sealed record BgpTool(string BaseQuery) : QueryType;

    public sealed record Irr(string Resource) : QueryType;

    public sealed record LookingGlass(string Resource) : QueryType;

    /// <summary>The <c>prefix-asn-RPKI</c> two-part suffix — parsed specially in
    /// <see cref="QueryAnalyzer"/> before the generic suffix registry loop, since it isn't a
    /// simple fixed-suffix strip (see plan.md's port of core/query.rs's RPKI branch). Parameter
    /// named <c>AsnValue</c> rather than <c>Asn</c> — a nested type of the same name already
    /// exists on this sealed hierarchy and record positional properties can't share a name with
    /// an inherited nested type.</summary>
    public sealed record Rpki(string Prefix, string AsnValue) : QueryType;

    /// <summary>One of the 12 direct-connect IRR registries (RADB, ALTDB, AFRINIC, APNIC, ARIN,
    /// BELL, JPIRR, LACNIC, LEVEL3, NTTCOM, RIPE, RIS, TC) — a single variant parameterized by
    /// registry key rather than 12 near-identical ones, since they only differ in which WHOIS
    /// server/port the IRR direct-connect service (in WhoisServer.Services) dispatches to.</summary>
    public sealed record IrrDirect(string RegistryKey, string BaseQuery) : QueryType;

    public sealed record Manrs(string BaseQuery) : QueryType;

    public sealed record PeeringDb(string BaseQuery) : QueryType;

    public sealed record Geo(string Resource) : QueryType;

    public sealed record RirGeo(string Resource) : QueryType;

    public sealed record Prefixes(string AsnValue) : QueryType;

    public sealed record Dns(string BaseQuery) : QueryType;

    public sealed record Ping(string BaseQuery) : QueryType;

    public sealed record Traceroute(string BaseQuery) : QueryType;

    public sealed record Ntp(string Server) : QueryType;

    public sealed record Ssl(string BaseQuery) : QueryType;

    public sealed record Crt(string DomainName) : QueryType;

    public sealed record CfStatus(string BaseQuery) : QueryType;

    // ── Phase 5: remaining integrations ─────────────────────────────────────

    public sealed record Icp(string BaseQuery) : QueryType;

    public sealed record Pen(string BaseQuery) : QueryType;

    public sealed record Pixiv(string BaseQuery) : QueryType;

    public sealed record Lyric(string BaseQuery) : QueryType;

    /// <summary>Trigger is an exact Chinese phrase match OR the <c>-MEAL</c>/<c>-MEAL-CN</c>
    /// suffix — special-cased in <see cref="QueryAnalyzer"/> before the generic suffix registry
    /// loop (no base query to carry, unlike every other suffix-driven variant here).</summary>
    public sealed record Meal : QueryType;

    public sealed record MealCN : QueryType;

    public sealed record Wikipedia(string BaseQuery) : QueryType;

    public sealed record GitHub(string BaseQuery) : QueryType;

    public sealed record Steam(string BaseQuery) : QueryType;

    public sealed record SteamSearch(string BaseQuery) : QueryType;

    public sealed record Imdb(string BaseQuery) : QueryType;

    public sealed record ImdbSearch(string BaseQuery) : QueryType;

    public sealed record Minecraft(string BaseQuery) : QueryType;

    public sealed record MinecraftUser(string BaseQuery) : QueryType;

    public sealed record Acgc(string BaseQuery) : QueryType;

    public sealed record Alma(string PackageName) : QueryType;

    public sealed record Aosc(string PackageName) : QueryType;

    public sealed record Aur(string PackageName) : QueryType;

    public sealed record Cargo(string PackageName) : QueryType;

    public sealed record CurseForge(string PackageName) : QueryType;

    public sealed record Debian(string PackageName) : QueryType;

    public sealed record Epel(string PackageName) : QueryType;

    public sealed record Modrinth(string PackageName) : QueryType;

    public sealed record NixOs(string PackageName) : QueryType;

    public sealed record Npm(string PackageName) : QueryType;

    public sealed record OpenSuse(string PackageName) : QueryType;

    public sealed record OpenWrt(string PackageName) : QueryType;

    public sealed record Pypi(string PackageName) : QueryType;

    public sealed record Ubuntu(string PackageName) : QueryType;

    // ── Phase 6: native plugin system ───────────────────────────────────────

    /// <summary>Generic passthrough for any suffix claimed by a dynamically-loaded plugin — unlike
    /// every other variant here, plugins aren't known at compile time, so there's no dedicated
    /// record per plugin (mirrors the Rust enum's own <c>Plugin(String, String)</c> variant, which
    /// carries the suffix and base query for exactly this reason).</summary>
    public sealed record Plugin(string Suffix, string BaseQuery) : QueryType;

    // ── Phase 7: filled gap from Phase 2 (found while building the full feature-flag matrix) ──

    /// <summary>The <c>-EMAIL</c> suffix — a DN42-specific recursive email-address aggregator
    /// (see <c>EmailSearchService</c>), not a general internet-WHOIS email lookup despite the
    /// generic-sounding name.</summary>
    public sealed record Email(string BaseQuery) : QueryType;
}
