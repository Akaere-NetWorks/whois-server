using System.Text.RegularExpressions;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/alma.rs — the <c>-ALMA</c> suffix (AlmaLinux package lookup).
///
/// Deliberate bug fix, not a byte-for-byte port: the Rust version only probed whether a
/// repository's <c>repodata/repomd.xml</c> was reachable (always true) and then fabricated a
/// package entry claiming <c>version: Available</c> for *any* query string — including
/// nonexistent packages, so every query "succeeded". There is no public JSON package-search API
/// for AlmaLinux (verified live: no packages.almalinux.org search endpoint exists, and the real
/// yum <c>primary.xml.gz</c> repodata is 15-40MB compressed per repo — too heavy to fetch on every
/// query without a cache). Instead this port fetches the real, plain Apache directory listing of
/// each repo's flat <c>Packages/</c> folder (verified live against repo.almalinux.org: listings
/// are 250KB-1.2MB HTML, small enough per-query) and parses the standard RPM NVRA filename
/// convention (<c>name-version-release.arch.rpm</c>) to find a genuine match — so a nonexistent
/// package now correctly reports not-found instead of always "Available".
/// </summary>
public sealed class AlmaPackageService(HttpClient httpClient)
{
    private static readonly (string Repo, string BaseUrl)[] Repositories =
    [
        ("BaseOS", "https://repo.almalinux.org/almalinux/9/BaseOS/x86_64/os"),
        ("AppStream", "https://repo.almalinux.org/almalinux/9/AppStream/x86_64/os"),
        ("Extras", "https://repo.almalinux.org/almalinux/9/extras/x86_64/os"),
    ];

    private static readonly Regex RpmLinkRegex = new(
        """<a href="([^"]+\.rpm)">[^<]*</a>\s+\S+\s+\S+\s+(\d+)""",
        RegexOptions.Compiled);

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidName(packageName))
        {
            return $"""
                % Invalid AlmaLinux package name format: '{packageName}'
                % Package names must be <= 100 characters, contain no spaces, and use only
                % letters, digits, '+', '-', '.', or '_' (standard RPM naming conventions).

                """;
        }

        try
        {
            foreach (var (repo, baseUrl) in Repositories)
            {
                var found = await FindInRepoAsync(baseUrl, packageName, ct);
                if (found is { } pkg)
                {
                    return FormatFound(pkg, repo, baseUrl, packageName);
                }
            }
        }
        catch
        {
            // Any network/parse failure falls through to the not-found response below.
        }

        return FormatNotFound(packageName);
    }

    private async Task<NvraMatch?> FindInRepoAsync(string baseUrl, string packageName, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, $"{baseUrl}/Packages/");
        request.Headers.TryAddWithoutValidation("User-Agent", "whois-server/1.0 (AlmaLinux package lookup)");

        HttpResponseMessage response;
        try
        {
            response = await httpClient.SendAsync(request, ct);
        }
        catch
        {
            return null;
        }

        using (response)
        {
            if (!response.IsSuccessStatusCode)
            {
                return null;
            }

            var html = await response.Content.ReadAsStringAsync(ct);

            NvraMatch? best = null;
            foreach (Match m in RpmLinkRegex.Matches(html))
            {
                var filename = m.Groups[1].Value;
                if (!TryParseNvra(filename, out var name, out var version, out var release, out var arch))
                {
                    continue;
                }

                if (!string.Equals(name, packageName, StringComparison.Ordinal))
                {
                    continue;
                }

                var size = long.TryParse(m.Groups[2].Value, out var s) ? s : 0;
                var candidate = new NvraMatch(name, version, release, arch, size, filename);

                // Prefer x86_64/noarch builds over i686/src if multiple arches match.
                if (arch is "x86_64" or "noarch")
                {
                    return candidate;
                }

                best ??= candidate;
            }

            return best;
        }
    }

    /// <summary>Parses the standard RPM "Name-Version-Release.Arch.rpm" filename convention.
    /// Version/Release conventionally never contain '-', so splitting from the right twice
    /// isolates them, leaving (a possibly dash-containing) package name.</summary>
    private static bool TryParseNvra(string filename, out string name, out string version, out string release, out string arch)
    {
        name = version = release = arch = "";
        if (!filename.EndsWith(".rpm", StringComparison.Ordinal))
        {
            return false;
        }

        var trimmed = filename[..^4];
        var lastDot = trimmed.LastIndexOf('.');
        if (lastDot < 0)
        {
            return false;
        }

        arch = trimmed[(lastDot + 1)..];
        var nvr = trimmed[..lastDot];

        var releaseDash = nvr.LastIndexOf('-');
        if (releaseDash < 0)
        {
            return false;
        }

        release = nvr[(releaseDash + 1)..];
        var nv = nvr[..releaseDash];

        var versionDash = nv.LastIndexOf('-');
        if (versionDash < 0)
        {
            return false;
        }

        version = nv[(versionDash + 1)..];
        name = nv[..versionDash];

        return name.Length > 0;
    }

    private static bool IsValidName(string packageName) =>
        packageName.Length > 0
        && packageName.Length <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || "+-._".Contains(c));

    private static string FormatFound(NvraMatch pkg, string repo, string baseUrl, string query)
    {
        var sizeMb = pkg.Size / 1_048_576.0;
        return $"""
            AlmaLinux Package Information: {query}
            {new string('=', 60)}
            package: {pkg.Name}
            version: {pkg.Version}
            release: {pkg.Release}
            architecture: {pkg.Arch}
            size: {sizeMb:F2} MB
            repository: AlmaLinux-{repo}
            package-format: RPM
            distribution: AlmaLinux 9
            download-url: {baseUrl}/Packages/{pkg.Filename}

            % Installation Instructions:
            % dnf install {query}
            % yum install {query}

            % Additional Resources:
            % AlmaLinux Packages: https://packages.almalinux.org
            % AlmaLinux Wiki: https://wiki.almalinux.org/
            % Repository Mirrors: https://mirrors.almalinux.org/

            % Information retrieved live from the AlmaLinux {repo} repository
            % Query processed by WHOIS server

            """;
    }

    private static string FormatNotFound(string packageName) =>
        $"""
        % AlmaLinux Package '{packageName}' not found
        %
        % Search suggestions:
        % - Check package name spelling
        % - Package might be in the EPEL repository instead (try {packageName}-EPEL)
        % - Package might be named differently in the RHEL ecosystem
        %
        % dnf search {packageName}
        % dnf provides {packageName}
        %
        % Repository Information:
        % AlmaLinux Packages: https://packages.almalinux.org
        % EPEL Repository: https://docs.fedoraproject.org/en-US/epel/
        %
        % AlmaLinux is a 1:1 binary compatible fork of RHEL.
        % Most RHEL/CentOS packages are available; enable EPEL for additional software.

        """;

    private readonly record struct NvraMatch(string Name, string Version, string Release, string Arch, long Size, string Filename);
}
