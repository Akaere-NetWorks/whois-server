using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/acgc.rs — the <c>-ACGC</c> suffix ("Anime/Comic/Game Character" lookup).
/// Searches Moegirl Wiki (zh.moegirl.org.cn, a Chinese ACG wiki) via its MediaWiki API.
///
/// Not a byte-for-byte port: the Rust version's infobox-field extraction (~40 regex patterns
/// pulling 声优/年龄/身高/萌点/etc. out of the raw wikitext via <c>prop=revisions&amp;rvprop=content</c>)
/// no longer works at all — confirmed live against the real API, Moegirl Wiki now rejects
/// <c>action=query&amp;list=search</c> AND <c>prop=revisions</c> for anonymous callers with an
/// "action-notallowed" error (this is a wiki-side API restriction affecting the original Rust
/// implementation identically, not something this port broke). <c>action=opensearch</c> (for
/// finding the page) and <c>prop=extracts</c> (for a plain-text summary) still work anonymously,
/// so this port uses those and returns the plain-text intro instead of parsed infobox fields —
/// a reduced but genuinely working feature, rather than a byte-faithful port of logic that no
/// longer has any data to operate on.
/// </summary>
public sealed class AcgcService(HttpClient httpClient)
{
    private const string BaseUrl = "https://zh.moegirl.org.cn/api.php";

    // Moegirl Wiki rejects requests with no browser-like User-Agent.
    private const string UserAgent =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

    /// <summary><paramref name="baseQuery"/> is already suffix-stripped and expected to be a
    /// character name (any language Moegirl Wiki indexes, typically Chinese or Japanese/English).</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid ACGC query. Please provide a character name.\nExample: 利姆鲁-ACGC\n";
        }

        try
        {
            var titles = await SearchCharacterAsync(baseQuery, ct);
            if (titles.Count == 0)
            {
                return $"ACGC Character Not Found: {baseQuery}\nNo matching characters found on Moegirl Wiki.\n";
            }

            return await GetCharacterDetailsAsync(titles[0], ct);
        }
        catch (Exception e)
        {
            return $"ACGC Query Failed for: {baseQuery}\nError: {e.Message}\n";
        }
    }

    /// <summary>Uses the MediaWiki <c>opensearch</c> action, not <c>action=query&amp;list=search</c>
    /// (the latter rejects anonymous callers on Moegirl Wiki with an "action-notallowed" error —
    /// see the class doc comment). <c>opensearch</c> returns a plain JSON array
    /// (<c>[query, [titles...], [descriptions...], [urls...]]</c>), not the structured
    /// <c>query.search</c> shape.</summary>
    private async Task<List<string>> SearchCharacterAsync(string query, CancellationToken ct)
    {
        var url = $"{BaseUrl}?action=opensearch&format=json&search={Uri.EscapeDataString(query)}" +
                  "&limit=5&namespace=0";

        using var response = await httpClient.SendAsync(CreateRequest(url), ct);
        response.EnsureSuccessStatusCode();

        var json = await response.Content.ReadFromJsonAsync<JsonElement>(cancellationToken: ct);
        if (json.ValueKind != JsonValueKind.Array || json.GetArrayLength() < 2)
        {
            return [];
        }

        return [.. json[1].EnumerateArray().Select(e => e.GetString()).OfType<string>()];
    }

    private async Task<string> GetCharacterDetailsAsync(string title, CancellationToken ct)
    {
        var url = $"{BaseUrl}?action=query&format=json&titles={Uri.EscapeDataString(title)}" +
                  "&prop=extracts&exintro=1&explaintext=1&exsectionformat=plain&exlimit=1";

        using var response = await httpClient.SendAsync(CreateRequest(url), ct);
        response.EnsureSuccessStatusCode();

        var data = await response.Content.ReadFromJsonAsync<MediaWikiResponse>(cancellationToken: ct);
        var page = data?.Query?.Pages?.Values.FirstOrDefault(p => p.PageId is not null);

        return page is null
            ? throw new InvalidOperationException("No character details found")
            : FormatCharacterInfo(page);
    }

    private static HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);
        return request;
    }

    private static string FormatCharacterInfo(MediaWikiPage page)
    {
        var sb = new StringBuilder();
        sb.Append($"ACGC Character Information: {page.Title}\n");
        sb.Append(new string('=', 60)).Append('\n');

        if (page.PageId is { } pageId)
        {
            sb.Append($"page-id: {pageId}\n");
        }

        sb.Append($"character-name: {page.Title}\n");
        sb.Append("source: Moegirl Wiki (萌娘百科)\n");

        if (page.Extract is { Length: > 0 } extract)
        {
            var cleaned = CleanWikiText(extract);
            if (cleaned.Length > 0)
            {
                sb.Append($"description: {cleaned}\n");
            }
        }

        sb.Append($"moegirl-url: https://zh.moegirl.org.cn/{Uri.EscapeDataString(page.Title)}\n");
        return sb.ToString();
    }

    /// <summary>Strips leftover MediaWiki markup from a plain-text extract down to readable text.</summary>
    private static string CleanWikiText(string text)
    {
        text = text.Trim();
        if (text.Length == 0)
        {
            return "";
        }

        text = Regex.Replace(text, "<[^>]*>", "");
        text = Regex.Replace(text, @"\s+", " ");
        text = text.Replace("&nbsp;", " ").Replace("&lt;", "<").Replace("&gt;", ">").Replace("&amp;", "&");

        return text.Trim();
    }

    private sealed record MediaWikiResponse([property: JsonPropertyName("query")] MediaWikiQuery? Query);

    private sealed record MediaWikiQuery([property: JsonPropertyName("pages")] Dictionary<string, MediaWikiPage>? Pages);

    private sealed record MediaWikiPage(
        [property: JsonPropertyName("pageid")] long? PageId,
        [property: JsonPropertyName("title")] string Title,
        [property: JsonPropertyName("extract")] string? Extract);
}
