using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/cargo.rs — the <c>-CARGO</c> suffix (crates.io lookup).
/// crates.io's API policy requires a descriptive, non-generic <c>User-Agent</c> identifying the
/// calling application (anonymous/browser-spoofed UAs get rate-limited harder) — ported verbatim
/// from the Rust version's identifying string rather than a fake browser UA.
/// </summary>
public sealed class CargoPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://crates.io/api/v1/crates/";

    public async Task<string> ProcessAsync(string crateName, CancellationToken ct = default)
    {
        if (!IsValidName(crateName))
        {
            return $"""
                % Invalid Cargo crate name format: '{crateName}'
                % Crate names must be <= 64 characters, use only letters, digits, '-', or '_',
                % and must not start or end with '-'.

                """;
        }

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"{ApiBase}{Uri.EscapeDataString(crateName)}");
            request.Headers.TryAddWithoutValidation("User-Agent", "Mozilla/5.0 (compatible; WHOIS-Server/1.0)");

            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                return FormatNotFound(crateName);
            }

            var data = await response.Content.ReadFromJsonAsync<CratesResponse>(cancellationToken: ct);
            if (data is null)
            {
                return FormatNotFound(crateName);
            }

            return FormatFound(data, crateName);
        }
        catch
        {
            return FormatNotFound(crateName);
        }
    }

    private static string FormatFound(CratesResponse data, string query)
    {
        var info = data.Crate;
        var sb = new StringBuilder();

        sb.Append($"Rust Crate Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"crate-name: {info.Name}\n");
        sb.Append($"version: {info.NewestVersion}\n");

        if (info.MaxStableVersion is { } stable && stable != info.NewestVersion)
        {
            sb.Append($"stable-version: {stable}\n");
        }

        if (info.Description is { } description)
        {
            sb.Append($"description: {description}\n");
        }

        var latest = data.Versions?.FirstOrDefault();
        if (latest is not null)
        {
            if (latest.License is { } license)
            {
                sb.Append($"license: {license}\n");
            }

            if (latest.PublishedBy is { } publisher)
            {
                sb.Append(publisher.Name is { } name
                    ? $"published-by: {name} ({publisher.Login})\n"
                    : $"published-by: {publisher.Login}\n");
            }

            if (latest.Yanked)
            {
                sb.Append("yanked: true\n");
            }

            if (latest.CrateSize is { } crateSize)
            {
                var sizeKb = crateSize / 1024.0;
                sb.Append(sizeKb >= 1024.0
                    ? $"package-size: {sizeKb / 1024.0:F2} MB\n"
                    : $"package-size: {sizeKb:F2} KB\n");
            }
        }

        if (info.Homepage is { Length: > 0 } homepage)
        {
            sb.Append($"homepage: {homepage}\n");
        }

        if (info.Repository is { Length: > 0 } repository)
        {
            sb.Append($"repository: {repository}\n");
        }

        if (info.Documentation is { Length: > 0 } documentation)
        {
            sb.Append($"documentation: {documentation}\n");
        }

        sb.Append($"total-downloads: {FormatNumber(info.Downloads)}\n");
        if (info.RecentDownloads is { } recent)
        {
            sb.Append($"recent-downloads: {FormatNumber(recent)}\n");
        }

        if (data.Categories is { Count: > 0 } categories)
        {
            sb.Append($"categories: {string.Join(", ", categories.Take(5).Select(c => c.Category))}\n");
        }

        if (data.Keywords is { Count: > 0 } keywords)
        {
            sb.Append($"keywords: {string.Join(", ", keywords.Take(10).Select(k => k.Keyword))}\n");
        }

        if (data.Versions is { Count: > 1 } versions)
        {
            sb.Append($"total-versions: {versions.Count}\n");
            var recentVersions = versions.Take(5)
                .Select(v => v.Yanked ? $"{v.Num} (yanked)" : v.Num);
            sb.Append($"recent-versions: {string.Join(", ", recentVersions)}\n");
        }

        sb.Append($"created: {FormatTimestamp(info.CreatedAt)}\n");
        sb.Append($"updated: {FormatTimestamp(info.UpdatedAt)}\n");

        var encodedName = Uri.EscapeDataString(info.Name);
        sb.Append($"crates-io-url: https://crates.io/crates/{encodedName}\n");
        sb.Append($"docs-rs-url: https://docs.rs/{encodedName}\n");
        sb.Append($"api-url: {ApiBase}{encodedName}\n");
        sb.Append("registry: crates.io (Rust Package Registry)\n");
        sb.Append("source: crates.io API\n\n");
        sb.Append("% Information retrieved from crates.io\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private static string FormatNumber(ulong num) => num switch
    {
        >= 1_000_000 => $"{num / 1_000_000.0:F1}M",
        >= 1_000 => $"{num / 1_000.0:F1}K",
        _ => num.ToString(),
    };

    private static string FormatTimestamp(string isoTimestamp) =>
        DateTimeOffset.TryParse(isoTimestamp, out var dt)
            ? dt.UtcDateTime.ToString("yyyy-MM-dd HH:mm:ss") + " UTC"
            : isoTimestamp;

    private static bool IsValidName(string crateName) =>
        crateName.Length is > 0 and <= 64
        && crateName.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_')
        && !crateName.StartsWith('-')
        && !crateName.EndsWith('-');

    private static string FormatNotFound(string crateName) =>
        $"""
        Rust Crate Not Found: {crateName}
        No crate with this name was found on crates.io.

        You can search manually at: https://crates.io/search?q={Uri.EscapeDataString(crateName)}

        % Crate not found on crates.io
        % Query processed by WHOIS server

        """;

    private sealed record CratesResponse(
        [property: JsonPropertyName("crate")] CrateInfo Crate,
        [property: JsonPropertyName("versions")] List<CrateVersion>? Versions,
        [property: JsonPropertyName("keywords")] List<CrateKeyword>? Keywords,
        [property: JsonPropertyName("categories")] List<CrateCategory>? Categories);

    private sealed record CrateInfo(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("description")] string? Description,
        [property: JsonPropertyName("homepage")] string? Homepage,
        [property: JsonPropertyName("documentation")] string? Documentation,
        [property: JsonPropertyName("repository")] string? Repository,
        [property: JsonPropertyName("downloads")] ulong Downloads,
        [property: JsonPropertyName("recent_downloads")] ulong? RecentDownloads,
        [property: JsonPropertyName("max_stable_version")] string? MaxStableVersion,
        [property: JsonPropertyName("newest_version")] string NewestVersion,
        [property: JsonPropertyName("created_at")] string CreatedAt,
        [property: JsonPropertyName("updated_at")] string UpdatedAt);

    private sealed record CrateVersion(
        [property: JsonPropertyName("num")] string Num,
        [property: JsonPropertyName("yanked")] bool Yanked,
        [property: JsonPropertyName("license")] string? License,
        [property: JsonPropertyName("crate_size")] long? CrateSize,
        [property: JsonPropertyName("published_by")] CrateUser? PublishedBy);

    private sealed record CrateUser(
        [property: JsonPropertyName("login")] string Login,
        [property: JsonPropertyName("name")] string? Name);

    private sealed record CrateKeyword([property: JsonPropertyName("keyword")] string Keyword);

    private sealed record CrateCategory([property: JsonPropertyName("category")] string Category);
}
