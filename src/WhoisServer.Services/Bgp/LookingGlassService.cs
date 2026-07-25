using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Bgp;

/// <summary>Ported from src/services/looking_glass.rs — the <c>-LG</c> suffix, RIPE STAT's
/// Looking Glass API reformatted as BIRD routing-daemon-style configuration blocks.</summary>
public sealed class LookingGlassService(HttpClient httpClient)
{
    public async Task<string> ProcessAsync(string resource, CancellationToken ct = default)
    {
        try
        {
            var response = await httpClient.GetAsync(
                $"https://stat.ripe.net/data/looking-glass/data.json?resource={Uri.EscapeDataString(resource)}", ct);
            response.EnsureSuccessStatusCode();

            var lg = await response.Content.ReadFromJsonAsync<LookingGlassResponse>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty Looking Glass response");

            if (lg.DataCallStatus != "supported")
            {
                throw new InvalidOperationException("Looking Glass data call not supported");
            }

            return FormatBirdOutput(lg.Data, resource);
        }
        catch (Exception e)
        {
            return $"% Looking Glass Query Failed\n% Resource: {resource}\n% Error: {e.Message}\n";
        }
    }

    private static string FormatBirdOutput(LookingGlassData data, string resource)
    {
        var sb = new StringBuilder();
        sb.Append($"% RIPE STAT Looking Glass data for {resource}\n");
        sb.Append("% Data from RIPE NCC Route Information Service (RIS)\n");
        sb.Append("% Output in BIRD routing daemon style\n\n");

        if (data.Rrcs.Count == 0)
        {
            sb.Append("% No routing data found\n");
            return sb.ToString();
        }

        var routesByPrefix = new Dictionary<string, List<PeerData>>();
        foreach (var rrc in data.Rrcs)
        {
            foreach (var peer in rrc.Peers)
            {
                if (!routesByPrefix.TryGetValue(peer.Prefix, out var list))
                {
                    routesByPrefix[peer.Prefix] = list = [];
                }

                list.Add(peer);
            }
        }

        foreach (var (prefix, peers) in routesByPrefix)
        {
            sb.Append($"# Routes for prefix {prefix}\n");

            foreach (var peer in peers)
            {
                sb.Append($"route {prefix} via {peer.NextHop} {{\n");
                sb.Append($"    # Peer: {peer.Peer} (AS{peer.AsnOrigin})\n");
                sb.Append($"    # AS-Path: {peer.AsPath}\n");
                sb.Append($"    # Origin: {peer.Origin}\n");

                if (!string.IsNullOrEmpty(peer.Community))
                {
                    sb.Append($"    # Communities: {peer.Community}\n");
                }

                if (!string.IsNullOrEmpty(peer.LargeCommunity))
                {
                    sb.Append($"    # Large Communities: {peer.LargeCommunity}\n");
                }

                if (!string.IsNullOrEmpty(peer.ExtendedCommunity))
                {
                    sb.Append($"    # Extended Communities: {peer.ExtendedCommunity}\n");
                }

                sb.Append($"    # Last Updated: {peer.LastUpdated}\n");
                sb.Append($"    # Latest Time: {peer.LatestTime}\n");

                sb.Append($"    bgp_path.len = {peer.AsPath.Split(' ', StringSplitOptions.RemoveEmptyEntries).Length};\n");
                sb.Append($"    bgp_origin = {peer.Origin.ToUpperInvariant()};\n");
                sb.Append($"    bgp_next_hop = {peer.NextHop};\n");

                if (!string.IsNullOrEmpty(peer.Community))
                {
                    foreach (var community in peer.Community.Split(' ', StringSplitOptions.RemoveEmptyEntries))
                    {
                        var parts = community.Split(':');
                        if (parts.Length >= 2)
                        {
                            sb.Append($"    bgp_community.add(({parts[0]},{parts[1]}));\n");
                        }
                    }
                }

                sb.Append("}\n\n");
            }
        }

        var totalRoutes = data.Rrcs.Sum(r => r.Peers.Count);
        sb.Append($"# Summary: {totalRoutes} routes from {data.Rrcs.Count} RRC collectors\n");
        sb.Append("# RRC Locations:\n");
        foreach (var rrc in data.Rrcs)
        {
            sb.Append($"#   {rrc.Rrc}: {rrc.Location} ({rrc.Peers.Count} peers)\n");
        }

        return sb.ToString();
    }

    private sealed record LookingGlassResponse(
        [property: JsonPropertyName("data")] LookingGlassData Data,
        [property: JsonPropertyName("data_call_status")] string DataCallStatus);

    private sealed record LookingGlassData([property: JsonPropertyName("rrcs")] List<RrcData> Rrcs);

    private sealed record RrcData(
        [property: JsonPropertyName("rrc")] string Rrc,
        [property: JsonPropertyName("location")] string Location,
        [property: JsonPropertyName("peers")] List<PeerData> Peers);

    private sealed record PeerData(
        [property: JsonPropertyName("asn_origin")] string AsnOrigin,
        [property: JsonPropertyName("as_path")] string AsPath,
        [property: JsonPropertyName("community")] string Community,
        [property: JsonPropertyName("largeCommunity")] string LargeCommunity,
        [property: JsonPropertyName("extendedCommunity")] string ExtendedCommunity,
        [property: JsonPropertyName("last_updated")] string LastUpdated,
        [property: JsonPropertyName("prefix")] string Prefix,
        [property: JsonPropertyName("peer")] string Peer,
        [property: JsonPropertyName("origin")] string Origin,
        [property: JsonPropertyName("next_hop")] string NextHop,
        [property: JsonPropertyName("latest_time")] string LatestTime);
}
