namespace WhoisServer.Core.Color;

/// <summary>
/// Header-based color-negotiation protocol layered on top of raw WHOIS (not part of RFC 3912) —
/// a client can send an <c>X-WHOIS-COLOR-PROBE:</c> or <c>X-WHOIS-COLOR:</c> header line before
/// (or interleaved with) its actual query line.
/// </summary>
public sealed class ColorProtocol
{
    public bool Enabled { get; set; } = true;
    public ColorScheme? Scheme { get; set; }
    public bool ClientSupportsColor { get; private set; }

    /// <summary>Returns true if this request is a capability probe (caller should short-circuit
    /// and respond with <see cref="GetCapabilityResponse"/> without processing any query).</summary>
    public bool ParseHeaders(string request)
    {
        foreach (var rawLine in request.Split('\n'))
        {
            var line = rawLine.Trim().TrimEnd('\r');

            if (line.StartsWith("X-WHOIS-COLOR-PROBE:", StringComparison.OrdinalIgnoreCase))
            {
                ClientSupportsColor = true;
                return true;
            }

            if (line.StartsWith("X-WHOIS-COLOR:", StringComparison.OrdinalIgnoreCase))
            {
                var parts = line.Split(':', 2);
                if (parts.Length < 2)
                {
                    continue;
                }

                var valuePart = parts[1].Trim();
                var schemeStr = valuePart.StartsWith("scheme=", StringComparison.OrdinalIgnoreCase)
                    ? valuePart["scheme=".Length..]
                    : valuePart;

                var scheme = ColorSchemeExtensions.FromString(schemeStr);
                if (scheme is not null)
                {
                    Scheme = scheme;
                    ClientSupportsColor = true;
                }
            }
        }

        return false;
    }

    public bool ShouldColorize() => Enabled && ClientSupportsColor && Scheme is not null;

    public string GetCapabilityResponse() =>
        Enabled
            ? "X-WHOIS-COLOR-SUPPORT: 1.0 schemes=ripe,ripe-dark,bgptools,bgptools-dark\r\n\r\n"
            : "X-WHOIS-COLOR-SUPPORT: no\r\n\r\n";
}
