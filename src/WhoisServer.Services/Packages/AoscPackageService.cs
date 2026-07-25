using System.Net;
using System.Text;
using System.Text.RegularExpressions;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/aosc.rs — the <c>-AOSC</c> suffix (AOSC OS package lookup).
/// AOSC has no JSON package API, so — like the Rust version — this scrapes the real
/// packages.aosc.io HTML package page with the same regex patterns (ported near-verbatim since
/// they were evidently reverse-engineered against the live markup; simplified to the single most
/// reliable architecture/size extraction pattern rather than all three of the Rust version's
/// fallback heuristics).
/// </summary>
public sealed class AoscPackageService(HttpClient httpClient)
{
    private const string PackagesUrl = "https://packages.aosc.io/packages/";
    private const string SearchUrl = "https://packages.aosc.io/search?q=";

    private const string BrowserUserAgent =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

    private static readonly Regex VersionRegex = new("""<span class="pkg-version">([^<]+)</span>""", RegexOptions.Compiled);
    private static readonly Regex DescRegex = new("""<p class="description pkg-description">([^<]+)</p>""", RegexOptions.Compiled);
    private static readonly Regex SectionRegex = new("""<b class="pkg-field">Section</b>:\s*([^<]+)""", RegexOptions.Compiled);
    private static readonly Regex DependsBlockRegex = new(
        """<b class="pkg-field pkg-dep-rel">Depends</b>\s*:\s*\n\s*((?:<span class="pkg-dep"><a href="[^"]+">([^<]+)</a></span>,?\s*)+)""",
        RegexOptions.Compiled);
    private static readonly Regex DepNameRegex = new("""<a href="([^"]+)">([^<]+)</a>""", RegexOptions.Compiled);
    private static readonly Regex UpstreamSourceRegex = new(
        """<b class="pkg-field"[^>]*>Upstream</b>:\s*<a href="([^"]+)">source</a>""", RegexOptions.Compiled);
    private static readonly Regex UpstreamTarballRegex = new("""<a href="([^"]+)"\s*>\(tarball\)[^<]*</a>""", RegexOptions.Compiled);
    private static readonly Regex UpstreamVerRegex = new("""<a href="[^"]+"\s*\(git\)\s*([^<]+)</a>""", RegexOptions.Compiled);
    private static readonly Regex ArchSizeRegex = new("""([a-z0-9]+):\s+(\d+\.\d+\s+[KMGT]?iB)""", RegexOptions.Compiled);

    private static readonly HashSet<string> KnownArches =
        new(StringComparer.Ordinal) { "amd64", "arm64", "loongarch64", "loongson3", "mips64r6el", "ppc64el", "riscv64" };

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidName(packageName))
        {
            return $"""
                % Invalid AOSC package name format: '{packageName}'
                % Package names must be <= 100 characters, contain no spaces, and use only
                % letters, digits, '+', '-', '.', or '_'.

                """;
        }

        string html;
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"{PackagesUrl}{Uri.EscapeDataString(packageName)}");
            request.Headers.TryAddWithoutValidation("User-Agent", BrowserUserAgent);
            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                return FormatNotFound(packageName);
            }

            html = await response.Content.ReadAsStringAsync(ct);
        }
        catch
        {
            return FormatNotFound(packageName);
        }

        return ParseAndFormat(html, packageName);
    }

    private static string ParseAndFormat(string html, string query)
    {
        var version = Decode(VersionRegex.Match(html) is { Success: true } vm ? vm.Groups[1].Value : "unknown");
        var description = Decode(DescRegex.Match(html) is { Success: true } dm ? dm.Groups[1].Value : $"AOSC package: {query}");
        var section = SectionRegex.Match(html) is { Success: true } sm ? Decode(sm.Groups[1].Value.Trim()) : null;

        var depends = new List<string>();
        var dependsBlock = DependsBlockRegex.Match(html);
        if (dependsBlock.Success)
        {
            foreach (Match dm2 in DepNameRegex.Matches(dependsBlock.Groups[1].Value))
            {
                var depName = dm2.Groups[1].Value;
                if (!depName.StartsWith("http", StringComparison.Ordinal)
                    && !depName.StartsWith('/')
                    && !depName.Contains("github.com"))
                {
                    depends.Add(Decode(depName));
                }
            }
        }

        string? upstreamUrl = null;
        var upSource = UpstreamSourceRegex.Match(html);
        if (upSource.Success && upSource.Groups[1].Value.StartsWith("http", StringComparison.Ordinal) && upSource.Groups[1].Value.Length > 8)
        {
            upstreamUrl = upSource.Groups[1].Value;
        }
        else
        {
            var upTarball = UpstreamTarballRegex.Match(html);
            if (upTarball.Success && upTarball.Groups[1].Value.StartsWith("http", StringComparison.Ordinal))
            {
                upstreamUrl = upTarball.Groups[1].Value;
            }
        }

        var upstreamVerMatch = UpstreamVerRegex.Match(html);
        var upstreamVersion = upstreamVerMatch.Success ? Decode(upstreamVerMatch.Groups[1].Value.Trim()) : null;

        var architectures = new List<(string Arch, string Size)>();
        foreach (Match am in ArchSizeRegex.Matches(html))
        {
            var arch = am.Groups[1].Value.Trim();
            if (KnownArches.Contains(arch))
            {
                architectures.Add((arch, am.Groups[2].Value.Trim()));
            }
        }

        if (architectures.Count == 0 && version == "unknown")
        {
            return FormatNotFound(query);
        }

        var sb = new StringBuilder();
        sb.Append($"AOSC Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"package-name: {query}\n");
        sb.Append($"version: {version}\n");
        sb.Append($"description: {description}\n");

        if (section is not null)
        {
            sb.Append($"section: {section}\n");
        }

        if (depends.Count > 0)
        {
            sb.Append($"depends: {string.Join(", ", depends)}\n");
        }

        if (upstreamUrl is not null)
        {
            sb.Append($"upstream: {upstreamUrl}\n");
        }

        if (upstreamVersion is not null)
        {
            sb.Append($"upstream-version: {upstreamVersion}\n");
        }

        if (architectures.Count > 0)
        {
            sb.Append($"architectures: {string.Join(", ", architectures.Select(a => a.Arch))}\n");
            foreach (var (arch, size) in architectures)
            {
                sb.Append($"  {arch}: {version} ({size})\n");
            }
        }

        sb.Append($"aosc-url: {PackagesUrl}{Uri.EscapeDataString(query)}\n");
        sb.Append("repository: AOSC OS\n");
        sb.Append("source: AOSC Packages\n\n");
        sb.Append("% Information retrieved from AOSC packages\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private static string Decode(string s) => WebUtility.HtmlDecode(s);

    private static bool IsValidName(string packageName) =>
        packageName.Length > 0
        && packageName.Length <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || "+-._".Contains(c));

    private static string FormatNotFound(string packageName) =>
        $"""
        AOSC Package Not Found: {packageName}
        No package with this name was found in AOSC repositories.

        You can search manually at: {SearchUrl}{Uri.EscapeDataString(packageName)}

        % Package not found in AOSC repositories
        % Query processed by WHOIS server

        """;
}
