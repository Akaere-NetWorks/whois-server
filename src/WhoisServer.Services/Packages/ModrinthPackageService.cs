using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/modrinth.rs — the <c>-MODRINTH</c> suffix. Looks a slug/id up
/// directly against the Modrinth API first; on a non-success response, falls back to a text search
/// and returns the top hits instead (mirrors the Rust two-step direct-then-search behavior).
/// No caching: Modrinth's public API is fast and has generous rate limits, same as the Rust version
/// (which never cached this either).
/// </summary>
public sealed class ModrinthPackageService(HttpClient httpClient)
{
    private const string ApiBase = "https://api.modrinth.com/v2";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        var name = packageName.Trim();
        if (name.Length == 0)
        {
            return FormatNotFound(packageName);
        }

        try
        {
            using var projectResponse = await httpClient.GetAsync(
                $"{ApiBase}/project/{Uri.EscapeDataString(name)}", ct);

            if (projectResponse.IsSuccessStatusCode)
            {
                var project = await projectResponse.Content.ReadFromJsonAsync<ModrinthProject>(cancellationToken: ct);
                if (project is not null)
                {
                    return FormatProject(project);
                }
            }

            return await SearchAsync(name, ct);
        }
        catch
        {
            return FormatNotFound(name);
        }
    }

    private async Task<string> SearchAsync(string query, CancellationToken ct)
    {
        try
        {
            using var response = await httpClient.GetAsync(
                $"{ApiBase}/search?query={Uri.EscapeDataString(query)}&limit=5", ct);

            if (!response.IsSuccessStatusCode)
            {
                return FormatNotFound(query);
            }

            var result = await response.Content.ReadFromJsonAsync<ModrinthSearchResponse>(cancellationToken: ct);
            if (result is null || result.Hits.Count == 0)
            {
                return FormatNotFound(query);
            }

            var sb = new StringBuilder();
            sb.Append($"% Modrinth Search Results for: {query}\n");
            sb.Append($"% Total results: {result.TotalHits}\n%\n");

            foreach (var hit in result.Hits)
            {
                sb.Append($"% --- {hit.Title} ---\n");
                sb.Append($"project-slug:         {hit.Slug}\n");
                sb.Append($"project-type:         {hit.ProjectType}\n");
                sb.Append($"description:          {hit.Description}\n");
                sb.Append($"author:               {hit.Author}\n");
                sb.Append($"downloads:            {hit.Downloads}\n");
                sb.Append($"followers:            {hit.Follows}\n");
                if (hit.Categories.Count > 0)
                {
                    sb.Append($"categories:           {string.Join(", ", hit.Categories)}\n");
                }
                sb.Append($"client-side:          {hit.ClientSide}\n");
                sb.Append($"server-side:          {hit.ServerSide}\n");
                sb.Append($"license:              {hit.License}\n");
                if (hit.GameVersions.Count > 0)
                {
                    sb.Append($"mc-versions:          {hit.GameVersions.Count} versions available\n");
                }
                sb.Append($"modrinth-url:         https://modrinth.com/{hit.ProjectType}/{hit.Slug}\n\n");
            }

            sb.Append("% Use the exact slug for detailed info: <slug>-MODRINTH\n");
            return sb.ToString();
        }
        catch
        {
            return FormatNotFound(query);
        }
    }

    private static string FormatProject(ModrinthProject p)
    {
        var sb = new StringBuilder();
        sb.Append($"% Modrinth: {p.Title} ({p.ProjectType.ToUpperInvariant()})\n%\n");
        sb.Append($"project-slug:         {p.Slug}\n");
        sb.Append($"project-name:         {p.Title}\n");
        sb.Append($"project-type:         {p.ProjectType}\n");
        sb.Append($"description:          {p.Description}\n");
        sb.Append($"downloads:            {p.Downloads:N0}\n");
        sb.Append($"followers:            {p.Followers:N0}\n");

        if (p.Categories.Count > 0)
        {
            sb.Append($"categories:           {string.Join(", ", p.Categories)}\n");
        }

        sb.Append($"client-side:          {p.ClientSide}\n");
        sb.Append($"server-side:          {p.ServerSide}\n");

        if (p.Loaders.Count > 0)
        {
            sb.Append($"mod-loaders:          {string.Join(", ", p.Loaders)}\n");
        }

        if (p.GameVersions.Count > 0)
        {
            var total = p.GameVersions.Count;
            var display = total > 10
                ? $"{p.GameVersions[0]} to {p.GameVersions[^1]} ({total} versions)"
                : string.Join(", ", p.GameVersions);
            sb.Append($"minecraft-versions:   {display}\n");
        }

        if (p.License is { } license)
        {
            var licenseName = string.IsNullOrEmpty(license.Name) ? license.Id : license.Name;
            sb.Append($"license:              {licenseName} ({license.Id})\n");
            if (license.Url is { } licenseUrl)
            {
                sb.Append($"license-url:          {licenseUrl}\n");
            }
        }

        sb.Append($"published:            {FormatDate(p.Published)}\n");
        sb.Append($"last-updated:         {FormatDate(p.Updated)}\n");
        if (p.Versions.Count > 0)
        {
            sb.Append($"published-versions:   {p.Versions.Count}\n");
        }

        sb.Append('\n');
        sb.Append($"modrinth-url:         https://modrinth.com/{p.ProjectType}/{p.Slug}\n");
        if (p.SourceUrl is { } source) sb.Append($"source-code:          {source}\n");
        if (p.IssuesUrl is { } issues) sb.Append($"issue-tracker:        {issues}\n");
        if (p.WikiUrl is { } wiki) sb.Append($"wiki:                 {wiki}\n");
        if (p.DiscordUrl is { } discord) sb.Append($"discord:              {discord}\n");

        if (p.DonationUrls.Count > 0)
        {
            sb.Append('\n');
            foreach (var donation in p.DonationUrls)
            {
                sb.Append($"donate ({donation.Platform}):  {donation.Url}\n");
            }
        }

        sb.Append('\n');
        sb.Append("% Information retrieved from Modrinth API\n");
        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            Modrinth Project Not Found: {packageName}
            No project with this slug/ID was found, and no search results matched either.

            You can search manually at: https://modrinth.com/mods?q={Uri.EscapeDataString(packageName)}

            % Project not found on Modrinth
            """;
    }

    private static string FormatDate(string? isoDate) =>
        isoDate is null ? "unknown" : isoDate.Split('T')[0];

    private sealed record ModrinthProject(
        [property: JsonPropertyName("slug")] string Slug,
        [property: JsonPropertyName("title")] string Title,
        [property: JsonPropertyName("description")] string Description,
        [property: JsonPropertyName("project_type")] string ProjectType,
        [property: JsonPropertyName("categories")] List<string> Categories,
        [property: JsonPropertyName("additional_categories")] List<string>? AdditionalCategories,
        [property: JsonPropertyName("client_side")] string ClientSide,
        [property: JsonPropertyName("server_side")] string ServerSide,
        [property: JsonPropertyName("downloads")] long Downloads,
        [property: JsonPropertyName("followers")] int Followers,
        [property: JsonPropertyName("versions")] List<string> Versions,
        [property: JsonPropertyName("game_versions")] List<string> GameVersions,
        [property: JsonPropertyName("loaders")] List<string> Loaders,
        [property: JsonPropertyName("license")] ModrinthLicense? License,
        [property: JsonPropertyName("published")] string Published,
        [property: JsonPropertyName("updated")] string Updated,
        [property: JsonPropertyName("source_url")] string? SourceUrl,
        [property: JsonPropertyName("issues_url")] string? IssuesUrl,
        [property: JsonPropertyName("wiki_url")] string? WikiUrl,
        [property: JsonPropertyName("discord_url")] string? DiscordUrl,
        [property: JsonPropertyName("donation_urls")] List<ModrinthDonation>? DonationUrlsRaw)
    {
        [JsonIgnore]
        public List<ModrinthDonation> DonationUrls => DonationUrlsRaw ?? [];
    }

    private sealed record ModrinthLicense(
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("url")] string? Url);

    private sealed record ModrinthDonation(
        [property: JsonPropertyName("platform")] string Platform,
        [property: JsonPropertyName("url")] string Url);

    private sealed record ModrinthSearchResponse(
        [property: JsonPropertyName("hits")] List<ModrinthSearchHit> Hits,
        [property: JsonPropertyName("total_hits")] int TotalHits);

    private sealed record ModrinthSearchHit(
        [property: JsonPropertyName("slug")] string Slug,
        [property: JsonPropertyName("title")] string Title,
        [property: JsonPropertyName("description")] string Description,
        [property: JsonPropertyName("categories")] List<string> Categories,
        [property: JsonPropertyName("client_side")] string ClientSide,
        [property: JsonPropertyName("server_side")] string ServerSide,
        [property: JsonPropertyName("project_type")] string ProjectType,
        [property: JsonPropertyName("downloads")] long Downloads,
        [property: JsonPropertyName("follows")] int Follows,
        [property: JsonPropertyName("author")] string Author,
        [property: JsonPropertyName("versions")] List<string> GameVersions,
        [property: JsonPropertyName("license")] string License);
}
