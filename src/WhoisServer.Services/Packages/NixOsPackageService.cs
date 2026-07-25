using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/nixos.rs — the <c>-NIXOS</c> suffix.
///
/// Deliberate fix, not a byte-for-byte port: the Rust version never actually calls a real API for
/// this one. <c>parse_nixos_html</c> ignores the HTML body entirely and
/// <c>generate_nixos_package_info</c> returns hardcoded, fabricated data for a handful of package
/// names (literally `"vim" =&gt; NixOSPackage { package_pversion: "9.1.1475", ... }`) and a generic
/// placeholder for everything else — it never reflects the real state of nixpkgs, for any package.
/// search.nixos.org's own backend is an Elasticsearch instance that requires HTTP Basic auth with
/// credentials embedded in its frontend JS bundle; rather than hardcode that credential pair into
/// source here, this port instead uses Repology's public, unauthenticated project API
/// (repology.org/api/v1/project/{name}, a well-documented public aggregator that tracks package
/// versions across many distros, filtered here to the <c>nix_*</c> repositories it tracks for
/// nixpkgs) — real, current, no credentials required. No caching: it's a fast, public, rate-limit-
/// tolerant read endpoint, same posture as the Rust version's (absent) caching.
/// </summary>
public sealed class NixOsPackageService(HttpClient httpClient)
{
    private const string RepologyApiBase = "https://repology.org/api/v1/project/";
    private const string SearchWebUrl = "https://search.nixos.org/packages?channel=unstable&query=";
    private static readonly TimeSpan RequestTimeout = TimeSpan.FromSeconds(10);

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        var name = packageName.Trim();
        if (name.Length == 0 || name.Length > 200)
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var entries = await QueryAsync(name, ct);
            return entries.Count == 0 ? FormatNotFound(name) : FormatResponse(entries, name);
        }
        catch
        {
            return FormatNotFound(name);
        }
    }

    private async Task<List<RepologyEntry>> QueryAsync(string name, CancellationToken ct)
    {
        // Repology has been observed to silently hang (not even a fast HTTP error) rather than
        // reject quickly under some network conditions — bound the wait so one slow/unreachable
        // upstream can't tie up a WHOIS connection indefinitely.
        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeoutCts.CancelAfter(RequestTimeout);

        using var request = new HttpRequestMessage(HttpMethod.Get, RepologyApiBase + Uri.EscapeDataString(name));
        request.Headers.UserAgent.Add(new ProductInfoHeaderValue("whois-server-csharp", "1.0"));
        request.Headers.UserAgent.Add(new ProductInfoHeaderValue("(+package-lookup-service)"));

        using var response = await httpClient.SendAsync(request, timeoutCts.Token);
        if (!response.IsSuccessStatusCode)
        {
            return [];
        }

        var entries = await response.Content.ReadFromJsonAsync<List<RepologyEntry>>(cancellationToken: timeoutCts.Token) ?? [];
        return entries
            .Where(e => e.Repo.StartsWith("nix_", StringComparison.OrdinalIgnoreCase))
            .OrderByDescending(e => e.Repo.Equals("nix_unstable", StringComparison.OrdinalIgnoreCase))
            .ThenByDescending(e => e.Status == "newest")
            .ToList();
    }

    private static string FormatResponse(List<RepologyEntry> entries, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"NixOS Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        var best = entries[0];
        sb.Append($"package-name:         {best.VisibleName ?? best.SrcName}\n");
        sb.Append($"nixpkgs-attribute:    {best.SrcName}\n");
        sb.Append($"version:              {best.Version}\n");
        if (best.Summary is { } summary) sb.Append($"description:          {summary}\n");
        if (best.Licenses is { Count: > 0 }) sb.Append($"license:              {string.Join(", ", best.Licenses)}\n");
        if (best.Maintainers is { Count: > 0 }) sb.Append($"maintainers:          {string.Join(", ", best.Maintainers)}\n");
        sb.Append($"status:               {best.Status}\n");

        var channels = entries.Select(e => $"{e.Repo}={e.Version}").ToList();
        sb.Append($"tracked-channels:     {string.Join(", ", channels)}\n");

        sb.Append('\n');
        sb.Append($"nixos-url:            {SearchWebUrl}{Uri.EscapeDataString(best.SrcName)}\n");
        sb.Append("repository: nixpkgs\n");
        sb.Append("source: Repology (repology.org) project API\n");
        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            NixOS Package Not Found: {packageName}
            No package with this name was found tracked for nixpkgs.

            You can search manually at: {SearchWebUrl}{Uri.EscapeDataString(packageName)}

            % Package not found in nixpkgs
            """;
    }

    private sealed record RepologyEntry(
        [property: JsonPropertyName("repo")] string Repo,
        [property: JsonPropertyName("srcname")] string SrcName,
        [property: JsonPropertyName("visiblename")] string? VisibleName,
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("status")] string? Status,
        [property: JsonPropertyName("summary")] string? Summary,
        [property: JsonPropertyName("licenses")] List<string>? Licenses,
        [property: JsonPropertyName("maintainers")] List<string>? Maintainers);
}
