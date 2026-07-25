using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/pypi.rs — the <c>-PYPI</c> suffix. Queries
/// <c>https://pypi.org/pypi/&lt;name&gt;/json</c>, PyPI's real (and only) JSON API. No caching:
/// the API is fast and free, same as the Rust version (which never cached this either).
/// </summary>
public sealed class PypiPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://pypi.org/pypi/";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidPackageName(packageName))
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var package = await QueryAsync(packageName, ct);
            return package is null ? FormatNotFound(packageName) : FormatResponse(package, packageName);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    /// <summary>Ported verbatim from the Rust version: max 214 chars, ASCII alphanumeric plus
    /// <c>-_.</c> only (PyPI's actual normalization rules are looser than this, but this matches
    /// what the Rust version already enforced).</summary>
    private static bool IsValidPackageName(string packageName) =>
        packageName.Length is > 0 and <= 214
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_' or '.');

    private async Task<PyPiResponse?> QueryAsync(string packageName, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(
            HttpMethod.Get, $"{ApiBase}{Uri.EscapeDataString(packageName)}/json");
        request.Headers.UserAgent.ParseAdd("Mozilla/5.0 (compatible; WHOIS-Server/1.0)");

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            return null;
        }

        return await response.Content.ReadFromJsonAsync<PyPiResponse>(cancellationToken: ct);
    }

    private static string FormatResponse(PyPiResponse package, string query)
    {
        var info = package.Info;
        var sb = new StringBuilder();
        sb.Append($"PyPI Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        sb.Append($"package-name: {info.Name}\n");
        sb.Append($"version: {info.Version}\n");

        if (info.Summary is { Length: > 0 } summary)
        {
            sb.Append($"summary: {summary}\n");
        }

        var author = (info.Author, info.AuthorEmail) switch
        {
            ({ } a, { } e) => $"{a} <{e}>",
            ({ } a, null) => a,
            (null, { } e) => e,
            _ => null,
        };
        if (author is not null) sb.Append($"author: {author}\n");

        var maintainer = (info.Maintainer, info.MaintainerEmail) switch
        {
            ({ } m, { } e) => $"{m} <{e}>",
            ({ } m, null) => m,
            (null, { } e) => e,
            _ => null,
        };
        if (maintainer is not null) sb.Append($"maintainer: {maintainer}\n");

        if (info.License is { Length: > 0 } license)
        {
            sb.Append($"license: {license}\n");
        }

        if (info.HomePage is { Length: > 0 } homepage)
        {
            sb.Append($"homepage: {homepage}\n");
        }

        if (info.ProjectUrls is { Count: > 0 } projectUrls)
        {
            foreach (var (key, url) in projectUrls.Take(5))
            {
                sb.Append($"{key.ToLowerInvariant().Replace(' ', '-')}: {url}\n");
            }
        }

        if (info.RequiresPython is { } requiresPython)
        {
            sb.Append($"requires-python: {requiresPython}\n");
        }

        if (info.Keywords is { Length: > 0 } keywords)
        {
            sb.Append($"keywords: {keywords}\n");
        }

        if (info.RequiresDist is { Count: > 0 } requiresDist)
        {
            var deps = requiresDist.Take(10).Select(d => d.Split(' ', 2)[0]);
            sb.Append($"dependencies: {string.Join(", ", deps)}\n");
            if (requiresDist.Count > 10)
            {
                sb.Append($"... and {requiresDist.Count - 10} more dependencies\n");
            }
        }

        if (info.Classifiers is { } classifiers)
        {
            var languages = classifiers
                .Where(c => c.StartsWith("Programming Language", StringComparison.Ordinal))
                .Take(3)
                .Select(c => c.Split("::").Last().Trim())
                .ToList();
            if (languages.Count > 0)
            {
                sb.Append($"programming-languages: {string.Join(", ", languages)}\n");
            }

            var devStatus = classifiers
                .FirstOrDefault(c => c.StartsWith("Development Status", StringComparison.Ordinal))
                ?.Split("::").Last().Trim();
            if (devStatus is not null)
            {
                sb.Append($"development-status: {devStatus}\n");
            }
        }

        if (package.Urls is { Count: > 0 } urls)
        {
            var totalSize = urls.Sum(u => u.Size ?? 0);
            if (totalSize > 0)
            {
                sb.Append($"total-size: {totalSize / 1024.0 / 1024.0:F2} MB\n");
            }

            var wheelCount = urls.Count(u => u.PackageType == "bdist_wheel");
            var sourceCount = urls.Count(u => u.PackageType == "sdist");
            if (wheelCount > 0) sb.Append($"wheel-files: {wheelCount}\n");
            if (sourceCount > 0) sb.Append($"source-files: {sourceCount}\n");
        }

        sb.Append($"pypi-url: https://pypi.org/project/{Uri.EscapeDataString(info.Name)}/\n");
        sb.Append($"api-url: {ApiBase}{Uri.EscapeDataString(info.Name)}/json\n");
        sb.Append("source: PyPI API\n");
        sb.Append('\n');
        sb.Append("% Information retrieved from PyPI\n");

        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            PyPI Package Not Found: {packageName}
            No package with this name was found on PyPI.

            You can search manually at: https://pypi.org/search/?q={Uri.EscapeDataString(packageName)}

            % Package not found on PyPI
            """;
    }

    private sealed record PyPiResponse(
        [property: JsonPropertyName("info")] PyPiInfo Info,
        [property: JsonPropertyName("urls")] List<PyPiUrl>? Urls);

    private sealed record PyPiInfo(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("summary")] string? Summary,
        [property: JsonPropertyName("home_page")] string? HomePage,
        [property: JsonPropertyName("author")] string? Author,
        [property: JsonPropertyName("author_email")] string? AuthorEmail,
        [property: JsonPropertyName("maintainer")] string? Maintainer,
        [property: JsonPropertyName("maintainer_email")] string? MaintainerEmail,
        [property: JsonPropertyName("license")] string? License,
        [property: JsonPropertyName("keywords")] string? Keywords,
        [property: JsonPropertyName("classifiers")] List<string>? Classifiers,
        [property: JsonPropertyName("requires_dist")] List<string>? RequiresDist,
        [property: JsonPropertyName("requires_python")] string? RequiresPython,
        [property: JsonPropertyName("project_urls")] Dictionary<string, string>? ProjectUrls);

    private sealed record PyPiUrl(
        [property: JsonPropertyName("size")] long? Size,
        [property: JsonPropertyName("packagetype")] string PackageType);
}
