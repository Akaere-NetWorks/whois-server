using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Query;

namespace WhoisServer.Services.Rdap;

/// <summary>
/// Minimal RDAP (RFC 7480-7484) client. There is no mature general-purpose .NET RDAP client
/// library (confirmed during research — see plan.md §2 item 10), so this talks directly to
/// <c>rdap.org</c>'s public bootstrap redirector (it 302s a domain/ip/autnum lookup to whichever
/// registry's RDAP server is actually authoritative), rather than reimplementing IANA's RDAP
/// bootstrap-registry JSON parsing from scratch. The Rust version instead used the (Rust-only)
/// <c>rdap</c> crate's <c>format_whois</c> for its output shape; this formatter is a new,
/// deliberately simpler implementation covering the fields real registries actually return, not a
/// line-for-line match — consistent with the "not byte-for-byte" decision.
/// </summary>
public sealed class RdapClient(HttpClient httpClient, ILogger<RdapClient> logger)
{
    /// <summary>Read off the assembly rather than hardcoded, so it tracks
    /// <c>VersionPrefix</c> in Directory.Build.props instead of silently going stale in the
    /// outbound User-Agent below.</summary>
    private static readonly string UserAgentVersion =
        typeof(RdapClient).Assembly.GetName().Version?.ToString(3) ?? "0.0.0";

    /// <summary>Returns null if <paramref name="queryType"/> has no RDAP mapping.</summary>
    public static string? BuildRequestPath(QueryType queryType) => queryType switch
    {
        QueryType.Domain d => $"domain/{d.Name}",
        QueryType.IPv4 ip => $"ip/{ip.Address}",
        QueryType.IPv6 ip => $"ip/{ip.Address}",
        QueryType.Asn asn => $"autnum/{StripAsPrefix(asn.Value)}",
        QueryType.BareAsn asn => $"autnum/{asn.Value}",
        _ => null,
    };

    private static string StripAsPrefix(string value) =>
        value.StartsWith("AS", StringComparison.OrdinalIgnoreCase) ? value[2..] : value;

    public async Task<string> QueryAsync(string query, QueryType queryType, CancellationToken ct = default)
    {
        var path = BuildRequestPath(queryType)
            ?? throw new NotSupportedException($"No RDAP mapping for query type {queryType.GetType().Name}");

        logger.LogDebug("Querying RDAP via rdap.org/{Path}", path);

        using var request = new HttpRequestMessage(HttpMethod.Get, $"https://rdap.org/{path}");
        request.Headers.TryAddWithoutValidation("Accept", "application/rdap+json");
        // Some registries (observed: RIPE's rdap.db.ripe.net, reached via rdap.org's redirect)
        // 403 requests with no User-Agent at all — a real, empirically-found requirement, not
        // a guess (default HttpClient requests send none).
        request.Headers.TryAddWithoutValidation("User-Agent", $"whois-server/{UserAgentVersion} (+https://github.com/Akaere-NetWorks/whois-server)");

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException($"RDAP query failed with HTTP {(int)response.StatusCode}");
        }

        await using var stream = await response.Content.ReadAsStreamAsync(ct);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: ct);

        var whoisText = FormatAsWhoisText(document.RootElement);
        if (string.IsNullOrWhiteSpace(whoisText))
        {
            throw new InvalidOperationException("RDAP returned an empty response");
        }

        return $"% RDAP (Registration Data Access Protocol) Response\n% Query: {query}\n%\n{whoisText}\n% End of RDAP response\n";
    }

    private static string FormatAsWhoisText(JsonElement root)
    {
        var sb = new StringBuilder();

        AppendIfPresent(sb, "domain", root, "ldhName");
        AppendIfPresent(sb, "handle", root, "handle");
        AppendIfPresent(sb, "unicode-name", root, "unicodeName");
        AppendIfPresent(sb, "name", root, "name");
        AppendIfPresent(sb, "country", root, "country");
        AppendIfPresent(sb, "type", root, "type");

        if (root.TryGetProperty("status", out var status) && status.ValueKind == JsonValueKind.Array)
        {
            foreach (var s in status.EnumerateArray())
            {
                sb.AppendLine($"status:         {s.GetString()}");
            }
        }

        if (root.TryGetProperty("nameservers", out var nameservers) && nameservers.ValueKind == JsonValueKind.Array)
        {
            foreach (var ns in nameservers.EnumerateArray())
            {
                if (ns.TryGetProperty("ldhName", out var ldh))
                {
                    sb.AppendLine($"nserver:        {ldh.GetString()}");
                }
            }
        }

        if (root.TryGetProperty("entities", out var entities) && entities.ValueKind == JsonValueKind.Array)
        {
            foreach (var entity in entities.EnumerateArray())
            {
                var roles = entity.TryGetProperty("roles", out var rolesEl) && rolesEl.ValueKind == JsonValueKind.Array
                    ? string.Join(",", rolesEl.EnumerateArray().Select(r => r.GetString()))
                    : "entity";
                var name = ExtractVCardName(entity) ?? (entity.TryGetProperty("handle", out var h) ? h.GetString() : null);
                if (name is not null)
                {
                    sb.AppendLine($"{roles}:         {name}");
                }
            }
        }

        if (root.TryGetProperty("events", out var events) && events.ValueKind == JsonValueKind.Array)
        {
            foreach (var ev in events.EnumerateArray())
            {
                var action = ev.TryGetProperty("eventAction", out var a) ? a.GetString() : null;
                var date = ev.TryGetProperty("eventDate", out var d) ? d.GetString() : null;
                if (action is not null && date is not null)
                {
                    sb.AppendLine($"{action}:         {date}");
                }
            }
        }

        return sb.ToString().TrimEnd('\n');
    }

    private static string? ExtractVCardName(JsonElement entity)
    {
        if (!entity.TryGetProperty("vcardArray", out var vcard) || vcard.ValueKind != JsonValueKind.Array || vcard.GetArrayLength() < 2)
        {
            return null;
        }

        foreach (var field in vcard[1].EnumerateArray())
        {
            if (field.ValueKind == JsonValueKind.Array && field.GetArrayLength() >= 4
                && field[0].GetString() == "fn")
            {
                return field[3].GetString();
            }
        }

        return null;
    }

    private static void AppendIfPresent(StringBuilder sb, string label, JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var value) && value.ValueKind == JsonValueKind.String)
        {
            sb.AppendLine($"{label}:         {value.GetString()}");
        }
    }
}
