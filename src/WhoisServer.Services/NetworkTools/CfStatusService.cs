using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/cfstatus.rs — the <c>-CFSTATUS</c> suffix: overall
/// status / component list / unresolved incidents from Cloudflare's public status page API.</summary>
public sealed class CfStatusService(HttpClient httpClient)
{
    private const string ApiBase = "https://www.cloudflarestatus.com/api/v2";

    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        var normalized = baseQuery.Trim().ToUpperInvariant();
        try
        {
            return normalized switch
            {
                "COMPONENTS" => await QueryComponentsAsync(ct),
                "INCIDENTS" => await QueryIncidentsAsync(ct),
                _ => await QueryStatusAsync(ct),
            };
        }
        catch (Exception e)
        {
            return $"% Cloudflare Status Query Failed\n% Error: {e.Message}\n";
        }
    }

    private async Task<string> QueryStatusAsync(CancellationToken ct)
    {
        using var response = await httpClient.GetAsync($"{ApiBase}/status.json", ct);
        response.EnsureSuccessStatusCode();
        var status = await response.Content.ReadFromJsonAsync<StatusResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty Cloudflare status response");

        var indicatorSymbol = status.Status.Indicator switch
        {
            "none" => "✓",
            "minor" or "major" => "⚠",
            "critical" => "✗",
            _ => "?",
        };

        return $"""
            % Cloudflare Status - {status.Page.Name}
            % Last Updated: {status.Page.UpdatedAt}
            % URL: {status.Page.Url}
            %
            % Status: {indicatorSymbol} {status.Status.Description}
            % Indicator: {status.Status.Indicator}
            %
            % Query 'components-CFSTATUS' for component details
            % Query 'incidents-CFSTATUS' for unresolved incidents

            """;
    }

    private async Task<string> QueryComponentsAsync(CancellationToken ct)
    {
        using var response = await httpClient.GetAsync($"{ApiBase}/components.json", ct);
        response.EnsureSuccessStatusCode();
        var result = await response.Content.ReadFromJsonAsync<ComponentsResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty Cloudflare components response");

        var sb = new StringBuilder();
        sb.Append($"% Cloudflare Components - {result.Page.Name}\n% Last Updated: {result.Page.UpdatedAt}\n%\n");

        if (result.Components.Count == 0)
        {
            sb.Append("% No components found\n");
            return sb.ToString();
        }

        foreach (var component in result.Components.OrderBy(c => c.Position))
        {
            var symbol = component.Status switch
            {
                "operational" => "✓",
                "degraded_performance" or "partial_outage" => "⚠",
                "major_outage" => "✗",
                _ => "?",
            };

            sb.Append($"% {symbol} {component.Name} ({component.Status})\n");
        }

        return sb.ToString();
    }

    private async Task<string> QueryIncidentsAsync(CancellationToken ct)
    {
        using var response = await httpClient.GetAsync($"{ApiBase}/incidents/unresolved.json", ct);
        response.EnsureSuccessStatusCode();
        var result = await response.Content.ReadFromJsonAsync<IncidentsResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty Cloudflare incidents response");

        var sb = new StringBuilder();
        sb.Append($"% Cloudflare Incidents - {result.Page.Name}\n% Last Updated: {result.Page.UpdatedAt}\n%\n");

        if (result.Incidents.Count == 0)
        {
            sb.Append("% No unresolved incidents\n% All systems operational\n");
            return sb.ToString();
        }

        foreach (var incident in result.Incidents)
        {
            sb.Append($"% {incident.Name} [{incident.Impact.ToUpperInvariant()}]\n");
            sb.Append($"%   Status: {incident.Status}\n");
            sb.Append($"%   Updated: {incident.UpdatedAt}\n");
            sb.Append($"%   Short Link: {incident.Shortlink}\n%\n");
        }

        return sb.ToString();
    }

    private sealed record PageInfo(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("url")] string Url,
        [property: JsonPropertyName("updated_at")] string UpdatedAt);

    private sealed record StatusInfo(
        [property: JsonPropertyName("description")] string Description,
        [property: JsonPropertyName("indicator")] string Indicator);

    private sealed record StatusResponse(
        [property: JsonPropertyName("page")] PageInfo Page,
        [property: JsonPropertyName("status")] StatusInfo Status);

    private sealed record Component(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("status")] string Status,
        [property: JsonPropertyName("position")] int Position);

    private sealed record ComponentsResponse(
        [property: JsonPropertyName("page")] PageInfo Page,
        [property: JsonPropertyName("components")] List<Component> Components);

    private sealed record Incident(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("status")] string Status,
        [property: JsonPropertyName("impact")] string Impact,
        [property: JsonPropertyName("updated_at")] string UpdatedAt,
        [property: JsonPropertyName("shortlink")] string Shortlink);

    private sealed record IncidentsResponse(
        [property: JsonPropertyName("page")] PageInfo Page,
        [property: JsonPropertyName("incidents")] List<Incident> Incidents);
}
