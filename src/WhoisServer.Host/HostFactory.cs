using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Patch;
using WhoisServer.Core.Storage;
using WhoisServer.Core.Stats;
using WhoisServer.Host.Web;
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
#if FEATURE_TELEMETRY
using WhoisServer.Services.Telemetry;
#endif
using WhoisServer.Services.Whois;
#if FEATURE_DN42
using WhoisServer.Core.Dn42;
using WhoisServer.Services.Dn42;
#endif
#if FEATURE_PLUGINS
using WhoisServer.Plugins;
#endif

namespace WhoisServer.Host;

/// <summary>
/// DI wiring extracted from Program.cs so integration tests can build the exact same host
/// (against an isolated cache directory / test port) instead of duplicating the registration list
/// and risking it drifting out of sync with what actually ships. Builds a <see cref="WebApplication"/>
/// (not a plain Generic Host) as of Phase 4: it still implements <see cref="IHost"/> so the WHOIS
/// TCP listener and other BackgroundServices work exactly as before, but now Kestrel also serves
/// the dashboard/API on <c>cliOptions.WebPort</c>.
/// </summary>
public static class HostFactory
{
    public static WebApplication Build(CliOptions cliOptions, string cacheDir, string[]? args = null)
    {
        var builder = WebApplication.CreateBuilder(args ?? []);
        builder.WebHost.UseUrls($"http://{cliOptions.Host}:{cliOptions.WebPort}");

        builder.Logging.SetMinimumLevel(cliOptions.Debug || cliOptions.Trace ? LogLevel.Debug : LogLevel.Information);

        builder.Services.AddSingleton(cliOptions);
        builder.Services.AddHttpClient();
#if FEATURE_WEB
        builder.Services.AddOpenApi();
#endif

        Directory.CreateDirectory(cacheDir);
        builder.Services.AddSingleton<IKvStore>(_ => new SqliteKvStore(Path.Combine(cacheDir, "patches.db")));

        builder.Services.AddSingleton(sp => new StatsManager(
            new SqliteKvStore(Path.Combine(cacheDir, "stats.db")), sp.GetRequiredService<ILogger<StatsManager>>()));

        builder.Services.AddSingleton<WhoisClient>();
        builder.Services.AddSingleton(sp => new IanaReferralCache(
            sp.GetRequiredService<WhoisClient>(),
            new SqliteKvStore(Path.Combine(cacheDir, "iana-referral.db")),
            sp.GetRequiredService<ILogger<IanaReferralCache>>()));
        builder.Services.AddSingleton<WhoisResolutionService>();
        builder.Services.AddHttpClient<RdapClient>();
        builder.Services.AddSingleton<ResolutionOrchestrator>();
        builder.Services.AddSingleton<DescService>();
        builder.Services.AddSingleton<PatchManager>();

#if FEATURE_DN42
        // ── Phase 3: DN42 ────────────────────────────────────────────────────
        var dn42RegistryPath = Path.Combine(cacheDir, "dn42-registry");
        var dn42Store = new SqliteKvStore(Path.Combine(cacheDir, "dn42.db"));
        builder.Services.AddSingleton(_ => new Dn42Registry(dn42Store));
        builder.Services.AddSingleton<IDn42QueryRouter>(sp => sp.GetRequiredService<Dn42Registry>());
        builder.Services.AddSingleton(sp => new Dn42RegistrySync(
            dn42RegistryPath, dn42Store, sp.GetRequiredService<ILogger<Dn42RegistrySync>>()));
        builder.Services.AddHostedService<Dn42SyncHostedService>();
#endif

        // ── Phase 2: network intelligence services ─────────────────────────
        // (Individual registrations below are #if-gated per atomic feature; the suffix
        // registration call itself never needs the #if since it's a no-op when nothing inside
        // registered anything — see NetworkServicesRegistration for the per-suffix gating.)
        NetworkServicesRegistration.RegisterQueryTypes();

#if FEATURE_BGP
        builder.Services.AddSingleton<BgpToolService>();
        builder.Services.AddHttpClient<IrrExplorerService>();
        builder.Services.AddHttpClient<LookingGlassService>();
        builder.Services.AddHttpClient<RpkiValidationService>();
#endif
#if FEATURE_BGP_IRR
        builder.Services.AddSingleton<IrrDirectService>();
#endif
#if FEATURE_BGP_MANRS
        builder.Services.AddSingleton(sp => new ManrsService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            new SqliteKvStore(Path.Combine(cacheDir, "manrs.db"))));
#endif
#if FEATURE_PEERINGDB
        builder.Services.AddSingleton(sp => new PeeringDbService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            new SqliteKvStore(Path.Combine(cacheDir, "peeringdb.db"))));
#endif
#if FEATURE_GEO || FEATURE_BGP_PREFIXES
        // GeoAggregatorService is shared by the geo-gated Geo/RirGeo suffixes and the separately-
        // gated bgp-prefixes Prefixes suffix (see NetworkServicesRegistration's doc comment) — it
        // needs registering if EITHER flag is on.
        builder.Services.AddSingleton(sp => new GeoAggregatorService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            Environment.GetEnvironmentVariable("IPINFO_TOKEN") ?? string.Empty));
#endif
#if FEATURE_DNS
        builder.Services.AddHttpClient<DohClient>();
        builder.Services.AddSingleton<DnsService>();
#endif
#if FEATURE_PING || FEATURE_TRACEROUTE
        // GlobalpingClient backs both Ping and Traceroute — needs registering if either is on.
        builder.Services.AddHttpClient<GlobalpingClient>();
        builder.Services.AddSingleton(sp => new IpInfoClient(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            Environment.GetEnvironmentVariable("IPINFO_API_TOKEN")));
#endif
#if FEATURE_PING
        builder.Services.AddSingleton<PingService>();
#endif
#if FEATURE_TRACEROUTE
        builder.Services.AddSingleton<TracerouteService>();
#endif
#if FEATURE_NTP
        builder.Services.AddSingleton<NtpService>();
#endif
#if FEATURE_SSL
        builder.Services.AddSingleton<SslService>();
#endif
#if FEATURE_CRT
        builder.Services.AddHttpClient<CrtService>();
#endif
#if FEATURE_CFSTATUS
        builder.Services.AddHttpClient<CfStatusService>();
#endif
#if FEATURE_EMAIL
        builder.Services.AddSingleton<EmailSearchService>();
#endif

        // ── Phase 5: remaining integrations ─────────────────────────────────
        MiscServicesRegistration.RegisterQueryTypes();
        EntertainmentServicesRegistration.RegisterQueryTypes();
        PackagesServicesRegistration.RegisterQueryTypes();

#if FEATURE_ICP
        builder.Services.AddSingleton(sp => new IcpService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            new SqliteKvStore(Path.Combine(cacheDir, "icp.db"))));
#endif
#if FEATURE_PEN
        builder.Services.AddSingleton(sp => new PenService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            new SqliteKvStore(Path.Combine(cacheDir, "pen.db"))));
        builder.Services.AddHostedService<PenSyncHostedService>();
#endif
#if FEATURE_PIXIV
        builder.Services.AddSingleton(sp => new PixivAuthClient(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            Environment.GetEnvironmentVariable("PIXIV_REFRESH_TOKEN")));
        builder.Services.AddSingleton(sp => new PixivClient(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            sp.GetRequiredService<PixivAuthClient>()));
        builder.Services.AddSingleton<PixivService>();
#endif

#if FEATURE_STEAM
        builder.Services.AddSingleton(sp => new SteamService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            Environment.GetEnvironmentVariable("STEAM_API_KEY")));
#endif
#if FEATURE_IMDB
        builder.Services.AddSingleton(sp => new ImdbService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            Environment.GetEnvironmentVariable("OMDB_API_KEY")));
#endif
#if FEATURE_MINECRAFT
        builder.Services.AddHttpClient<MinecraftService>();
#endif
#if FEATURE_ACGC
        builder.Services.AddHttpClient<AcgcService>();
#endif
#if FEATURE_LYRIC
        builder.Services.AddHttpClient<LyricService>();
#endif
#if FEATURE_MEAL
        builder.Services.AddHttpClient<MealService>();
#endif
#if FEATURE_WIKIPEDIA
        builder.Services.AddHttpClient<WikipediaService>();
#endif
#if FEATURE_GITHUB
        builder.Services.AddHttpClient<GitHubService>();
#endif

#if FEATURE_PKG_ALMA
        builder.Services.AddHttpClient<AlmaPackageService>();
#endif
#if FEATURE_PKG_AOSC
        builder.Services.AddHttpClient<AoscPackageService>();
#endif
#if FEATURE_PKG_AUR
        builder.Services.AddHttpClient<AurPackageService>();
#endif
#if FEATURE_PKG_CARGO
        builder.Services.AddHttpClient<CargoPackageService>();
#endif
#if FEATURE_PKG_CURSEFORGE
        builder.Services.AddHttpClient<CurseForgePackageService>();
#endif
#if FEATURE_PKG_DEBIAN
        builder.Services.AddHttpClient<DebianPackageService>();
#endif
#if FEATURE_PKG_EPEL
        builder.Services.AddHttpClient<EpelPackageService>();
#endif
#if FEATURE_PKG_MODRINTH
        builder.Services.AddHttpClient<ModrinthPackageService>();
#endif
#if FEATURE_PKG_NIXOS
        builder.Services.AddHttpClient<NixOsPackageService>();
#endif
#if FEATURE_PKG_NPM
        builder.Services.AddHttpClient<NpmPackageService>();
#endif
#if FEATURE_PKG_PYPI
        builder.Services.AddHttpClient<PypiPackageService>();
#endif
#if FEATURE_PKG_OPENSUSE
        builder.Services.AddHttpClient<OpenSusePackageService>();
#endif
#if FEATURE_PKG_UBUNTU
        builder.Services.AddHttpClient<UbuntuPackageService>();
#endif
#if FEATURE_PKG_OPENWRT
        builder.Services.AddSingleton(sp => new OpenWrtPackageService(
            sp.GetRequiredService<IHttpClientFactory>().CreateClient(),
            new SqliteKvStore(Path.Combine(cacheDir, "openwrt.db"))));
#endif

#if FEATURE_PLUGINS
        // ── Phase 6: native plugin system ────────────────────────────────────
        // Registered empty here (before Build()); LoadPluginsAsync (called from Program.cs after
        // Build(), same pattern as LoadPatchesAsync) populates it once IHttpClientFactory/
        // ILoggerFactory actually exist. QueryAnalyzer's suffix registry is a mutable static, so
        // plugin suffixes registered at that later point still take effect before any query flows.
        builder.Services.AddSingleton<PluginRegistry>();
        builder.Services.AddSingleton(_ => new SqliteKvStore(Path.Combine(cacheDir, "plugins.db")));
#endif

#if FEATURE_TELEMETRY
        // ── Phase 7: telemetry ───────────────────────────────────────────────
        builder.Services.AddHttpClient<TelemetryService>();
#endif

        builder.Services.AddSingleton<QueryProcessor>();
        builder.Services.AddHostedService<WhoisTcpServer>();
        builder.Services.AddHostedService<StatsFlushService>();

        var app = builder.Build();

#if FEATURE_WEB
        // Scope note: Feature_Web disables the dashboard/API *surface* (static files, API routes,
        // dashboard HTML) — Kestrel itself still binds cliOptions.WebPort regardless, since Phase 4
        // deliberately unified the WHOIS TCP listener and the web dashboard onto one
        // WebApplication/Generic Host rather than keeping them as separable processes the way
        // Rust's Axum router is. A build with Feature_Web=false just 404s every HTTP request
        // instead of not listening at all; genuinely not listening would need reverting that
        // Phase 4 architecture decision, which isn't worth it for this flag alone.
        app.UseStaticFiles();
        app.MapWhoisApi();
#if FEATURE_PIXIV
        app.MapPixivProxy();
#endif
        app.MapOpenApi("/api/openapi.json");

        // The dashboard is a tiny client-side "app.js" that switches view based on
        // location.pathname (see wwwroot/app.js) — every unmatched GET falls back to the same
        // shell, matching the plan.md decision not to pull in a client router just to switch
        // between two views. Goes through WebRootFileProvider (not a raw WebRootPath string,
        // which can be null when the process's content root has no physical wwwroot folder next
        // to it, e.g. under the integration test host) so it resolves correctly via the static
        // web assets manifest either way.
        app.MapFallbackToFile("index.html");
#endif

        return app;
    }

    public static async Task LoadPatchesAsync(IHost host)
    {
        var logger = host.Services.GetRequiredService<ILogger<Program>>();
        var patchManager = host.Services.GetRequiredService<PatchManager>();
        try
        {
            var count = await patchManager.LoadFromStorageAsync();
            logger.LogInformation("Loaded {Count} response patches", count);
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "Continuing without response patches");
        }
    }

    /// <summary>No-op when Feature_Plugins=false (the <see cref="PluginRegistry"/>/plugin-cache
    /// services this needs simply aren't registered in that build), so callers never need their
    /// own <c>#if FEATURE_PLUGINS</c>.</summary>
    public static async Task LoadPluginsAsync(IHost host, string pluginsDir = "plugins")
    {
#if FEATURE_PLUGINS
        var registry = host.Services.GetRequiredService<PluginRegistry>();
        var sharedCache = host.Services.GetRequiredService<SqliteKvStore>();
        var loggerFactory = host.Services.GetRequiredService<ILoggerFactory>();
        await PluginLoader.LoadAllAsync(registry, pluginsDir, sharedCache, loggerFactory);
#else
        await Task.CompletedTask;
#endif
    }
}
