using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/epel.rs — the <c>-EPEL</c> suffix.
///
/// Deliberate bug fix, not a byte-for-byte port: the Rust version, like alma.rs, only probed
/// whether a repo's <c>repodata/repomd.xml</c> was reachable (always true) and fabricated a
/// package entry claiming <c>version: Available</c> for *any* query, so nonexistent packages
/// always "succeeded". Unlike AlmaLinux, EPEL packages are built by Fedora's Koji and are indexed
/// by Fedora's real, public metadata API — <c>mdapi.fedoraproject.org</c> — verified live
/// (<c>GET /epel9/pkg/htop</c> returns genuine JSON: version/release/summary/description/url/
/// package-size/etc; a nonexistent package returns HTTP 400). This port queries that real API
/// across epel10/epel9/epel8 branches (matching the Rust version's version-fallback ordering)
/// instead of faking a result.
/// </summary>
public sealed class EpelPackageService(HttpClient httpClient)
{
    private static readonly string[] Branches = ["epel10", "epel9", "epel8"];
    private const string EpelDocsUrl = "https://docs.fedoraproject.org/en-US/epel/";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidName(packageName))
        {
            return $"""
                % Invalid EPEL package name format: '{packageName}'
                % Package names must be <= 100 characters, contain no spaces, and use only
                % letters, digits, '+', '-', '.', or '_'.

                """;
        }

        foreach (var branch in Branches)
        {
            MdApiPackage? pkg;
            try
            {
                using var request = new HttpRequestMessage(
                    HttpMethod.Get, $"https://mdapi.fedoraproject.org/{branch}/pkg/{Uri.EscapeDataString(packageName)}");
                request.Headers.TryAddWithoutValidation("User-Agent", "whois-server/1.0 (EPEL package lookup)");

                using var response = await httpClient.SendAsync(request, ct);
                if (!response.IsSuccessStatusCode)
                {
                    continue;
                }

                pkg = await response.Content.ReadFromJsonAsync<MdApiPackage>(cancellationToken: ct);
            }
            catch
            {
                continue;
            }

            if (pkg is not null)
            {
                return FormatFound(pkg, branch, packageName);
            }
        }

        return FormatNotFound(packageName);
    }

    private static string FormatFound(MdApiPackage pkg, string branch, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"EPEL Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"package: {pkg.Basename ?? query}\n");

        var version = pkg.Version is { Length: > 0 }
            ? (pkg.Release is { Length: > 0 } ? $"{pkg.Version}-{pkg.Release}" : pkg.Version)
            : "unknown";
        sb.Append($"version: {version}\n");

        if (pkg.Arch is { } arch) sb.Append($"architecture: {arch}\n");
        if (pkg.Summary is { } summary) sb.Append($"summary: {summary}\n");
        if (pkg.Description is { } description)
        {
            var desc = description.Length > 300 ? description[..300] + "..." : description;
            sb.Append($"description: {desc.Replace('\n', ' ')}\n");
        }

        if (pkg.Url is { } url) sb.Append($"upstream-url: {url}\n");
        if (long.TryParse(pkg.PackageSize, out var pkgSize))
        {
            sb.Append($"package-size: {pkgSize / 1024.0:F2} KB\n");
        }

        if (long.TryParse(pkg.InstalledSize, out var installedSize))
        {
            sb.Append($"installed-size: {installedSize / 1024.0:F2} KB\n");
        }

        sb.Append($"epel-branch: {branch}\n");
        sb.Append("distribution: EPEL (Extra Packages for Enterprise Linux)\n");
        sb.Append("package-format: RPM\n");
        sb.Append("compatible-with: RHEL, CentOS, AlmaLinux, Rocky Linux\n");

        sb.Append("\n% Installation Instructions:\n");
        sb.Append("% dnf install epel-release   # enable EPEL first\n");
        sb.Append($"% dnf install {query}\n");

        sb.Append("\n% Additional Resources:\n");
        sb.Append($"% EPEL Documentation: {EpelDocsUrl}\n");
        sb.Append("% Package Database: https://packages.fedoraproject.org/\n");
        sb.Append($"% Live metadata: https://mdapi.fedoraproject.org/{branch}/pkg/{Uri.EscapeDataString(query)}\n");
        sb.Append("\n% Information retrieved live from Fedora's mdapi metadata service\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private static bool IsValidName(string packageName) =>
        packageName.Length > 0
        && packageName.Length <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || "+-._".Contains(c));

    private static string FormatNotFound(string packageName) =>
        $"""
        % EPEL Package '{packageName}' not found
        %
        % Search suggestions:
        % - Check package name spelling
        % - Package might be in a different EPEL version (8 vs 9 vs 10)
        % - Package might be in main distribution repositories instead
        %
        % dnf install epel-release
        % dnf search {packageName} --enablerepo=epel
        %
        % EPEL Repository Information:
        % EPEL Documentation: {EpelDocsUrl}
        % Package Search: https://packages.fedoraproject.org/
        %
        % EPEL provides additional packages for RHEL-compatible distributions,
        % maintained by the Fedora Project community.

        """;

    private sealed record MdApiPackage(
        [property: JsonPropertyName("version")] string? Version,
        [property: JsonPropertyName("release")] string? Release,
        [property: JsonPropertyName("arch")] string? Arch,
        [property: JsonPropertyName("summary")] string? Summary,
        [property: JsonPropertyName("description")] string? Description,
        [property: JsonPropertyName("basename")] string? Basename,
        [property: JsonPropertyName("url")] string? Url,
        [property: JsonPropertyName("package-size")] string? PackageSize,
        [property: JsonPropertyName("installed-size")] string? InstalledSize);
}
