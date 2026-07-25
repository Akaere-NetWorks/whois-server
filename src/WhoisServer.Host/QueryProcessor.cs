using Microsoft.Extensions.Logging;
using WhoisServer.Core.Color;
using WhoisServer.Core.Patch;
using WhoisServer.Core.Query;
using WhoisServer.Services;
#if FEATURE_BGP || FEATURE_BGP_IRR || FEATURE_BGP_MANRS || FEATURE_PEERINGDB
using WhoisServer.Services.Bgp;
#endif
#if FEATURE_STEAM || FEATURE_IMDB || FEATURE_MINECRAFT || FEATURE_ACGC || FEATURE_LYRIC || FEATURE_MEAL || FEATURE_WIKIPEDIA || FEATURE_GITHUB
using WhoisServer.Services.Entertainment;
#endif
#if FEATURE_GEO || FEATURE_BGP_PREFIXES
using WhoisServer.Services.Geo;
#endif
#if FEATURE_ICP
using WhoisServer.Services.Icp;
#endif
#if FEATURE_DNS || FEATURE_PING || FEATURE_TRACEROUTE || FEATURE_NTP || FEATURE_SSL || FEATURE_CRT || FEATURE_CFSTATUS
using WhoisServer.Services.NetworkTools;
#endif
#if FEATURE_PKG_ALMA || FEATURE_PKG_AOSC || FEATURE_PKG_AUR || FEATURE_PKG_CARGO || FEATURE_PKG_CURSEFORGE || FEATURE_PKG_DEBIAN || FEATURE_PKG_EPEL || FEATURE_PKG_MODRINTH || FEATURE_PKG_NIXOS || FEATURE_PKG_NPM || FEATURE_PKG_OPENSUSE || FEATURE_PKG_OPENWRT || FEATURE_PKG_PYPI || FEATURE_PKG_UBUNTU
using WhoisServer.Services.Packages;
#endif
#if FEATURE_PEN
using WhoisServer.Services.Pen;
#endif
#if FEATURE_PIXIV
using WhoisServer.Services.Pixiv;
#endif
using WhoisServer.Services.Rdap;
#if FEATURE_PLUGINS
using WhoisServer.Plugins;
#endif

namespace WhoisServer.Host;

/// <summary>
/// Dispatches a classified query to the right handler, then applies colorization and response
/// patches uniformly — ported from src/core/query_processor.rs's <c>process_query</c>. Every
/// constructor parameter/case below is <c>#if FEATURE_XXX</c>-gated to match Phase 7's full
/// feature-flag matrix (see Directory.Build.props and plan.md §6/§10) — when a feature is off,
/// its service is never constructed, its suffix is never registered (see the
/// <c>*ServicesRegistration</c> classes), and the corresponding <c>QueryType</c> variant simply
/// never gets produced by <see cref="QueryAnalyzer"/>, so the switch below never needs a runtime
/// "is this enabled" check — the case just doesn't exist in that build.
/// </summary>
public sealed class QueryProcessor(
    ResolutionOrchestrator resolution,
    DescService descService,
    RdapClient rdapClient,
    PatchManager patchManager,
#if FEATURE_BGP
    BgpToolService bgpToolService,
    IrrExplorerService irrExplorerService,
    LookingGlassService lookingGlassService,
    RpkiValidationService rpkiValidationService,
#endif
#if FEATURE_BGP_IRR
    IrrDirectService irrDirectService,
#endif
#if FEATURE_BGP_MANRS
    ManrsService manrsService,
#endif
#if FEATURE_PEERINGDB
    PeeringDbService peeringDbService,
#endif
#if FEATURE_GEO || FEATURE_BGP_PREFIXES
    GeoAggregatorService geoAggregatorService,
#endif
#if FEATURE_DNS
    DnsService dnsService,
#endif
#if FEATURE_PING
    PingService pingService,
#endif
#if FEATURE_TRACEROUTE
    TracerouteService tracerouteService,
#endif
#if FEATURE_NTP
    NtpService ntpService,
#endif
#if FEATURE_SSL
    SslService sslService,
#endif
#if FEATURE_CRT
    CrtService crtService,
#endif
#if FEATURE_CFSTATUS
    CfStatusService cfStatusService,
#endif
#if FEATURE_EMAIL
    EmailSearchService emailSearchService,
#endif
#if FEATURE_ICP
    IcpService icpService,
#endif
#if FEATURE_PEN
    PenService penService,
#endif
#if FEATURE_PIXIV
    PixivService pixivService,
#endif
#if FEATURE_STEAM
    SteamService steamService,
#endif
#if FEATURE_IMDB
    ImdbService imdbService,
#endif
#if FEATURE_MINECRAFT
    MinecraftService minecraftService,
#endif
#if FEATURE_ACGC
    AcgcService acgcService,
#endif
#if FEATURE_LYRIC
    LyricService lyricService,
#endif
#if FEATURE_MEAL
    MealService mealService,
#endif
#if FEATURE_WIKIPEDIA
    WikipediaService wikipediaService,
#endif
#if FEATURE_GITHUB
    GitHubService gitHubService,
#endif
#if FEATURE_PKG_ALMA
    AlmaPackageService almaPackageService,
#endif
#if FEATURE_PKG_AOSC
    AoscPackageService aoscPackageService,
#endif
#if FEATURE_PKG_AUR
    AurPackageService aurPackageService,
#endif
#if FEATURE_PKG_CARGO
    CargoPackageService cargoPackageService,
#endif
#if FEATURE_PKG_CURSEFORGE
    CurseForgePackageService curseForgePackageService,
#endif
#if FEATURE_PKG_DEBIAN
    DebianPackageService debianPackageService,
#endif
#if FEATURE_PKG_EPEL
    EpelPackageService epelPackageService,
#endif
#if FEATURE_PKG_MODRINTH
    ModrinthPackageService modrinthPackageService,
#endif
#if FEATURE_PKG_NIXOS
    NixOsPackageService nixOsPackageService,
#endif
#if FEATURE_PKG_NPM
    NpmPackageService npmPackageService,
#endif
#if FEATURE_PKG_PYPI
    PypiPackageService pypiPackageService,
#endif
#if FEATURE_PKG_OPENSUSE
    OpenSusePackageService openSusePackageService,
#endif
#if FEATURE_PKG_OPENWRT
    OpenWrtPackageService openWrtPackageService,
#endif
#if FEATURE_PKG_UBUNTU
    UbuntuPackageService ubuntuPackageService,
#endif
#if FEATURE_PLUGINS
    PluginRegistry pluginRegistry,
#endif
    ILogger<QueryProcessor> logger)
{
    public async Task<string> ProcessAsync(string query, QueryType queryType, ColorScheme? colorScheme, CancellationToken ct = default)
    {
        var response = await DispatchAsync(query, queryType, ct);

        if (colorScheme is not null)
        {
            response = new Colorizer(colorScheme.Value).ColorizeResponse(response, queryType);
        }

        return patchManager.ApplyPatches(query, response);
    }

    private async Task<string> DispatchAsync(string query, QueryType queryType, CancellationToken ct)
    {
        switch (queryType)
        {
            case QueryType.Help:
                return HelpService.GenerateResponse();

            case QueryType.UpdatePatch:
                return await patchManager.UpdateFromRemoteAsync(ct: ct);

            case QueryType.Desc desc:
                return await descService.ProcessAsync(desc.BaseQuery, ct);

            case QueryType.Rdap rdap:
                return await ProcessExplicitRdapAsync(rdap.BaseQuery, ct);

            case QueryType.Domain or QueryType.IPv4 or QueryType.IPv6 or QueryType.Asn
                or QueryType.BareAsn or QueryType.Unknown:
                return await resolution.ResolveAsync(query, queryType, ct);

#if FEATURE_BGP
            case QueryType.BgpTool bgpTool:
                return await bgpToolService.ProcessAsync(bgpTool.BaseQuery, ct);

            case QueryType.Irr irr:
                return await irrExplorerService.ProcessAsync(irr.Resource, ct);

            case QueryType.LookingGlass lg:
                return await lookingGlassService.ProcessAsync(lg.Resource, ct);

            case QueryType.Rpki rpki:
                return await rpkiValidationService.ProcessAsync(rpki.Prefix, rpki.AsnValue, ct);
#endif

#if FEATURE_BGP_IRR
            case QueryType.IrrDirect irrDirect:
                return await irrDirectService.ProcessAsync(irrDirect.RegistryKey, irrDirect.BaseQuery, ct);
#endif

#if FEATURE_BGP_MANRS
            case QueryType.Manrs manrs:
                return await manrsService.ProcessAsync(manrs.BaseQuery, ct);
#endif

#if FEATURE_PEERINGDB
            case QueryType.PeeringDb peeringDb:
                return await peeringDbService.ProcessAsync(peeringDb.BaseQuery, ct);
#endif

#if FEATURE_GEO
            case QueryType.Geo geo:
                return await geoAggregatorService.ProcessGeoAsync(geo.Resource, ct);

            case QueryType.RirGeo rirGeo:
                return await geoAggregatorService.ProcessRirGeoAsync(rirGeo.Resource, ct);
#endif

#if FEATURE_BGP_PREFIXES
            case QueryType.Prefixes prefixes:
                return await geoAggregatorService.ProcessPrefixesAsync(prefixes.AsnValue, ct);
#endif

#if FEATURE_DNS
            case QueryType.Dns dns:
                return await dnsService.ProcessAsync(dns.BaseQuery, ct);
#endif

#if FEATURE_PING
            case QueryType.Ping ping:
                return await pingService.ProcessAsync(ping.BaseQuery, ct);
#endif

#if FEATURE_TRACEROUTE
            case QueryType.Traceroute traceroute:
                return await tracerouteService.ProcessAsync(traceroute.BaseQuery, ct);
#endif

#if FEATURE_NTP
            case QueryType.Ntp ntp:
                return await ntpService.ProcessAsync(ntp.Server, ct);
#endif

#if FEATURE_SSL
            case QueryType.Ssl ssl:
                return await sslService.ProcessAsync(ssl.BaseQuery, ct);
#endif

#if FEATURE_CRT
            case QueryType.Crt crt:
                return await crtService.ProcessAsync(crt.DomainName, ct);
#endif

#if FEATURE_CFSTATUS
            case QueryType.CfStatus cfStatus:
                return await cfStatusService.ProcessAsync(cfStatus.BaseQuery, ct);
#endif

#if FEATURE_EMAIL
            case QueryType.Email email:
                return await emailSearchService.ProcessAsync(email.BaseQuery, ct);
#endif

            // ── Phase 5: remaining integrations ─────────────────────────────
#if FEATURE_ICP
            case QueryType.Icp icp:
                return await icpService.ProcessAsync(icp.BaseQuery, ct);
#endif

#if FEATURE_PEN
            case QueryType.Pen pen:
                return await penService.HandleQueryAsync(pen.BaseQuery, ct);
#endif

#if FEATURE_PIXIV
            case QueryType.Pixiv pixiv:
                return await pixivService.ProcessAsync(pixiv.BaseQuery, ct);
#endif

#if FEATURE_STEAM
            case QueryType.Steam steam:
                return await steamService.ProcessAsync(steam.BaseQuery, ct);

            case QueryType.SteamSearch steamSearch:
                return await steamService.ProcessSearchAsync(steamSearch.BaseQuery, ct);
#endif

#if FEATURE_IMDB
            case QueryType.Imdb imdb:
                return await imdbService.ProcessAsync(imdb.BaseQuery, ct);

            case QueryType.ImdbSearch imdbSearch:
                return await imdbService.ProcessSearchAsync(imdbSearch.BaseQuery, ct);
#endif

#if FEATURE_MINECRAFT
            case QueryType.Minecraft minecraft:
                return await minecraftService.ProcessServerAsync(minecraft.BaseQuery, ct);

            case QueryType.MinecraftUser minecraftUser:
                return await minecraftService.ProcessUserAsync(minecraftUser.BaseQuery, ct);
#endif

#if FEATURE_ACGC
            case QueryType.Acgc acgc:
                return await acgcService.ProcessAsync(acgc.BaseQuery, ct);
#endif

#if FEATURE_LYRIC
            case QueryType.Lyric lyric:
                return await lyricService.ProcessAsync(lyric.BaseQuery, ct);
#endif

#if FEATURE_MEAL
            case QueryType.Meal:
                return await mealService.ProcessAsync(ct);

            case QueryType.MealCN:
                return await mealService.ProcessCnAsync(ct);
#endif

#if FEATURE_WIKIPEDIA
            case QueryType.Wikipedia wikipedia:
                return await wikipediaService.ProcessAsync(wikipedia.BaseQuery, ct);
#endif

#if FEATURE_GITHUB
            case QueryType.GitHub gitHub:
                return await gitHubService.ProcessAsync(gitHub.BaseQuery, ct);
#endif

#if FEATURE_PKG_ALMA
            case QueryType.Alma alma:
                return await almaPackageService.ProcessAsync(alma.PackageName, ct);
#endif

#if FEATURE_PKG_AOSC
            case QueryType.Aosc aosc:
                return await aoscPackageService.ProcessAsync(aosc.PackageName, ct);
#endif

#if FEATURE_PKG_AUR
            case QueryType.Aur aur:
                return await aurPackageService.ProcessAsync(aur.PackageName, ct);
#endif

#if FEATURE_PKG_CARGO
            case QueryType.Cargo cargo:
                return await cargoPackageService.ProcessAsync(cargo.PackageName, ct);
#endif

#if FEATURE_PKG_CURSEFORGE
            case QueryType.CurseForge curseForge:
                return await curseForgePackageService.ProcessAsync(curseForge.PackageName, ct);
#endif

#if FEATURE_PKG_DEBIAN
            case QueryType.Debian debian:
                return await debianPackageService.ProcessAsync(debian.PackageName, ct);
#endif

#if FEATURE_PKG_EPEL
            case QueryType.Epel epel:
                return await epelPackageService.ProcessAsync(epel.PackageName, ct);
#endif

#if FEATURE_PKG_MODRINTH
            case QueryType.Modrinth modrinth:
                return await modrinthPackageService.ProcessAsync(modrinth.PackageName, ct);
#endif

#if FEATURE_PKG_NIXOS
            case QueryType.NixOs nixOs:
                return await nixOsPackageService.ProcessAsync(nixOs.PackageName, ct);
#endif

#if FEATURE_PKG_NPM
            case QueryType.Npm npm:
                return await npmPackageService.ProcessAsync(npm.PackageName, ct);
#endif

#if FEATURE_PKG_PYPI
            case QueryType.Pypi pypi:
                return await pypiPackageService.ProcessAsync(pypi.PackageName, ct);
#endif

#if FEATURE_PKG_OPENSUSE
            case QueryType.OpenSuse openSuse:
                return await openSusePackageService.ProcessAsync(openSuse.PackageName, ct);
#endif

#if FEATURE_PKG_OPENWRT
            case QueryType.OpenWrt openWrt:
                return await openWrtPackageService.ProcessAsync(openWrt.PackageName, ct);
#endif

#if FEATURE_PKG_UBUNTU
            case QueryType.Ubuntu ubuntu:
                return await ubuntuPackageService.ProcessAsync(ubuntu.PackageName, ct);
#endif

#if FEATURE_PLUGINS
            // ── Phase 6: native plugin system ────────────────────────────────
            case QueryType.Plugin plugin:
                return await pluginRegistry.ProcessAsync(plugin.Suffix, plugin.BaseQuery, ct);
#endif

            default:
                throw new NotSupportedException($"No handler registered for query type {queryType.GetType().Name}");
        }
    }

    private async Task<string> ProcessExplicitRdapAsync(string baseQuery, CancellationToken ct)
    {
        var innerType = QueryAnalyzer.Analyze(baseQuery);
        if (RdapClient.BuildRequestPath(innerType) is null)
        {
            return $"""
                % RDAP Query Error
                % Unable to detect query type for: {baseQuery}
                %
                % Supported query types:
                %   - Domain names (e.g., example.com)
                %   - IP addresses (e.g., 8.8.8.8, 2001:4860:4860::8888)
                %   - AS numbers (e.g., AS15169 or 15169)

                """;
        }

        try
        {
            return await rdapClient.QueryAsync(baseQuery, innerType, ct);
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "Explicit RDAP query failed for {Query}", baseQuery);
            return $"""
                % RDAP Query Failed
                % Query: {baseQuery}
                % Error: {e.Message}
                %
                % This may be due to:
                %   - Network connectivity issues
                %   - RDAP service unavailable
                %   - Invalid or non-existent resource

                """;
        }
    }
}
