using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Bgp;

/// <summary>Ported from src/services/rpki.rs — the <c>prefix-asn-RPKI</c> suffix, validating a
/// route announcement against RPKI ROAs via a self-hosted validity API.</summary>
public sealed class RpkiValidationService(HttpClient httpClient)
{
    private const string ApiBase = "https://rpki.akae.re/api/v1/validity";

    public async Task<string> ProcessAsync(string prefix, string asn, CancellationToken ct = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"{ApiBase}/{asn}/{prefix}");
            request.Headers.TryAddWithoutValidation("User-Agent", "akaere-whois-server/1.0");

            using var response = await httpClient.SendAsync(request, ct);
            response.EnsureSuccessStatusCode();

            var result = await response.Content.ReadFromJsonAsync<RpkiResponse>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty RPKI validity response");

            return Format(prefix, asn, result);
        }
        catch (Exception e)
        {
            return $"% RPKI Validation Query Failed\n% Query: {prefix}-{asn}-RPKI\n% Error: {e.Message}\n";
        }
    }

    private static string Format(string prefix, string asn, RpkiResponse response)
    {
        var sb = new StringBuilder();
        sb.Append("% RPKI Validation Query\n% Data from rpki.akae.re\n");
        sb.Append($"% Query: {prefix}-{asn}-RPKI\n");
        sb.Append($"% Generated Time: {response.GeneratedTime}\n\n");

        sb.Append("route:\n");
        sb.Append($"  origin-asn:     {response.ValidatedRoute.Route.OriginAsn}\n");
        sb.Append($"  prefix:         {response.ValidatedRoute.Route.Prefix}\n\n");

        sb.Append("validity:\n");
        sb.Append($"  state:          {response.ValidatedRoute.Validity.State}\n");
        sb.Append($"  description:    {response.ValidatedRoute.Validity.Description}\n");
        if (response.ValidatedRoute.Validity.Reason is { } reason)
        {
            sb.Append($"  reason:         {reason}\n");
        }

        sb.Append('\n');

        sb.Append("vrps:\n");
        AppendVrpGroup(sb, "matched", response.ValidatedRoute.Validity.Vrps.Matched);
        AppendVrpGroup(sb, "unmatched-as", response.ValidatedRoute.Validity.Vrps.UnmatchedAs);
        AppendVrpGroup(sb, "unmatched-length", response.ValidatedRoute.Validity.Vrps.UnmatchedLength);

        sb.Append("\n% End of RPKI validation result\n");
        return sb.ToString();
    }

    private static void AppendVrpGroup(StringBuilder sb, string label, List<Vrp> vrps)
    {
        if (vrps.Count == 0)
        {
            sb.Append($"  {label}:        none\n");
            return;
        }

        sb.Append($"  {label}:\n");
        foreach (var vrp in vrps)
        {
            sb.Append($"    asn:          {vrp.Asn}\n");
            sb.Append($"    prefix:       {vrp.Prefix}\n");
            sb.Append($"    max-length:   {vrp.MaxLength}\n\n");
        }
    }

    private sealed record RpkiResponse(
        [property: JsonPropertyName("validated_route")] ValidatedRoute ValidatedRoute,
        [property: JsonPropertyName("generatedTime")] string GeneratedTime);

    private sealed record ValidatedRoute(
        [property: JsonPropertyName("route")] RouteInfo Route,
        [property: JsonPropertyName("validity")] ValidityInfo Validity);

    private sealed record RouteInfo(
        [property: JsonPropertyName("origin_asn")] string OriginAsn,
        [property: JsonPropertyName("prefix")] string Prefix);

    private sealed record ValidityInfo(
        [property: JsonPropertyName("state")] string State,
        [property: JsonPropertyName("description")] string Description,
        [property: JsonPropertyName("VRPs")] VrpsInfo Vrps,
        [property: JsonPropertyName("reason")] string? Reason);

    private sealed record VrpsInfo(
        [property: JsonPropertyName("matched")] List<Vrp> Matched,
        [property: JsonPropertyName("unmatched_as")] List<Vrp> UnmatchedAs,
        [property: JsonPropertyName("unmatched_length")] List<Vrp> UnmatchedLength);

    private sealed record Vrp(
        [property: JsonPropertyName("asn")] string Asn,
        [property: JsonPropertyName("prefix")] string Prefix,
        [property: JsonPropertyName("max_length")] string MaxLength);
}
