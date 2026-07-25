using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/lyric.rs — the <c>-LYRIC</c> suffix. Fetches a random Luotianyi
/// (Vocaloid) lyric from lty.vc's public API and formats it as WHOIS-style text. There is no
/// per-query parameter in the Rust version either: any prefix before "-LYRIC" is accepted but
/// discarded, since the API only ever returns one random lyric per call. No caching — every query
/// is a fresh random pick, matching the original.
/// </summary>
public sealed class LyricService(HttpClient httpClient)
{
    private const string ApiUrl = "https://lty.vc/lyric?format=json";

    // Ported verbatim from the Rust reqwest::Client builder's user_agent(...) call.
    private const string UserAgent = "WhoisServer/1.0 (https://github.com/Akaere-NetWorks/whois-server)";

    /// <summary><paramref name="baseQuery"/> is already suffix-stripped by
    /// <see cref="WhoisServer.Core.Query.QueryAnalyzer"/> but is otherwise unused — the lyric API
    /// has no per-query parameter, mirroring the Rust version's own behavior.</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, ApiUrl);
            request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);

            using var response = await httpClient.SendAsync(request, ct);
            response.EnsureSuccessStatusCode();

            var lyric = await response.Content.ReadFromJsonAsync<LyricResponse>(cancellationToken: ct)
                ?? throw new InvalidOperationException("Empty lyric API response");

            return Format(lyric);
        }
        catch (Exception e)
        {
            return $"""
                % Luotianyi Lyric Query Failed
                % Error: {e.Message}
                % Source: lty.vc

                """;
        }
    }

    private static string Format(LyricResponse lyric)
    {
        var sb = new StringBuilder();
        sb.Append($"Luotianyi Random Lyric: {lyric.Title}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"song-name:          {lyric.Title}\n");
        sb.Append("singer:             洛天依 (Luotianyi)\n");

        if (lyric.Author is { Count: > 0 })
        {
            sb.Append($"author:             {string.Join(", ", lyric.Author)}\n");
        }

        sb.Append($"year:               {lyric.Year}\n");
        sb.Append("source:             lty.vc\n\n");

        sb.Append("lyric-content:\n");
        foreach (var line in lyric.Lines)
        {
            sb.Append(line).Append('\n');
        }

        sb.Append("\n% Information retrieved from lty.vc API\n");
        sb.Append("% Query processed by WHOIS server\n");

        return sb.ToString();
    }

    private sealed record LyricResponse(
        [property: JsonPropertyName("title")] string Title,
        [property: JsonPropertyName("author")] List<string> Author,
        [property: JsonPropertyName("year")] uint Year,
        [property: JsonPropertyName("lines")] List<string> Lines);
}
