using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/github.rs — the <c>-GITHUB</c> suffix. A query containing '/'
/// (e.g. "octocat/Hello-World") is treated as an owner/repository lookup against
/// <c>GET /repos/{owner}/{repo}</c>; otherwise it's treated as a user/organization lookup against
/// <c>GET /users/{name}</c>. The Rust version reads no <c>GITHUB_TOKEN</c> env var (confirmed by
/// grep) — it calls the GitHub REST API fully unauthenticated, so this port does too. GitHub's
/// unauthenticated rate limit is 60 req/hour/IP; wiring an optional token later would just mean
/// adding an `Authorization: Bearer` header in <see cref="CreateRequest"/>, no shape change needed.
/// </summary>
public sealed class GitHubService(HttpClient httpClient)
{
    private const string ApiUrl = "https://api.github.com";

    // GitHub's API rejects requests with no User-Agent at all — ported verbatim from the Rust
    // version's hardcoded UA.
    private const string UserAgent = "Mozilla/5.0 (compatible; WHOIS-Server/1.0)";

    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "% Invalid GitHub query. Please provide a username or owner/repository.\n";
        }

        if (baseQuery.Contains('/'))
        {
            var parts = baseQuery.Split('/');
            if (parts.Length != 2 || !IsValidName(parts[0]) || !IsValidName(parts[1]))
            {
                return $"% Invalid GitHub repository format. Use: owner/repository\n% Query: {baseQuery}\n";
            }

            return await QueryRepositoryAsync(parts[0], parts[1], baseQuery, ct);
        }

        if (!IsValidName(baseQuery))
        {
            return $"% Invalid GitHub username format: {baseQuery}\n";
        }

        return await QueryUserAsync(baseQuery, ct);
    }

    private static bool IsValidName(string name) =>
        name.Length is > 0 and <= 39
        && name.All(c => char.IsAsciiLetterOrDigit(c) || c == '-')
        && !name.StartsWith('-')
        && !name.EndsWith('-')
        && !name.Contains("--", StringComparison.Ordinal);

    private async Task<string> QueryUserAsync(string username, CancellationToken ct)
    {
        try
        {
            using var request = CreateRequest($"{ApiUrl}/users/{Uri.EscapeDataString(username)}");
            using var response = await httpClient.SendAsync(request, ct);

            if (response.StatusCode == HttpStatusCode.NotFound)
            {
                return NotFoundMessage(username, "user");
            }

            response.EnsureSuccessStatusCode();

            var user = await response.Content.ReadFromJsonAsync<GitHubUser>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty GitHub user response");

            return FormatUser(user, username);
        }
        catch
        {
            // Mirrors the Rust version: any failure (404, other HTTP error, parse error, rate
            // limit) collapses to the same "not found" formatted response rather than throwing.
            return NotFoundMessage(username, "user");
        }
    }

    private async Task<string> QueryRepositoryAsync(string owner, string repo, string originalQuery, CancellationToken ct)
    {
        try
        {
            using var request = CreateRequest($"{ApiUrl}/repos/{Uri.EscapeDataString(owner)}/{Uri.EscapeDataString(repo)}");
            using var response = await httpClient.SendAsync(request, ct);

            if (response.StatusCode == HttpStatusCode.NotFound)
            {
                return NotFoundMessage(originalQuery, "repository");
            }

            response.EnsureSuccessStatusCode();

            var repository = await response.Content.ReadFromJsonAsync<GitHubRepository>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty GitHub repository response");

            return FormatRepository(repository, originalQuery);
        }
        catch
        {
            return NotFoundMessage(originalQuery, "repository");
        }
    }

    private static HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);
        return request;
    }

    private static string NotFoundMessage(string query, string resourceType) =>
        $"""
        % GitHub {resourceType.ToUpperInvariant()} Not Found: {query}
        % No {resourceType} with this name was found on GitHub.
        %
        % You can search manually at: https://github.com/search?q={Uri.EscapeDataString(query)}

        """;

    private static string FormatUser(GitHubUser user, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"GitHub User Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"username:           {user.Login}\n");
        sb.Append($"user-id:            {user.Id}\n");
        sb.Append($"user-type:          {user.Type}\n");

        if (user.Name is { Length: > 0 }) sb.Append($"display-name:       {user.Name}\n");
        if (user.Bio is { Length: > 0 }) sb.Append($"bio:                {user.Bio}\n");
        if (user.Company is { Length: > 0 }) sb.Append($"company:            {user.Company}\n");
        if (user.Location is { Length: > 0 }) sb.Append($"location:           {user.Location}\n");
        if (user.Blog is { Length: > 0 }) sb.Append($"website:            {user.Blog}\n");
        if (user.TwitterUsername is { Length: > 0 }) sb.Append($"twitter:            @{user.TwitterUsername}\n");

        sb.Append($"public-repos:       {user.PublicRepos}\n");
        sb.Append($"public-gists:       {user.PublicGists}\n");
        sb.Append($"followers:          {user.Followers}\n");
        sb.Append($"following:          {user.Following}\n");

        if (user.Hireable is { } hireable) sb.Append($"hireable:           {hireable}\n");

        sb.Append($"created-at:         {user.CreatedAt}\n");
        sb.Append($"updated-at:         {user.UpdatedAt}\n");
        sb.Append($"github-url:         {user.HtmlUrl}\n");

        sb.Append("\n% Information retrieved from GitHub API\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private static string FormatRepository(GitHubRepository repo, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"GitHub Repository Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"repository-name:    {repo.Name}\n");
        sb.Append($"full-name:          {repo.FullName}\n");

        if (repo.Description is { Length: > 0 }) sb.Append($"description:        {repo.Description}\n");

        sb.Append($"owner:              {repo.Owner.Login}\n");

        if (repo.Language is { Length: > 0 }) sb.Append($"language:           {repo.Language}\n");
        if (repo.License is not null) sb.Append($"license:            {repo.License.Name}\n");

        sb.Append($"default-branch:     {repo.DefaultBranch}\n");
        sb.Append($"stars:              {repo.StargazersCount}\n");
        sb.Append($"watchers:           {repo.WatchersCount}\n");
        sb.Append($"forks:              {repo.ForksCount}\n");
        sb.Append($"open-issues:        {repo.OpenIssuesCount}\n");
        sb.Append($"visibility:         {(repo.Private ? "private" : "public")}\n");

        if (repo.Fork) sb.Append("fork:               true\n");
        if (repo.Archived) sb.Append("archived:           true\n");
        if (repo.Topics is { Count: > 0 }) sb.Append($"topics:             {string.Join(", ", repo.Topics)}\n");

        sb.Append($"created-at:         {repo.CreatedAt}\n");
        sb.Append($"updated-at:         {repo.UpdatedAt}\n");

        if (repo.PushedAt is { Length: > 0 }) sb.Append($"pushed-at:          {repo.PushedAt}\n");

        sb.Append($"github-url:         {repo.HtmlUrl}\n");
        sb.Append($"clone-url:          {repo.CloneUrl}\n");

        sb.Append("\n% Information retrieved from GitHub API\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private sealed record GitHubUser(
        [property: JsonPropertyName("login")] string Login,
        [property: JsonPropertyName("id")] long Id,
        [property: JsonPropertyName("html_url")] string HtmlUrl,
        [property: JsonPropertyName("type")] string Type,
        [property: JsonPropertyName("name")] string? Name,
        [property: JsonPropertyName("company")] string? Company,
        [property: JsonPropertyName("blog")] string? Blog,
        [property: JsonPropertyName("location")] string? Location,
        [property: JsonPropertyName("bio")] string? Bio,
        [property: JsonPropertyName("twitter_username")] string? TwitterUsername,
        [property: JsonPropertyName("hireable")] bool? Hireable,
        [property: JsonPropertyName("public_repos")] long PublicRepos,
        [property: JsonPropertyName("public_gists")] long PublicGists,
        [property: JsonPropertyName("followers")] long Followers,
        [property: JsonPropertyName("following")] long Following,
        [property: JsonPropertyName("created_at")] string CreatedAt,
        [property: JsonPropertyName("updated_at")] string UpdatedAt);

    private sealed record GitHubRepository(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("full_name")] string FullName,
        [property: JsonPropertyName("html_url")] string HtmlUrl,
        [property: JsonPropertyName("clone_url")] string CloneUrl,
        [property: JsonPropertyName("description")] string? Description,
        [property: JsonPropertyName("language")] string? Language,
        [property: JsonPropertyName("private")] bool Private,
        [property: JsonPropertyName("fork")] bool Fork,
        [property: JsonPropertyName("archived")] bool Archived,
        [property: JsonPropertyName("stargazers_count")] long StargazersCount,
        [property: JsonPropertyName("watchers_count")] long WatchersCount,
        [property: JsonPropertyName("forks_count")] long ForksCount,
        [property: JsonPropertyName("open_issues_count")] long OpenIssuesCount,
        [property: JsonPropertyName("default_branch")] string DefaultBranch,
        [property: JsonPropertyName("topics")] List<string>? Topics,
        [property: JsonPropertyName("license")] GitHubLicense? License,
        [property: JsonPropertyName("owner")] GitHubOwner Owner,
        [property: JsonPropertyName("created_at")] string CreatedAt,
        [property: JsonPropertyName("updated_at")] string UpdatedAt,
        [property: JsonPropertyName("pushed_at")] string? PushedAt);

    private sealed record GitHubOwner([property: JsonPropertyName("login")] string Login);

    private sealed record GitHubLicense([property: JsonPropertyName("name")] string Name);
}
