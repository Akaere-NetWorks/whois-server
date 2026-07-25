#if FEATURE_DN42
using System.Net;
using System.Net.Sockets;
using System.Text;
using WhoisServer.Core.Dn42;
using WhoisServer.Core.Query;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Dn42;

/// <summary>
/// Ported from src/dn42/git_backend.rs's <c>DN42Registry</c> (the query side; see
/// <see cref="Dn42RegistrySync"/> for the git-sync/populate side) — longest-prefix-match IP
/// lookups plus the object-type candidate chain from <see cref="Dn42ObjectLookup"/>, all backed
/// by <see cref="IKvStore"/> instead of LMDB.
///
/// Unlike core/query_processor.rs (which passes the *pre-classified* QueryType's already
/// mask-stripped IPv4/IPv6 address to DN42, losing the original CIDR prefix length), this re-parses
/// the raw query text itself for IP/CIDR shape — matching what git_backend.rs actually does
/// (it takes a raw &amp;str and calls query.split_once('/') itself, independent of core::query's
/// classification). Losing the mask here would silently break every DN42 network-block lookup.
/// </summary>
public sealed class Dn42Registry(IKvStore store) : IDn42QueryRouter
{
    public bool IsDefinitivelyDn42(QueryType queryType) => queryType switch
    {
        QueryType.Domain d => d.Name.EndsWith(".dn42", StringComparison.OrdinalIgnoreCase),
        QueryType.IPv4 ip => PrivateNetworkRanges.IsPrivate(ip.Address),
        QueryType.IPv6 ip => PrivateNetworkRanges.IsPrivate(ip.Address),
        QueryType.Asn asn => asn.Value.ToUpperInvariant().StartsWith("AS42424", StringComparison.Ordinal),
        _ => false,
    };

    public bool IsExplicitDn42Marker(string query)
    {
        var upper = query.ToUpperInvariant();
        return upper.EndsWith("-DN42", StringComparison.Ordinal) || upper.EndsWith("-MNT", StringComparison.Ordinal);
    }

    public async Task<string> ResolveAsync(string query, CancellationToken ct = default)
    {
        var sb = new StringBuilder();
        sb.Append($"% Query: {query}\n");

        if (TryParseIpQuery(query, out var address, out var mask, out var isV6))
        {
            sb.Append(isV6
                ? await ResolveIpv6Async(address!, mask, ct)
                : await ResolveIpv4Async(address!, mask, ct));
            return sb.ToString();
        }

        var content = await ResolveObjectAsync(query, ct);
        sb.Append(content ?? "% 404 Not Found\n");
        return sb.ToString();
    }

    private async Task<string> ResolveIpv4Async(IPAddress ip, int mask, CancellationToken ct)
    {
        var sb = new StringBuilder();

        var inetnumKey = await FindNetworkKeyAsync("inetnum", ip, mask, 32, ct);
        sb.Append(inetnumKey is not null ? await store.GetAsync(inetnumKey, ct) ?? "% 404 - inetnum not found\n" : "% 404 - inetnum not found\n");

        sb.Append("% Relevant route object:\n");

        var routeKey = await FindNetworkKeyAsync("route", ip, mask, 32, ct);
        sb.Append(routeKey is not null ? await store.GetAsync(routeKey, ct) ?? "% 404 - route not found\n" : "% 404 - route not found\n");

        return sb.ToString();
    }

    private async Task<string> ResolveIpv6Async(IPAddress ip, int mask, CancellationToken ct)
    {
        var sb = new StringBuilder();

        var inet6numKey = await FindNetworkKeyAsync("inet6num", ip, mask, 128, ct);
        sb.Append(inet6numKey is not null ? await store.GetAsync(inet6numKey, ct) ?? "% 404 - inet6num not found\n" : "% 404 - inet6num not found\n");

        sb.Append("% Relevant route object:\n");

        var route6Key = await FindNetworkKeyAsync("route6", ip, mask, 128, ct);
        sb.Append(route6Key is not null ? await store.GetAsync(route6Key, ct) ?? "% 404 - route6 not found\n" : "% 404 - route6 not found\n");

        return sb.ToString();
    }

    private async Task<string?> ResolveObjectAsync(string query, CancellationToken ct)
    {
        foreach (var (objectType, key) in Dn42ObjectLookup.GetCandidates(query))
        {
            var content = await store.GetAsync($"{objectType}/{key}", ct);
            if (content is not null)
            {
                return content;
            }
        }

        return null;
    }

    /// <summary>Longest-prefix match: for a query mask /N, tries candidate masks N, N-1, ..., 0
    /// (most specific first) and returns the first one whose "&lt;subdir&gt;/&lt;network&gt;_&lt;mask&gt;"
    /// key actually exists in storage. Matches git_backend.rs's find_ipv4_network/find_ipv6_network
    /// exactly, including the underscore-separated key format (the real DN42 registry's own file
    /// naming convention for network objects).</summary>
    private async Task<string?> FindNetworkKeyAsync(string subdir, IPAddress ip, int queryMask, int totalBits, CancellationToken ct)
    {
        var addressBytes = ip.GetAddressBytes();

        for (var mask = queryMask; mask >= 0; mask--)
        {
            var networkAddress = new IPAddress(ApplyMask(addressBytes, mask));
            var key = $"{subdir}/{networkAddress}_{mask}";
            if (await store.ExistsAsync(key, ct))
            {
                return key;
            }
        }

        return null;
    }

    private static byte[] ApplyMask(byte[] addressBytes, int mask)
    {
        var result = (byte[])addressBytes.Clone();
        var fullBytes = mask / 8;
        var remainingBits = mask % 8;

        if (remainingBits > 0 && fullBytes < result.Length)
        {
            var keepMask = (byte)(0xFF << (8 - remainingBits));
            result[fullBytes] &= keepMask;
            fullBytes++;
        }

        for (var i = fullBytes; i < result.Length; i++)
        {
            result[i] = 0;
        }

        return result;
    }

    private static bool TryParseIpQuery(string query, out IPAddress? address, out int mask, out bool isV6)
    {
        address = null;
        mask = 0;
        isV6 = false;

        var slashIndex = query.IndexOf('/');
        if (slashIndex >= 0)
        {
            var ipPart = query[..slashIndex];
            var maskPart = query[(slashIndex + 1)..];

            if (!IsValidIpText(ipPart, out var ip) || !int.TryParse(maskPart, out var parsedMask))
            {
                return false;
            }

            var maxMask = ip!.AddressFamily == AddressFamily.InterNetworkV6 ? 128 : 32;
            if (parsedMask < 0 || parsedMask > maxMask)
            {
                return false;
            }

            address = ip;
            mask = parsedMask;
            isV6 = ip.AddressFamily == AddressFamily.InterNetworkV6;
            return true;
        }

        if (IsValidIpText(query, out var single))
        {
            address = single;
            isV6 = single!.AddressFamily == AddressFamily.InterNetworkV6;
            mask = isV6 ? 128 : 32;
            return true;
        }

        return false;
    }

    /// <summary>Same legacy-inet_aton guard as WhoisServer.Core.Query.StrictIpText — without it,
    /// a bare-digit DN42 ASN like "4242420000" would misparse as an IPv4 address here too.</summary>
    private static bool IsValidIpText(string text, out IPAddress? address)
    {
        if (!IPAddress.TryParse(text, out address))
        {
            return false;
        }

        if (address.AddressFamily == AddressFamily.InterNetworkV6)
        {
            return true;
        }

        if (StrictIpText.IsStrictIpv4(text))
        {
            return true;
        }

        address = null;
        return false;
    }
}
#endif
