namespace WhoisServer.Services.Bgp;

/// <summary>Server/port table for the 12 direct-connect IRR registries — ported from
/// src/config.rs's per-registry constants. Keys match the registry names used in query suffixes
/// (upper-case, matching <see cref="WhoisServer.Core.Query.QueryType.IrrDirect.RegistryKey"/>).</summary>
public static class IrrRegistryConstants
{
    public static readonly IReadOnlyDictionary<string, (string Server, int Port)> Registries =
        new Dictionary<string, (string, int)>
        {
            ["RADB"] = ("whois.radb.net", 43),
            ["ALTDB"] = ("whois.altdb.net", 43),
            ["AFRINIC"] = ("whois.afrinic.net", 43),
            ["APNIC"] = ("whois.apnic.net", 43),
            ["ARIN"] = ("rr.arin.net", 43),
            ["BELL"] = ("whois.in.bell.ca", 43),
            ["JPIRR"] = ("jpirr.nic.ad.jp", 43),
            ["LACNIC"] = ("irr.lacnic.net", 43),
            ["LEVEL3"] = ("rr.level3.net", 43),
            ["NTTCOM"] = ("rr.ntt.net", 43),
            ["RIPE"] = ("whois.ripe.net", 43),
            ["RIS"] = ("riswhois.ripe.net", 43),
            ["TC"] = ("whois.bgp.net.br", 43),
        };
}
