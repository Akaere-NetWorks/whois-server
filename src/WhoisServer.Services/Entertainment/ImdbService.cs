using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/imdb.rs — the <c>-IMDB</c> (movie/show lookup) and <c>-IMDBSEARCH</c>
/// (title search) suffixes, backed by the OMDb API (omdbapi.com). Requires an API key
/// (<c>OMDB_API_KEY</c>); degrades gracefully with a configuration message when none is supplied,
/// matching the Rust version's behavior rather than throwing.
/// </summary>
public sealed class ImdbService(HttpClient httpClient, string? apiKey)
{
    private const string BaseUrl = "http://www.omdbapi.com/";

    /// <summary>Handles the <c>-IMDB</c> suffix. Direct lookup by IMDb ID (<c>tt</c>-prefixed, 9+
    /// chars) or exact title; if a title lookup fails, falls back to a fuzzy search and returns
    /// details for the first hit — mirroring the Rust version's fallback chain.</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid IMDb query format. Use: <title_or_imdb_id>-IMDB\nExample: Inception-IMDB or tt1375666-IMDB\n";
        }

        if (string.IsNullOrEmpty(apiKey))
        {
            return $"""
                IMDb Query Failed for: {baseQuery}
                OMDB API key not configured.
                To enable IMDb queries, set the OMDB_API_KEY environment variable.
                You can get a free API key from: http://www.omdbapi.com/apikey.aspx

                """;
        }

        var isImdbId = baseQuery.StartsWith("tt", StringComparison.Ordinal) && baseQuery.Length >= 9;
        var searchParam = isImdbId ? $"i={baseQuery}" : $"t={Uri.EscapeDataString(baseQuery)}";
        var url = $"{BaseUrl}?{searchParam}&apikey={apiKey}&plot=full";

        ImdbResponse? data;
        try
        {
            using var response = await httpClient.GetAsync(url, ct);
            if (!response.IsSuccessStatusCode)
            {
                return $"IMDb Query Failed for: {baseQuery}\nHTTP Status: {(int)response.StatusCode}\n";
            }

            data = await response.Content.ReadFromJsonAsync<ImdbResponse>(cancellationToken: ct);
        }
        catch (Exception e)
        {
            return $"IMDb Query Failed for: {baseQuery}\nError: {e.Message}\n";
        }

        if (data?.Response == "True")
        {
            return FormatInfo(data);
        }

        if (!isImdbId)
        {
            try
            {
                var searchResult = await SearchAndGetFirstResultAsync(baseQuery, ct);
                if (searchResult is not null)
                {
                    return searchResult;
                }
            }
            catch
            {
                // fall through to the "not found" message below
            }

            return $"""
                IMDb Information Not Found for: {baseQuery}
                {data?.Error ?? "Movie not found!"}
                Note: For non-English titles, try using the English title or IMDb ID (e.g., tt1234567-IMDB)
                Use '<title>-IMDBSEARCH' for broader search results.

                """;
        }

        return $"IMDb Information Not Found for: {baseQuery}\n{data?.Error ?? "Movie not found!"}\n";
    }

    /// <summary>Handles the <c>-IMDBSEARCH</c> suffix: title search, limited to 10 results.</summary>
    public async Task<string> ProcessSearchAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid IMDb search query. Please provide a search term.\nExample: Batman-IMDBSEARCH\n";
        }

        if (string.IsNullOrEmpty(apiKey))
        {
            return $"""
                IMDb Search Failed for: {baseQuery}
                OMDB API key not configured.
                To enable IMDb searches, set the OMDB_API_KEY environment variable.
                You can get a free API key from: http://www.omdbapi.com/apikey.aspx

                """;
        }

        var url = $"{BaseUrl}?s={Uri.EscapeDataString(baseQuery)}&apikey={apiKey}";

        ImdbSearchResponse? data;
        try
        {
            using var response = await httpClient.GetAsync(url, ct);
            if (!response.IsSuccessStatusCode)
            {
                return $"IMDb Search Failed for: {baseQuery}\nHTTP Status: {(int)response.StatusCode}\n";
            }

            data = await response.Content.ReadFromJsonAsync<ImdbSearchResponse>(cancellationToken: ct);
        }
        catch (Exception e)
        {
            return $"IMDb Search Failed for: {baseQuery}\nError: {e.Message}\n";
        }

        if (data?.Response != "True")
        {
            return $"IMDb Search Failed for: {baseQuery}\n{data?.Error ?? "Unknown error"}\n";
        }

        var results = data.Search ?? [];
        if (results.Count == 0)
        {
            return $"No IMDb search results found for: {baseQuery}\n";
        }

        return FormatSearchResults(baseQuery, results.Take(10).ToList());
    }

    private async Task<string?> SearchAndGetFirstResultAsync(string query, CancellationToken ct)
    {
        var url = $"{BaseUrl}?s={Uri.EscapeDataString(query)}&apikey={apiKey}";
        using var response = await httpClient.GetAsync(url, ct);
        if (!response.IsSuccessStatusCode)
        {
            return null;
        }

        var searchData = await response.Content.ReadFromJsonAsync<ImdbSearchResponse>(cancellationToken: ct);
        var first = searchData?.Response == "True" ? searchData.Search?.FirstOrDefault() : null;
        if (first is null)
        {
            return null;
        }

        var detailsUrl = $"{BaseUrl}?i={first.ImdbId}&apikey={apiKey}&plot=full";
        using var detailsResponse = await httpClient.GetAsync(detailsUrl, ct);
        if (!detailsResponse.IsSuccessStatusCode)
        {
            return null;
        }

        var details = await detailsResponse.Content.ReadFromJsonAsync<ImdbResponse>(cancellationToken: ct);
        return details?.Response == "True" ? FormatInfo(details) : null;
    }

    private static string FormatInfo(ImdbResponse imdb)
    {
        var sb = new StringBuilder();
        sb.Append(imdb.Title is { Length: > 0 } t ? $"IMDb Information for: {t}\n" : "IMDb Information\n");
        sb.Append(new string('=', 60)).Append('\n');

        if (imdb.ImdbId is { Length: > 0 }) sb.Append($"imdb-id: {imdb.ImdbId}\n");
        if (imdb.Title is { Length: > 0 }) sb.Append($"title: {imdb.Title}\n");
        if (imdb.Year is { Length: > 0 }) sb.Append($"year: {imdb.Year}\n");
        if (imdb.Type is { Length: > 0 }) sb.Append($"type: {imdb.Type}\n");
        if (imdb.Rated is { Length: > 0 }) sb.Append($"rated: {imdb.Rated}\n");
        if (imdb.Runtime is { Length: > 0 }) sb.Append($"runtime: {imdb.Runtime}\n");
        if (imdb.Genre is { Length: > 0 }) sb.Append($"genre: {imdb.Genre}\n");
        if (imdb.Director is { Length: > 0 }) sb.Append($"director: {imdb.Director}\n");
        if (imdb.Writer is { Length: > 0 }) sb.Append($"writer: {imdb.Writer}\n");
        if (imdb.Actors is { Length: > 0 }) sb.Append($"actors: {imdb.Actors}\n");
        if (imdb.Language is { Length: > 0 }) sb.Append($"language: {imdb.Language}\n");
        if (imdb.Country is { Length: > 0 }) sb.Append($"country: {imdb.Country}\n");
        if (imdb.Released is { Length: > 0 }) sb.Append($"released: {imdb.Released}\n");
        if (imdb.ImdbRating is { Length: > 0 }) sb.Append($"imdb-rating: {imdb.ImdbRating}/10\n");
        if (imdb.ImdbVotes is { Length: > 0 }) sb.Append($"imdb-votes: {imdb.ImdbVotes}\n");
        if (imdb.Metascore is { Length: > 0 }) sb.Append($"metascore: {imdb.Metascore}/100\n");

        if (imdb.Ratings is { Count: > 0 })
        {
            foreach (var rating in imdb.Ratings)
            {
                sb.Append($"rating-{rating.Source.ToLowerInvariant().Replace(' ', '-')}: {rating.Value}\n");
            }
        }

        if (imdb.BoxOffice is { Length: > 0 }) sb.Append($"box-office: {imdb.BoxOffice}\n");
        if (imdb.Awards is { Length: > 0 } awards && awards != "N/A") sb.Append($"awards: {awards}\n");
        if (imdb.Production is { Length: > 0 } production && production != "N/A") sb.Append($"production: {production}\n");
        if (imdb.Website is { Length: > 0 } website && website != "N/A") sb.Append($"website: {website}\n");
        if (imdb.TotalSeasons is { Length: > 0 }) sb.Append($"total-seasons: {imdb.TotalSeasons}\n");

        if (imdb.Plot is { Length: > 0 } plot && plot != "N/A")
        {
            sb.Append($"plot: {plot.Replace("\r\n", " ").Replace('\n', ' ')}\n");
        }

        if (imdb.ImdbId is { Length: > 0 } imdbId)
        {
            sb.Append($"imdb-url: https://www.imdb.com/title/{imdbId}/\n");
        }

        return sb.ToString();
    }

    private static string FormatSearchResults(string query, List<ImdbSearchResult> results)
    {
        var sb = new StringBuilder();
        sb.Append($"IMDb Search Results for: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"Found {results.Count} titles:\n\n");

        for (var i = 0; i < results.Count; i++)
        {
            var r = results[i];
            sb.Append($"{i + 1}. Title Information\n");
            sb.Append(new string('-', 25)).Append('\n');
            sb.Append($"imdb-id: {r.ImdbId}\n");
            sb.Append($"title: {r.Title}\n");
            sb.Append($"year: {r.Year}\n");
            sb.Append($"type: {r.Type}\n");
            sb.Append($"imdb-url: https://www.imdb.com/title/{r.ImdbId}/\n\n");
        }

        sb.Append($"% Use '{results[0].ImdbId}-IMDB' to get detailed information for a specific title\n");
        sb.Append("% Search limited to top 10 results\n");
        return sb.ToString();
    }

    private sealed record ImdbResponse(
        [property: JsonPropertyName("Response")] string? Response,
        [property: JsonPropertyName("Title")] string? Title,
        [property: JsonPropertyName("Year")] string? Year,
        [property: JsonPropertyName("Rated")] string? Rated,
        [property: JsonPropertyName("Released")] string? Released,
        [property: JsonPropertyName("Runtime")] string? Runtime,
        [property: JsonPropertyName("Genre")] string? Genre,
        [property: JsonPropertyName("Director")] string? Director,
        [property: JsonPropertyName("Writer")] string? Writer,
        [property: JsonPropertyName("Actors")] string? Actors,
        [property: JsonPropertyName("Plot")] string? Plot,
        [property: JsonPropertyName("Language")] string? Language,
        [property: JsonPropertyName("Country")] string? Country,
        [property: JsonPropertyName("Awards")] string? Awards,
        [property: JsonPropertyName("Ratings")] List<ImdbRating>? Ratings,
        [property: JsonPropertyName("Metascore")] string? Metascore,
        [property: JsonPropertyName("imdbRating")] string? ImdbRating,
        [property: JsonPropertyName("imdbVotes")] string? ImdbVotes,
        [property: JsonPropertyName("imdbID")] string? ImdbId,
        [property: JsonPropertyName("Type")] string? Type,
        [property: JsonPropertyName("BoxOffice")] string? BoxOffice,
        [property: JsonPropertyName("Production")] string? Production,
        [property: JsonPropertyName("Website")] string? Website,
        [property: JsonPropertyName("Error")] string? Error,
        [property: JsonPropertyName("totalSeasons")] string? TotalSeasons);

    private sealed record ImdbRating(
        [property: JsonPropertyName("Source")] string Source,
        [property: JsonPropertyName("Value")] string Value);

    private sealed record ImdbSearchResponse(
        [property: JsonPropertyName("Search")] List<ImdbSearchResult>? Search,
        [property: JsonPropertyName("Response")] string? Response,
        [property: JsonPropertyName("Error")] string? Error);

    private sealed record ImdbSearchResult(
        [property: JsonPropertyName("Title")] string Title,
        [property: JsonPropertyName("Year")] string Year,
        [property: JsonPropertyName("imdbID")] string ImdbId,
        [property: JsonPropertyName("Type")] string Type);
}
