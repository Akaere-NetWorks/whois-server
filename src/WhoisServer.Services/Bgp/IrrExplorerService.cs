using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Bgp;

/// <summary>Ported from src/services/irr.rs — the <c>-IRR</c> suffix, querying
/// irrexplorer.nlnog.net's REST API for comprehensive IRR/RPKI route validity across registries.</summary>
public sealed class IrrExplorerService(HttpClient httpClient)
{
    public async Task<string> ProcessAsync(string resource, CancellationToken ct = default)
    {
        try
        {
            using var request = new HttpRequestMessage(
                HttpMethod.Get, $"https://irrexplorer.nlnog.net/api/prefixes/prefix/{Uri.EscapeDataString(resource)}");
            request.Headers.TryAddWithoutValidation("User-Agent", "akaere-whois-server/1.0");

            using var response = await httpClient.SendAsync(request, ct);
            response.EnsureSuccessStatusCode();

            var entries = await response.Content.ReadFromJsonAsync<List<IrrEntry>>(cancellationToken: ct) ?? [];
            return Format(resource, entries);
        }
        catch (Exception e)
        {
            return $"% IRR Explorer Query Failed\n% Query: {resource}\n% Error: {e.Message}\n";
        }
    }

    private static string Format(string resource, List<IrrEntry> entries)
    {
        var sb = new StringBuilder();
        sb.Append("% IRR Explorer Query\r\n% Data from https://irrexplorer.nlnog.net/\r\n");
        sb.Append($"% Query: {resource}\r\n\r\n");

        if (entries.Count == 0)
        {
            sb.Append("% No IRR data available\r\n");
            return sb.ToString();
        }

        for (var i = 0; i < entries.Count; i++)
        {
            if (i > 0)
            {
                sb.Append("\r\n% --- Next Entry ---\r\n\r\n");
            }

            var entry = entries[i];
            sb.Append($"% Prefix: {entry.Prefix}\r\n");
            if (entry.Rir is not null)
            {
                sb.Append($"% RIR: {entry.Rir}\r\n");
            }

            sb.Append($"% Overall Category: {entry.CategoryOverall}\r\n");
            if (entry.GoodnessOverall is not null)
            {
                sb.Append($"% Goodness Score: {entry.GoodnessOverall}\r\n");
            }

            if (entry.BgpOrigins is { Count: > 0 })
            {
                sb.Append($"% BGP Origins: {string.Join(", ", entry.BgpOrigins.Select(o => $"AS{o}"))}\r\n");
            }

            sb.Append("\r\n");

            if (entry.Messages is { Count: > 0 })
            {
                sb.Append("% Messages:\r\n");
                foreach (var message in entry.Messages)
                {
                    sb.Append($"% [{message.Category.ToUpperInvariant(),7}] {message.Text}\r\n");
                }

                sb.Append("\r\n");
            }

            AppendRoutes(sb, "RPKI Routes", entry.RpkiRoutes);

            if (entry.IrrRoutes is not null)
            {
                foreach (var (name, routes) in entry.IrrRoutes.AllDatabases())
                {
                    if (routes is { Count: > 0 })
                    {
                        sb.Append($"% IRR Database: {name}\r\n");
                        AppendRouteDetails(sb, routes);
                    }
                }
            }
        }

        return sb.ToString();
    }

    private static void AppendRoutes(StringBuilder sb, string label, List<IrrRoute>? routes)
    {
        if (routes is not { Count: > 0 })
        {
            return;
        }

        sb.Append($"% {label}:\r\n");
        AppendRouteDetails(sb, routes);
    }

    private static void AppendRouteDetails(StringBuilder sb, List<IrrRoute> routes)
    {
        foreach (var route in routes)
        {
            sb.Append($"% RPKI Status: {route.RpkiStatus}\r\n");
            sb.Append($"% RPSL Primary Key: {route.RpslPk}\r\n");
            if (route.RpkiMaxLength is not null)
            {
                sb.Append($"% Max Length: {route.RpkiMaxLength}\r\n");
            }

            sb.Append("\r\n");
            var text = route.RpslText.Replace("\n", "\r\n");
            sb.Append(text);
            if (!text.EndsWith("\r\n", StringComparison.Ordinal))
            {
                sb.Append("\r\n");
            }

            sb.Append("\r\n");
        }
    }

    private sealed record IrrEntry(
        [property: JsonPropertyName("rpkiRoutes")] List<IrrRoute>? RpkiRoutes,
        [property: JsonPropertyName("categoryOverall")] string CategoryOverall,
        [property: JsonPropertyName("irrRoutes")] IrrRoutesByDatabase? IrrRoutes,
        [property: JsonPropertyName("messages")] List<IrrMessage>? Messages,
        [property: JsonPropertyName("bgpOrigins")] List<uint>? BgpOrigins,
        [property: JsonPropertyName("prefix")] string Prefix,
        [property: JsonPropertyName("rir")] string? Rir,
        [property: JsonPropertyName("goodnessOverall")] int? GoodnessOverall);

    private sealed record IrrRoutesByDatabase(
        [property: JsonPropertyName("RIPE")] List<IrrRoute>? Ripe,
        [property: JsonPropertyName("RADB")] List<IrrRoute>? Radb,
        [property: JsonPropertyName("ARIN")] List<IrrRoute>? Arin,
        [property: JsonPropertyName("APNIC")] List<IrrRoute>? Apnic,
        [property: JsonPropertyName("AFRINIC")] List<IrrRoute>? Afrinic,
        [property: JsonPropertyName("LACNIC")] List<IrrRoute>? Lacnic,
        [property: JsonPropertyName("LEVEL3")] List<IrrRoute>? Level3,
        [property: JsonPropertyName("ALTDB")] List<IrrRoute>? Altdb,
        [property: JsonPropertyName("BELL")] List<IrrRoute>? Bell,
        [property: JsonPropertyName("JPIRR")] List<IrrRoute>? Jpirr,
        [property: JsonPropertyName("NTTCOM")] List<IrrRoute>? Nttcom,
        [property: JsonPropertyName("RPKI")] List<IrrRoute>? Rpki)
    {
        public IEnumerable<(string Name, List<IrrRoute>? Routes)> AllDatabases() =>
        [
            ("RIPE", Ripe), ("RADB", Radb), ("ARIN", Arin), ("APNIC", Apnic), ("AFRINIC", Afrinic),
            ("LACNIC", Lacnic), ("LEVEL3", Level3), ("ALTDB", Altdb), ("BELL", Bell),
            ("JPIRR", Jpirr), ("NTTCOM", Nttcom), ("RPKI", Rpki),
        ];
    }

    private sealed record IrrRoute(
        [property: JsonPropertyName("rpkiStatus")] string RpkiStatus,
        [property: JsonPropertyName("rpslPk")] string RpslPk,
        [property: JsonPropertyName("rpslText")] string RpslText,
        [property: JsonPropertyName("asn")] uint Asn,
        [property: JsonPropertyName("rpkiMaxLength")] uint? RpkiMaxLength);

    private sealed record IrrMessage(
        [property: JsonPropertyName("category")] string Category,
        [property: JsonPropertyName("text")] string Text);
}
