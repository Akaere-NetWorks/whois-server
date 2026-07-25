using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/ubuntu.rs — the <c>-UBUNTU</c> suffix. Queries Launchpad's
/// real <c>getPublishedBinaries</c> API for the Ubuntu primary archive. Bug fix versus the Rust
/// version: the Rust query omits <c>exact_match=true</c>, so a query for "vim" also matches
/// unrelated packages whose name merely *contains* "vim" (e.g. "biosyntax-vim") — confirmed live
/// against the real API during this port. Added <c>exact_match=true&amp;status=Published</c> here.
/// No caching: Launchpad's API is fast and free, same as the Rust version (which never cached
/// this either).
/// </summary>
public sealed class UbuntuPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://api.launchpad.net/1.0/ubuntu/+archive/primary";
    private const string SearchUrl = "https://packages.ubuntu.com";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidPackageName(packageName))
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var packages = await QueryAsync(packageName, ct);
            return packages.Count == 0 ? FormatNotFound(packageName) : FormatResponse(packages, packageName);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    /// <summary>Ported verbatim from the Rust version: Debian-style naming — max 100 chars, ASCII
    /// alphanumeric plus <c>+-._</c>, no spaces.</summary>
    private static bool IsValidPackageName(string packageName) =>
        packageName.Length is > 0 and <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || c is '+' or '-' or '.' or '_');

    private async Task<List<UbuntuPackageInfo>> QueryAsync(string packageName, CancellationToken ct)
    {
        var url = $"{ApiBase}?ws.op=getPublishedBinaries&binary_name={Uri.EscapeDataString(packageName)}" +
                  "&exact_match=true&status=Published&ws.size=5";

        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            return [];
        }

        var result = await response.Content.ReadFromJsonAsync<UbuntuSearchResult>(cancellationToken: ct);
        return result?.Entries ?? [];
    }

    private static string FormatResponse(List<UbuntuPackageInfo> packages, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"Ubuntu Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        var first = true;
        foreach (var package in packages.Take(3))
        {
            if (!first) sb.Append('\n');
            first = false;

            sb.Append($"package-name: {package.BinaryPackageName}\n");
            if (package.BinaryPackageVersion is { } version) sb.Append($"version: {version}\n");
            if (package.ComponentName is { } component) sb.Append($"component: {component}\n");
            if (package.SourcePackageName is { } sourceName) sb.Append($"source-package: {sourceName}\n");
            if (package.SourcePackageVersion is { } sourceVersion) sb.Append($"source-version: {sourceVersion}\n");
            if (package.SectionName is { } section) sb.Append($"section: {section}\n");
            if (package.Status is { } status) sb.Append($"status: {status}\n");
            if (package.DatePublished is { } date) sb.Append($"date-published: {date}\n");

            sb.Append($"ubuntu-url: {SearchUrl}/search?keywords={Uri.EscapeDataString(package.BinaryPackageName)}\n");
        }

        sb.Append("repository: Ubuntu\n");
        sb.Append("source: Launchpad API\n");
        sb.Append('\n');
        sb.Append("% Information retrieved from Ubuntu packages (Launchpad)\n");
        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            Ubuntu Package Not Found: {packageName}
            No package with this name was found in Ubuntu repositories.

            You can search manually at: {SearchUrl}/search?keywords={Uri.EscapeDataString(packageName)}

            % Package not found in Ubuntu repositories
            """;
    }

    private sealed record UbuntuSearchResult([property: JsonPropertyName("entries")] List<UbuntuPackageInfo>? Entries);

    private sealed record UbuntuPackageInfo(
        [property: JsonPropertyName("binary_package_name")] string BinaryPackageName,
        [property: JsonPropertyName("binary_package_version")] string? BinaryPackageVersion,
        [property: JsonPropertyName("component_name")] string? ComponentName,
        [property: JsonPropertyName("source_package_name")] string? SourcePackageName,
        [property: JsonPropertyName("source_package_version")] string? SourcePackageVersion,
        [property: JsonPropertyName("section_name")] string? SectionName,
        [property: JsonPropertyName("status")] string? Status,
        [property: JsonPropertyName("date_published")] string? DatePublished);
}
