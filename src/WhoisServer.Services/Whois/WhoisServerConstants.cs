namespace WhoisServer.Services.Whois;

/// <summary>Ported from src/config.rs — the small subset of WHOIS server constants Phase 1 needs.
/// Later phases add the IRR-registry constants (RADB, AFRINIC, APNIC, ...) alongside the services
/// that use them, rather than front-loading the whole list here.</summary>
public static class WhoisServerConstants
{
    public const string DefaultWhoisServer = "whois.ripe.net";
    public const int DefaultWhoisPort = 43;
    public const string RipeWhoisServer = "whois.ripe.net";
    public const int RipeWhoisPort = 43;
    public const string RadbWhoisServer = "whois.radb.net";
    public const int RadbWhoisPort = 43;
    public const string IanaWhoisServer = "whois.iana.org";
    public const int IanaWhoisPort = 43;

    public const string ServerBanner = "% Akaere NetWorks Whois Server";

    public static readonly TimeSpan DefaultTimeout = TimeSpan.FromSeconds(10);
}
