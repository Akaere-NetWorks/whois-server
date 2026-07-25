using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/debian.rs — the <c>-DEBIAN</c> suffix, backed by the real
/// sources.debian.org JSON API (<c>/api/src/{package}/</c>).
/// </summary>
public sealed class DebianPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://sources.debian.org/api/src";
    private const string DebianPackagesUrl = "https://packages.debian.org";
    private const string UbuntuPackagesUrl = "https://packages.ubuntu.com";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidName(packageName))
        {
            return $"""
                % Invalid Debian package name format: '{packageName}'
                % Package names must be <= 100 characters, contain no spaces, and use only
                % letters, digits, '+', '-', '.', or '_'.

                """;
        }

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"{ApiBase}/{Uri.EscapeDataString(packageName)}/");
            request.Headers.TryAddWithoutValidation("User-Agent", "whois-server/1.0 (Debian package lookup)");

            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                return FormatNotFound(packageName);
            }

            var data = await response.Content.ReadFromJsonAsync<DebianPackageResponse>(cancellationToken: ct);
            if (data?.Versions is not { Count: > 0 })
            {
                return FormatNotFound(packageName);
            }

            return FormatFound(data);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    private static string FormatFound(DebianPackageResponse package)
    {
        var latest = package.Versions[0];
        var sb = new StringBuilder();

        sb.Append($"package: {package.Package}\n");
        sb.Append($"version: {latest.Version}\n");

        if (latest.Suites is { Count: > 0 })
        {
            sb.Append($"suites: {string.Join(", ", latest.Suites)}\n");
        }

        if (latest.Area is { } area)
        {
            sb.Append($"area: {area}\n");
        }

        if (latest.Binaries is { Count: > 0 })
        {
            sb.Append($"binary-packages: {string.Join(", ", latest.Binaries)}\n");
        }

        sb.Append("repository: Debian Source Repository\n");
        sb.Append("package-format: deb\n");

        sb.Append("\n% Available Versions:\n");
        var shown = 0;
        foreach (var version in package.Versions)
        {
            if (shown >= 5)
            {
                sb.Append($"% ... and {package.Versions.Count - 5} more versions\n");
                break;
            }

            sb.Append($"% {version.Version}: {string.Join(", ", version.Suites ?? [])} ({version.Area ?? "unknown"})\n");
            shown++;
        }

        sb.Append("\n% Installation Instructions:\n");
        sb.Append($"% apt install {package.Package}\n");
        sb.Append($"% apt-get install {package.Package}\n");

        sb.Append("\n% Additional Information:\n");
        sb.Append($"% Debian Package: {DebianPackagesUrl}/search?keywords={Uri.EscapeDataString(package.Package)}\n");
        sb.Append($"% Ubuntu Package: {UbuntuPackagesUrl}/search?keywords={Uri.EscapeDataString(package.Package)}\n");
        sb.Append($"% Source Code: https://sources.debian.org/src/{package.Package}/\n");
        sb.Append($"% Bug Reports: https://bugs.debian.org/{package.Package}\n");
        sb.Append($"% Package Tracker: https://tracker.debian.org/pkg/{package.Package}\n");

        return sb.ToString();
    }

    private static bool IsValidName(string packageName) =>
        packageName.Length > 0
        && packageName.Length <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || "+-._".Contains(c));

    private static string FormatNotFound(string packageName) =>
        $"""
        % Package '{packageName}' not found in the Debian repository
        %
        % Search suggestions:
        % - Check package name spelling
        % - Try searching on: {DebianPackagesUrl}/search?keywords={Uri.EscapeDataString(packageName)}
        % - Try Ubuntu packages: {UbuntuPackagesUrl}/search?keywords={Uri.EscapeDataString(packageName)}
        % - Package might be in a PPA or third-party repository
        %
        % apt search {packageName}
        % apt-cache search {packageName}
        %
        % Repository Information:
        % Debian Packages: {DebianPackagesUrl}
        % Ubuntu Packages: {UbuntuPackagesUrl}
        % Source Browser: https://sources.debian.org/

        """;

    private sealed record DebianPackageResponse(
        [property: JsonPropertyName("package")] string Package,
        [property: JsonPropertyName("versions")] List<DebianVersion> Versions);

    private sealed record DebianVersion(
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("suites")] List<string>? Suites,
        [property: JsonPropertyName("area")] string? Area,
        [property: JsonPropertyName("binaries")] List<string>? Binaries);
}
