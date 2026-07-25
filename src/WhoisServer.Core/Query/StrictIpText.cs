using System.Text.RegularExpressions;

namespace WhoisServer.Core.Query;

/// <summary>
/// .NET's <see cref="System.Net.IPAddress.TryParse(string, out System.Net.IPAddress)"/> accepts
/// legacy inet_aton-style forms ("213605", "192.168.1", etc. interpreted as a packed 32-bit
/// integer) that a real dotted-quad parser rejects outright. Without this guard, a bare ASN like
/// "213605" silently misparses as an IPv4 address — first caught in
/// <see cref="QueryAnalyzer"/> (see QueryAnalyzerTests.ClassifiesBareAsn) and reused here since
/// the DN42 registry lookup (Phase 3) parses IP-shaped query text independently and needs the
/// exact same guard.
/// </summary>
public static class StrictIpText
{
    private static readonly Regex StrictIpv4Regex = new(
        @"^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$", RegexOptions.Compiled);

    public static bool IsStrictIpv4(string candidate)
    {
        var match = StrictIpv4Regex.Match(candidate);
        if (!match.Success)
        {
            return false;
        }

        for (var i = 1; i <= 4; i++)
        {
            var octet = match.Groups[i].Value;
            // Reject ambiguous leading zeros (matches modern Rust's std parser behavior).
            if (octet.Length > 1 && octet[0] == '0')
            {
                return false;
            }

            if (!byte.TryParse(octet, out _))
            {
                return false;
            }
        }

        return true;
    }
}
