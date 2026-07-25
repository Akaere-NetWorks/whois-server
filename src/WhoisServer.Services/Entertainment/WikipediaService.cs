using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/wikipedia.rs — the <c>-WIKIPEDIA</c> suffix. Searches English
/// Wikipedia's MediaWiki API for the query, then fetches the top result's intro extract,
/// categories, and language links. Simplified from the Rust version per the migration's
/// "not byte-for-byte" policy: the multi-format <c>touched</c> timestamp reparsing (two
/// chrono format attempts) is dropped in favor of surfacing the API's own ISO-8601 string
/// as-is — it's already human-readable and MediaWiki's format doesn't vary in practice.
/// </summary>
public sealed class WikipediaService(HttpClient httpClient)
{
    private const string BaseUrl = "https://en.wikipedia.org/w/api.php";

    // Wikipedia's API is picky about default/absent user agents for anonymous callers — ported
    // verbatim from the Rust version's hardcoded browser UA.
    private const string UserAgent =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "% Invalid Wikipedia query. Please provide an article name.\n% Example: Rust-WIKIPEDIA\n";
        }

        try
        {
            var searchResults = await SearchAsync(baseQuery, ct);
            if (searchResults.Count == 0)
            {
                return $"% Wikipedia Article Not Found: {baseQuery}\n% No matching articles found on Wikipedia.\n";
            }

            return await GetArticleDetailsAsync(searchResults[0].Title, ct);
        }
        catch (Exception e)
        {
            return $"% Wikipedia Query Failed for: {baseQuery}\n% Error: {e.Message}\n";
        }
    }

    private async Task<List<SearchResult>> SearchAsync(string query, CancellationToken ct)
    {
        var url = $"{BaseUrl}?action=query&format=json&list=search&srsearch={Uri.EscapeDataString(query)}" +
                  "&srlimit=5&srnamespace=0&srprop=size|wordcount|timestamp|snippet&utf8=1";

        using var request = CreateRequest(url);
        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();

        var data = await response.Content.ReadFromJsonAsync<WikiResponse>(cancellationToken: ct);
        return data?.Query?.Search ?? [];
    }

    private async Task<string> GetArticleDetailsAsync(string title, CancellationToken ct)
    {
        var url = $"{BaseUrl}?action=query&format=json&titles={Uri.EscapeDataString(title)}" +
                  "&prop=extracts|info|categories|langlinks&exintro=1&explaintext=1&exsectionformat=plain" +
                  "&exlimit=1&inprop=url|length|touched&cllimit=10&lllimit=10&utf8=1";

        using var request = CreateRequest(url);
        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();

        var data = await response.Content.ReadFromJsonAsync<WikiResponse>(cancellationToken: ct);
        var page = data?.Query?.Pages?.Values.FirstOrDefault(p => p.PageId is not null);

        return page is null
            ? throw new InvalidOperationException("No article details found")
            : Format(page);
    }

    private static HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);
        return request;
    }

    private static string Format(WikiPage page)
    {
        var sb = new StringBuilder();
        sb.Append($"Wikipedia Article Information: {page.Title}\n");
        sb.Append(new string('=', 60)).Append('\n');

        if (page.PageId is { } id)
        {
            sb.Append($"page-id:            {id}\n");
        }

        sb.Append($"title:              {page.Title}\n");
        sb.Append("source:             Wikipedia (English)\n");

        if (page.Length is { } length)
        {
            sb.Append($"article-length:     {length} bytes\n");
        }

        if (page.Touched is { } touched)
        {
            sb.Append($"last-modified:      {touched}\n");
        }

        if (page.Extract is { Length: > 0 } extract)
        {
            var cleaned = CleanWikiText(extract);
            if (cleaned.Length > 0)
            {
                var truncated = cleaned.Length > 800 ? $"{cleaned[..800]}..." : cleaned;
                sb.Append($"summary:            {truncated}\n");
            }
        }

        if (page.Categories is { Count: > 0 })
        {
            var names = page.Categories
                .Select(c => c.Title)
                .Where(t => !string.IsNullOrEmpty(t))
                .Select(t => t!.Replace("Category:", ""))
                .Take(8)
                .ToList();

            if (names.Count > 0)
            {
                sb.Append($"categories:         {string.Join(", ", names)}\n");
            }
        }

        if (page.LangLinks is { Count: > 0 })
        {
            var langs = page.LangLinks
                .Where(l => l.Lang is not null && l.Title is not null)
                .Select(l => $"{l.Lang} ({l.Title})")
                .Take(8)
                .ToList();

            if (langs.Count > 0)
            {
                sb.Append($"languages:          {string.Join(", ", langs)}\n");
            }
        }

        var url = page.CanonicalUrl ?? page.FullUrl ?? $"https://en.wikipedia.org/wiki/{Uri.EscapeDataString(page.Title)}";
        sb.Append($"wikipedia-url:      {url}\n");

        sb.Append("\n% Information retrieved from Wikipedia via MediaWiki API\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    /// <summary>Strips MediaWiki markup (templates, wikilinks, bold/italic, HTML tags, entities)
    /// down to plain readable text. Ported from the Rust version's <c>clean_wiki_text</c>.</summary>
    private static string CleanWikiText(string text)
    {
        text = text.Trim();
        if (text.Length == 0)
        {
            return text;
        }

        text = Regex.Replace(text, @"\{\{[^}]*\}\}", "");
        text = Regex.Replace(text, @"\[\[([^|\]]*\|)?([^\]]*)\]\]", "$2");
        text = Regex.Replace(text, @"\[\[[^\]]*$", "");
        text = text.Replace("'''", "").Replace("''", "");
        text = Regex.Replace(text, "<[^>]*>", "");
        text = Regex.Replace(text, @"<ref[^>]*>.*?</ref>", "", RegexOptions.Singleline);
        text = Regex.Replace(text, @"\s+", " ");
        text = text.Replace("&nbsp;", " ")
            .Replace("&lt;", "<")
            .Replace("&gt;", ">")
            .Replace("&amp;", "&")
            .Replace("&quot;", "\"")
            .Replace("&#39;", "'");

        return text.Trim();
    }

    private sealed record WikiResponse([property: JsonPropertyName("query")] WikiQuery? Query);

    private sealed record WikiQuery(
        [property: JsonPropertyName("pages")] Dictionary<string, WikiPage>? Pages,
        [property: JsonPropertyName("search")] List<SearchResult>? Search);

    private sealed record WikiPage(
        [property: JsonPropertyName("pageid")] long? PageId,
        [property: JsonPropertyName("title")] string Title,
        [property: JsonPropertyName("extract")] string? Extract,
        [property: JsonPropertyName("fullurl")] string? FullUrl,
        [property: JsonPropertyName("canonicalurl")] string? CanonicalUrl,
        [property: JsonPropertyName("length")] long? Length,
        [property: JsonPropertyName("touched")] string? Touched,
        [property: JsonPropertyName("categories")] List<WikiCategory>? Categories,
        [property: JsonPropertyName("langlinks")] List<WikiLangLink>? LangLinks);

    private sealed record WikiCategory([property: JsonPropertyName("title")] string? Title);

    private sealed record WikiLangLink(
        [property: JsonPropertyName("lang")] string? Lang,
        [property: JsonPropertyName("title")] string? Title);

    private sealed record SearchResult([property: JsonPropertyName("title")] string Title);
}
