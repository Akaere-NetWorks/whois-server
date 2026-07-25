using WhoisServer.Core.Query;

namespace WhoisServer.Services;

/// <summary>
/// Registers all Phase 2 "network intelligence" query suffixes into
/// <see cref="QueryAnalyzer"/>'s suffix registry. Called once at startup by the composition root
/// (WhoisServer.Host) — this is the "later phases call RegisterSuffix once at startup" pattern
/// plan.md §4.1 describes, kept in the Services project rather than hardcoded into Core so Core
/// stays decoupled from which concrete services exist.
///
/// Each block below is gated to match Cargo.toml's actual atomic-feature-to-suffix mapping exactly
/// (confirmed by reading core/query.rs's <c>#[cfg(feature = "...")]</c> attributes, not assumed):
/// <c>bgp</c> covers BgpTool/LookingGlass/Irr/Rpki; <c>bgp-irr</c> is the 13 direct-connect
/// registries; <c>bgp-manrs</c>, <c>peeringdb</c>, <c>bgp-prefixes</c>, and <c>geo</c> are each
/// their own flag despite some sharing a service class (<see cref="Geo.GeoAggregatorService"/>
/// handles both the <c>geo</c>-gated Geo/RirGeo suffixes and the <c>bgp-prefixes</c>-gated
/// Prefixes suffix — the class itself isn't feature-gated away, only the suffix registrations and
/// QueryProcessor dispatch cases are, since splitting the class would duplicate its shared HTTP
/// client setup for no behavioral benefit).
/// </summary>
public static class NetworkServicesRegistration
{
    public static void RegisterQueryTypes()
    {
        var help = new List<string> { "% Category: Network Intelligence" };

#if FEATURE_BGP
        QueryAnalyzer.RegisterSuffix("-BGPTOOL", baseQuery => new QueryType.BgpTool(baseQuery));
        QueryAnalyzer.RegisterSuffix("-IRR", resource => new QueryType.Irr(resource));
        QueryAnalyzer.RegisterSuffix("-LG", resource => new QueryType.LookingGlass(resource));
        help.Add("%   <AS/prefix>-BGPTOOL     BGP looking-glass info via bgp.tools");
        help.Add("%   <resource>-IRR          IRR explorer route object lookup");
        help.Add("%   <resource>-LG           Looking glass query");
#endif
#if FEATURE_BGP_MANRS
        QueryAnalyzer.RegisterSuffix("-MANRS", baseQuery => new QueryType.Manrs(baseQuery));
        help.Add("%   AS<n>-MANRS              MANRS compliance status");
#endif
#if FEATURE_PEERINGDB
        QueryAnalyzer.RegisterSuffix("-PEERINGDB", baseQuery => new QueryType.PeeringDb(baseQuery));
        help.Add("%   AS<n>-PEERINGDB          PeeringDB network info");
#endif
#if FEATURE_GEO
        QueryAnalyzer.RegisterSuffix("-RIRGEO", resource => new QueryType.RirGeo(resource));
        QueryAnalyzer.RegisterSuffix("-GEO", resource => new QueryType.Geo(resource));
        help.Add("%   <ip>-RIRGEO              RIR-reported geolocation");
        help.Add("%   <ip>-GEO                 Aggregated geolocation (multiple sources)");
#endif
#if FEATURE_BGP_PREFIXES
        QueryAnalyzer.RegisterSuffix("-PREFIXES", asn => new QueryType.Prefixes(asn));
        help.Add("%   AS<n>-PREFIXES           Announced prefixes for an ASN");
#endif
#if FEATURE_DNS
        QueryAnalyzer.RegisterSuffix("-DNS", baseQuery => new QueryType.Dns(baseQuery));
        help.Add("%   <domain>-DNS             DNS record lookup");
#endif
#if FEATURE_PING
        QueryAnalyzer.RegisterSuffix("-PING", baseQuery => new QueryType.Ping(baseQuery));
        help.Add("%   <host>-PING              Ping via Globalping");
#endif
#if FEATURE_TRACEROUTE
        QueryAnalyzer.RegisterSuffix("-TRACEROUTE", baseQuery => new QueryType.Traceroute(baseQuery));
        QueryAnalyzer.RegisterSuffix("-TRACE", baseQuery => new QueryType.Traceroute(baseQuery));
        help.Add("%   <host>-TRACEROUTE        Traceroute via Globalping");
#endif
#if FEATURE_NTP
        QueryAnalyzer.RegisterSuffix("-NTP", server => new QueryType.Ntp(server));
        help.Add("%   <server>-NTP             NTP server query");
#endif
#if FEATURE_SSL
        QueryAnalyzer.RegisterSuffix("-SSL", baseQuery => new QueryType.Ssl(baseQuery));
        help.Add("%   <domain>-SSL             TLS certificate info");
#endif
#if FEATURE_CRT
        QueryAnalyzer.RegisterSuffix("-CRT", domain => new QueryType.Crt(domain));
        help.Add("%   <domain>-CRT             crt.sh certificate transparency search");
#endif
#if FEATURE_CFSTATUS
        QueryAnalyzer.RegisterSuffix("-CFSTATUS", baseQuery => new QueryType.CfStatus(baseQuery));
        help.Add("%   CFSTATUS                 Cloudflare status summary");
#endif
#if FEATURE_EMAIL
        QueryAnalyzer.RegisterSuffix("-EMAIL", baseQuery => new QueryType.Email(baseQuery));
        help.Add("%   <resource>-EMAIL         DN42 email search");
#endif

#if FEATURE_BGP_IRR
        foreach (var registryKey in Bgp.IrrRegistryConstants.Registries.Keys)
        {
            var key = registryKey; // capture for the closure
            QueryAnalyzer.RegisterSuffix($"-{key}", baseQuery => new QueryType.IrrDirect(key, baseQuery));
        }
        help.Add("%   <resource>-<REGISTRY>    Direct query to a specific IRR registry (e.g. -RADB)");
#endif

        if (help.Count > 1)
        {
            HelpService.AdditionalSections.Add(string.Join("\n", help));
        }
    }
}
