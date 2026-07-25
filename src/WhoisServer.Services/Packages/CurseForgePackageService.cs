using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/curseforge.rs — the <c>-CURSEFORGE</c> suffix (CurseForge
/// Minecraft mod lookup). The CurseForge API requires a per-application API key (<c>x-api-key</c>
/// header) which the Rust version reads from the <c>CURSEFORGE_API_KEY</c> environment variable —
/// ported the same way here (an env var is a legitimate externalized-secret mechanism per
/// plan.md §4.5; no literal key is embedded). If the variable is unset, this returns the same
/// "not configured" notice as the original rather than throwing.
/// A numeric query is treated as a direct project ID lookup; anything else is a search
/// (gameId=432 is Minecraft, matching the Rust version).
/// </summary>
public sealed class CurseForgePackageService(HttpClient httpClient)
{
    private const string UserAgent = "whois-server-csharp/1.0 (contact: team@akae.re)";

    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(query) || query.Length > 200)
        {
            return "% Invalid CurseForge query\n";
        }

        var apiKey = Environment.GetEnvironmentVariable("CURSEFORGE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            return """
                % CurseForge API key not configured
                % Set the CURSEFORGE_API_KEY environment variable to enable CurseForge queries
                % Get your API key from: https://console.curseforge.com/

                """;
        }

        try
        {
            return ulong.TryParse(query, out var projectId)
                ? await GetProjectByIdAsync(apiKey, projectId, ct)
                : await SearchAsync(apiKey, query, ct);
        }
        catch (Exception e)
        {
            return $"% CurseForge query failed: {e.Message}\n";
        }
    }

    private async Task<string> GetProjectByIdAsync(string apiKey, ulong projectId, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, $"https://api.curseforge.com/v1/mods/{projectId}");
        request.Headers.TryAddWithoutValidation("Accept", "application/json");
        request.Headers.TryAddWithoutValidation("x-api-key", apiKey);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            return $"""
                % CurseForge project '{projectId}' not found
                % Status: {(int)response.StatusCode} {response.ReasonPhrase}

                """;
        }

        var data = await response.Content.ReadFromJsonAsync<CurseForgeResponse>(cancellationToken: ct);
        return data?.Data is { } project
            ? FormatProject(project)
            : $"% CurseForge project '{projectId}' not found\n";
    }

    private async Task<string> SearchAsync(string apiKey, string query, CancellationToken ct)
    {
        var url = $"https://api.curseforge.com/v1/mods/search?gameId=432&searchFilter={Uri.EscapeDataString(query)}&pageSize=5";
        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("Accept", "application/json");
        request.Headers.TryAddWithoutValidation("x-api-key", apiKey);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            return $"% CurseForge search failed: {(int)response.StatusCode} {response.ReasonPhrase}\n";
        }

        var data = await response.Content.ReadFromJsonAsync<SearchResponse>(cancellationToken: ct);
        if (data?.Data is not { Count: > 0 } projects)
        {
            return $"% No CurseForge mods found for: {query}\n";
        }

        var sb = new StringBuilder();
        sb.Append($"% CurseForge Search Results: {query}\n%\n");
        foreach (var project in projects)
        {
            sb.Append($"project-id: {project.Id}\n");
            sb.Append($"project-name: {project.Name}\n");
            sb.Append($"project-slug: {project.Slug}\n");
            sb.Append($"summary: {project.Summary}\n");
            if (project.Authors is { Count: > 0 })
            {
                sb.Append($"authors: {string.Join(", ", project.Authors.Select(a => a.Name))}\n");
            }

            sb.Append($"downloads: {FormatNumber(project.DownloadCount)}\n");
            sb.Append($"curseforge-url: https://www.curseforge.com/minecraft/mc-mods/{project.Slug}\n%\n");
        }

        sb.Append("% Use the project ID for detailed info: <project-id>-CURSEFORGE\n");
        return sb.ToString();
    }

    private static string FormatProject(CurseForgeProject project)
    {
        var sb = new StringBuilder();
        sb.Append($"project-id: {project.Id}\n");
        sb.Append($"project-name: {project.Name}\n");
        sb.Append($"project-slug: {project.Slug}\n");
        sb.Append($"summary: {project.Summary}\n");

        if (project.Authors is { Count: > 0 })
        {
            sb.Append($"authors: {string.Join(", ", project.Authors.Select(a => a.Name))}\n");
        }

        sb.Append($"total-downloads: {FormatNumber(project.DownloadCount)}\n");

        if (project.Categories is { Count: > 0 })
        {
            sb.Append($"categories: {string.Join(", ", project.Categories.Select(c => c.Name))}\n");
        }

        sb.Append($"created: {FormatDate(project.DateCreated)}\n");
        sb.Append($"last-modified: {FormatDate(project.DateModified)}\n");
        sb.Append($"last-release: {FormatDate(project.DateReleased)}\n");
        sb.Append($"curseforge-url: https://www.curseforge.com/minecraft/mc-mods/{project.Slug}\n");

        if (project.Links?.WebsiteUrl is { } website) sb.Append($"website: {website}\n");
        if (project.Links?.SourceUrl is { } source) sb.Append($"source-code: {source}\n");
        if (project.Links?.IssuesUrl is { } issues) sb.Append($"issue-tracker: {issues}\n");
        if (project.Links?.WikiUrl is { } wiki) sb.Append($"wiki: {wiki}\n");
        if (project.Logo?.Url is { } logo) sb.Append($"logo: {logo}\n");

        if (project.LatestFiles is { Count: > 0 } files)
        {
            var versions = files.SelectMany(f => f.GameVersions ?? []).Distinct().OrderBy(v => v).ToList();
            if (versions.Count > 0)
            {
                sb.Append($"minecraft-versions: {string.Join(", ", versions)}\n");
            }

            foreach (var file in files.Take(3))
            {
                sb.Append($"latest-file: {file.DisplayName} ({file.FileName}, {FormatDate(file.FileDate)})\n");
            }
        }

        sb.Append($"\n% View on CurseForge: https://www.curseforge.com/minecraft/mc-mods/{project.Slug}\n");
        return sb.ToString();
    }

    private static string FormatNumber(ulong n) => n.ToString("N0");

    private static string FormatDate(string? dateStr) =>
        string.IsNullOrEmpty(dateStr) ? "unknown" : dateStr.Split('T')[0];

    private sealed record CurseForgeResponse([property: JsonPropertyName("data")] CurseForgeProject Data);

    private sealed record SearchResponse([property: JsonPropertyName("data")] List<CurseForgeProject>? Data);

    private sealed record CurseForgeProject(
        [property: JsonPropertyName("id")] ulong Id,
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("slug")] string Slug,
        [property: JsonPropertyName("summary")] string Summary,
        [property: JsonPropertyName("links")] CurseForgeLinks? Links,
        [property: JsonPropertyName("authors")] List<CurseForgeAuthor>? Authors,
        [property: JsonPropertyName("logo")] CurseForgeLogo? Logo,
        [property: JsonPropertyName("downloadCount")] ulong DownloadCount,
        [property: JsonPropertyName("dateCreated")] string? DateCreated,
        [property: JsonPropertyName("dateModified")] string? DateModified,
        [property: JsonPropertyName("dateReleased")] string? DateReleased,
        [property: JsonPropertyName("categories")] List<CurseForgeCategory>? Categories,
        [property: JsonPropertyName("latestFiles")] List<CurseForgeFile>? LatestFiles);

    private sealed record CurseForgeLinks(
        [property: JsonPropertyName("websiteUrl")] string? WebsiteUrl,
        [property: JsonPropertyName("wikiUrl")] string? WikiUrl,
        [property: JsonPropertyName("issuesUrl")] string? IssuesUrl,
        [property: JsonPropertyName("sourceUrl")] string? SourceUrl);

    private sealed record CurseForgeAuthor([property: JsonPropertyName("name")] string Name);

    private sealed record CurseForgeLogo([property: JsonPropertyName("url")] string? Url);

    private sealed record CurseForgeCategory([property: JsonPropertyName("name")] string Name);

    private sealed record CurseForgeFile(
        [property: JsonPropertyName("displayName")] string DisplayName,
        [property: JsonPropertyName("fileName")] string FileName,
        [property: JsonPropertyName("fileDate")] string? FileDate,
        [property: JsonPropertyName("gameVersions")] List<string>? GameVersions);
}
