#if FEATURE_DN42
using System.Net;
using System.Net.Sockets;

namespace WhoisServer.Services.Dn42;

/// <summary>
/// Ported from src/config.rs's PRIVATE_IPV4_RANGES/PRIVATE_IPV6_RANGES — used to decide whether a
/// query's address should be routed to DN42 instead of the public RDAP/WHOIS backends. Uses
/// .NET's native <see cref="IPNetwork"/> for containment checks instead of the Rust version's
/// hand-rolled CIDR-contains logic via the `cidr` crate — a straightforward simplification, not a
/// behavior change (plan.md §5, Phase 3).
/// </summary>
public static class PrivateNetworkRanges
{
    private static readonly IPNetwork[] PrivateIpv4 =
    [
        IPNetwork.Parse("10.0.0.0/8"),      // RFC1918
        IPNetwork.Parse("172.16.0.0/12"),   // RFC1918
        IPNetwork.Parse("192.168.0.0/16"),  // RFC1918
        IPNetwork.Parse("169.254.0.0/16"),  // Link-local
        IPNetwork.Parse("192.0.2.0/24"),    // TEST-NET-1
        IPNetwork.Parse("198.51.100.0/24"), // TEST-NET-2
        IPNetwork.Parse("203.0.113.0/24"),  // TEST-NET-3
        IPNetwork.Parse("100.64.0.0/10"),   // CGNAT
        IPNetwork.Parse("127.0.0.0/8"),     // Loopback
    ];

    private static readonly IPNetwork[] PrivateIpv6 =
    [
        IPNetwork.Parse("fc00::/7"),      // Unique Local Addresses
        IPNetwork.Parse("fd00::/8"),      // ULA subset
        IPNetwork.Parse("fe80::/10"),     // Link-local
        IPNetwork.Parse("::1/128"),       // Loopback
        IPNetwork.Parse("2001:db8::/32"), // Documentation
    ];

    public static bool IsPrivate(IPAddress address) => address.AddressFamily switch
    {
        AddressFamily.InterNetwork => PrivateIpv4.Any(n => n.Contains(address)),
        AddressFamily.InterNetworkV6 => PrivateIpv6.Any(n => n.Contains(address)),
        _ => false,
    };
}
#endif
