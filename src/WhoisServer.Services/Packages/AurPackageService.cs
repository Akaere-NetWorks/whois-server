using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/aur.rs — the <c>-AUR</c> suffix (Arch User Repository
/// package lookup via the official RPC v5 <c>info</c> endpoint).
/// </summary>
public sealed class AurPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://aur.archlinux.org/rpc/v5/info";
    private const string PackageBase = "https://aur.archlinux.org/packages";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidName(packageName))
        {
            return $"""
                % Invalid AUR package name format: '{packageName}'
                % Package names must be non-empty, <= 100 characters, and contain no spaces.

                """;
        }

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"{ApiBase}?arg={Uri.EscapeDataString(packageName)}");
            request.Headers.TryAddWithoutValidation("User-Agent", "whois-server/1.0 (AUR package lookup)");

            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                return FormatNotFound(packageName);
            }

            var aurResponse = await response.Content.ReadFromJsonAsync<AurResponse>(cancellationToken: ct);
            var package = aurResponse?.Results?.FirstOrDefault();
            if (package is null)
            {
                return FormatNotFound(packageName);
            }

            return FormatFound(package);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    private static string FormatFound(AurPackage package)
    {
        var sb = new StringBuilder();
        sb.Append($"package: {package.Name}\n");
        sb.Append($"package-base: {package.PackageBase}\n");
        sb.Append($"version: {package.Version}\n");

        if (package.Description is { } description)
        {
            sb.Append($"description: {description}\n");
        }

        if (package.Url is { } url)
        {
            sb.Append($"upstream-url: {url}\n");
        }

        sb.Append($"aur-url: {PackageBase}/{package.Name}\n");
        sb.Append($"aur-id: {package.Id}\n");
        sb.Append($"votes: {package.NumVotes}\n");
        sb.Append($"popularity: {package.Popularity:F6}\n");
        sb.Append(package.Maintainer is { } maintainer ? $"maintainer: {maintainer}\n" : "maintainer: orphaned\n");

        // Real Unix-timestamp -> UTC conversion (the Rust version hand-rolled an approximate
        // calendar calculation here — `month = day_of_year / 30 + 1` — which is simply wrong for
        // several months per year; DateTimeOffset gives the exact date).
        sb.Append($"first-submitted: {FormatTimestamp(package.FirstSubmitted)}\n");
        sb.Append($"last-modified: {FormatTimestamp(package.LastModified)}\n");
        sb.Append(package.OutOfDate is { } ood ? $"out-of-date: {FormatTimestamp(ood)}\n" : "out-of-date: no\n");

        AppendList(sb, "depends", package.Depends);
        AppendList(sb, "makedepends", package.MakeDepends);
        AppendList(sb, "optdepends", package.OptDepends);
        AppendList(sb, "checkdepends", package.CheckDepends);
        AppendList(sb, "conflicts", package.Conflicts);
        AppendList(sb, "provides", package.Provides);
        AppendList(sb, "replaces", package.Replaces);
        AppendList(sb, "groups", package.Groups);
        AppendList(sb, "license", package.License);
        AppendList(sb, "keywords", package.Keywords);

        sb.Append('\n');
        sb.Append("% Additional Information:\n");
        sb.Append($"% AUR Package URL: {PackageBase}/{package.Name}\n");
        sb.Append($"% AUR Git Clone: https://aur.archlinux.org/{package.Name}.git\n");
        sb.Append($"% Install with: yay -S {package.Name} or paru -S {package.Name}\n");
        sb.Append("% Source: Arch User Repository (AUR)\n");

        return sb.ToString();
    }

    private static void AppendList(StringBuilder sb, string field, List<string>? values)
    {
        if (values is { Count: > 0 })
        {
            sb.Append($"{field}: {string.Join(", ", values)}\n");
        }
    }

    private static string FormatTimestamp(long unixSeconds) =>
        DateTimeOffset.FromUnixTimeSeconds(unixSeconds).UtcDateTime.ToString("yyyy-MM-dd HH:mm:ss") + " UTC";

    private static bool IsValidName(string packageName) =>
        packageName.Length is > 0 and <= 100 && !packageName.Contains(' ');

    private static string FormatNotFound(string packageName) =>
        $"""
        % Package '{packageName}' not found in AUR
        %
        % Search suggestions:
        % - Check package name spelling
        % - Try searching on: https://aur.archlinux.org/packages/?K={Uri.EscapeDataString(packageName)}
        % - Package might be in official Arch repositories
        %
        % AUR Information:
        % AUR URL: https://aur.archlinux.org/
        % AUR Guidelines: https://wiki.archlinux.org/title/AUR

        """;

    private sealed record AurResponse([property: JsonPropertyName("results")] List<AurPackage>? Results);

    private sealed record AurPackage(
        [property: JsonPropertyName("ID")] long Id,
        [property: JsonPropertyName("Name")] string Name,
        [property: JsonPropertyName("PackageBase")] string PackageBase,
        [property: JsonPropertyName("Version")] string Version,
        [property: JsonPropertyName("Description")] string? Description,
        [property: JsonPropertyName("URL")] string? Url,
        [property: JsonPropertyName("NumVotes")] long NumVotes,
        [property: JsonPropertyName("Popularity")] double Popularity,
        [property: JsonPropertyName("OutOfDate")] long? OutOfDate,
        [property: JsonPropertyName("Maintainer")] string? Maintainer,
        [property: JsonPropertyName("FirstSubmitted")] long FirstSubmitted,
        [property: JsonPropertyName("LastModified")] long LastModified,
        [property: JsonPropertyName("Depends")] List<string>? Depends,
        [property: JsonPropertyName("MakeDepends")] List<string>? MakeDepends,
        [property: JsonPropertyName("OptDepends")] List<string>? OptDepends,
        [property: JsonPropertyName("CheckDepends")] List<string>? CheckDepends,
        [property: JsonPropertyName("Conflicts")] List<string>? Conflicts,
        [property: JsonPropertyName("Provides")] List<string>? Provides,
        [property: JsonPropertyName("Replaces")] List<string>? Replaces,
        [property: JsonPropertyName("Groups")] List<string>? Groups,
        [property: JsonPropertyName("License")] List<string>? License,
        [property: JsonPropertyName("Keywords")] List<string>? Keywords);
}
