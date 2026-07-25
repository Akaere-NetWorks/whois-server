using System.Net;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Shared by <see cref="PingService"/> and <see cref="TracerouteService"/> — ported from
/// the (identically duplicated in Rust) <c>parse_location_query</c> in services/ping.rs and
/// services/traceroute.rs, collapsed into one place here. Parses an optional trailing location
/// code: "target" or "target-location" (e.g. "1.1.1.1-tw" -> target=1.1.1.1, location=tw).</summary>
public static class LocationQueryParser
{
    public static (string Target, string? Location) Parse(string query)
    {
        var lastDash = query.LastIndexOf('-');
        if (lastDash < 0)
        {
            return (query, null);
        }

        var potentialLocation = query[(lastDash + 1)..];
        var potentialTarget = query[..lastDash];

        var isValidTarget = potentialTarget.Contains('.') || IPAddress.TryParse(potentialTarget, out _);

        if (isValidTarget && potentialLocation.Length is > 0 and <= 5 && !potentialLocation.Contains('.'))
        {
            return (potentialTarget, potentialLocation);
        }

        return (query, null);
    }
}
