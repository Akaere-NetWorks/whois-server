using System.Text;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/opensuse.rs — the <c>-OPENSUSE</c> suffix.
///
/// Deliberate fix, not a byte-for-byte port: the Rust version never actually calls a real API.
/// <c>parse_opensuse_html</c> ignores the fetched HTML entirely and
/// <c>generate_opensuse_package_info</c> returns hardcoded, fabricated version numbers for "vim"
/// and "git" and a generic placeholder for everything else — it never reflects real openSUSE
/// package data. This port instead uses the openSUSE Build Service's real, unauthenticated
/// <c>/public/</c> API (confirmed live: <c>api.opensuse.org/public/source/openSUSE:Factory/&lt;pkg&gt;/_meta</c>
/// for title/description, <c>api.opensuse.org/public/build/openSUSE:Factory/standard/x86_64/&lt;pkg&gt;</c>
/// for the actual built binary/version/size list) against the <c>openSUSE:Factory</c> project —
/// the Tumbleweed development project, where the overwhelming majority of packages live — mirroring
/// the "Tumbleweed" choice the Rust version's fake data already assumed, just backed by real data
/// now. No caching: OBS's public API is fast and doesn't rate-limit this usage pattern.
/// </summary>
public sealed class OpenSusePackageService(HttpClient httpClient)
{
    private const string Project = "openSUSE:Factory";
    private const string Repo = "standard";
    private const string Arch = "x86_64";
    private const string ApiBase = "https://api.opensuse.org/public";
    private const string PackagesUrl = "https://software.opensuse.org/package/";

    private static readonly Regex BinaryNameRegex = new(
        @"^(?<name>.+)-(?<version>[^-]+)-(?<release>[^-]+)\.(?<arch>[a-zA-Z0-9_]+)\.rpm$",
        RegexOptions.Compiled);

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidPackageName(packageName))
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var meta = await FetchMetaAsync(packageName, ct);
            if (meta is null)
            {
                return FormatNotFound(packageName);
            }

            var binaries = await FetchBinariesAsync(packageName, ct);
            return FormatResponse(packageName, meta.Value.Title, meta.Value.Description, meta.Value.Devel, binaries);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    /// <summary>Ported verbatim from the Rust version: max 100 chars, ASCII alphanumeric plus
    /// <c>+-._</c>, no spaces.</summary>
    private static bool IsValidPackageName(string packageName) =>
        packageName.Length is > 0 and <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || c is '+' or '-' or '.' or '_');

    private async Task<(string? Title, string? Description, string? Devel)?> FetchMetaAsync(
        string packageName, CancellationToken ct)
    {
        var url = $"{ApiBase}/source/{Uri.EscapeDataString(Project)}/{Uri.EscapeDataString(packageName)}/_meta";
        using var response = await httpClient.GetAsync(url, ct);
        if (!response.IsSuccessStatusCode)
        {
            return null;
        }

        var xml = await response.Content.ReadAsStringAsync(ct);
        var doc = XDocument.Parse(xml);
        var root = doc.Root;
        if (root is null)
        {
            return null;
        }

        var title = root.Element("title")?.Value.Trim();
        var description = root.Element("description")?.Value.Trim();
        var develElement = root.Element("devel");
        var devel = develElement is null
            ? null
            : $"{develElement.Attribute("project")?.Value}/{develElement.Attribute("package")?.Value}";

        return (title, description, devel);
    }

    private async Task<List<ObsBinary>> FetchBinariesAsync(string packageName, CancellationToken ct)
    {
        var url = $"{ApiBase}/build/{Uri.EscapeDataString(Project)}/{Repo}/{Arch}/{Uri.EscapeDataString(packageName)}";
        using var response = await httpClient.GetAsync(url, ct);
        if (!response.IsSuccessStatusCode)
        {
            return [];
        }

        var xml = await response.Content.ReadAsStringAsync(ct);
        var doc = XDocument.Parse(xml);

        var binaries = new List<ObsBinary>();
        foreach (var element in doc.Root?.Elements("binary") ?? Enumerable.Empty<XElement>())
        {
            var filename = element.Attribute("filename")?.Value;
            if (filename is null || !filename.EndsWith(".rpm", StringComparison.Ordinal))
            {
                continue;
            }

            var match = BinaryNameRegex.Match(filename);
            if (!match.Success)
            {
                continue;
            }

            var size = long.TryParse(element.Attribute("size")?.Value, out var s) ? s : (long?)null;
            binaries.Add(new ObsBinary(
                match.Groups["name"].Value, match.Groups["version"].Value,
                match.Groups["release"].Value, match.Groups["arch"].Value, size));
        }

        return binaries;
    }

    private static string FormatResponse(
        string query, string? title, string? description, string? devel, List<ObsBinary> binaries)
    {
        var sb = new StringBuilder();
        sb.Append($"OpenSUSE Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        sb.Append($"package-name: {query}\n");
        if (title is { Length: > 0 }) sb.Append($"title: {title}\n");
        sb.Append($"project: {Project}\n");
        sb.Append("distribution: Tumbleweed\n");

        var primary = binaries.FirstOrDefault(b =>
            b.Name.Equals(query, StringComparison.OrdinalIgnoreCase) && b.Arch != "src");
        if (primary is not null)
        {
            sb.Append($"version: {primary.Version}\n");
            sb.Append($"release: {primary.Release}\n");
            sb.Append($"architecture: {primary.Arch}\n");
            if (primary.Size is { } size) sb.Append($"size: {size / 1024.0 / 1024.0:F2} MB\n");
        }

        if (description is { Length: > 0 })
        {
            var truncated = description.Length > 300 ? description[..300] + "..." : description;
            sb.Append($"description: {truncated}\n");
        }

        if (devel is not null)
        {
            sb.Append($"development-project: {devel}\n");
        }

        var subPackages = binaries
            .Where(b => !b.Name.Equals(query, StringComparison.OrdinalIgnoreCase) && b.Arch != "src")
            .Select(b => b.Name)
            .Distinct()
            .ToList();
        if (subPackages.Count > 0)
        {
            sb.Append($"related-packages: {string.Join(", ", subPackages.Take(10))}\n");
        }

        sb.Append($"opensuse-url: {PackagesUrl}{Uri.EscapeDataString(query)}\n");
        sb.Append("source: openSUSE Build Service public API\n");
        sb.Append('\n');
        sb.Append("% Information retrieved from openSUSE Build Service\n");
        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            OpenSUSE Package Not Found: {packageName}
            No package with this name was found in the openSUSE:Factory project.

            You can search manually at: {PackagesUrl}{Uri.EscapeDataString(packageName)}

            % Package not found in OpenSUSE repositories
            """;
    }

    private sealed record ObsBinary(string Name, string Version, string Release, string Arch, long? Size);
}
