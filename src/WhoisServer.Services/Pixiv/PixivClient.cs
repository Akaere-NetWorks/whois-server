using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using System.Web;

namespace WhoisServer.Services.Pixiv;

public sealed class PixivRateLimitException() : Exception("Pixiv API rate limit exceeded");

/// <summary>
/// Ported from src/services/pixiv/client.rs. Rewrites <c>i.pximg.net</c> image URLs to this
/// server's own <c>/pixiv-proxy/{path}</c> endpoint (see
/// <see cref="WhoisServer.Host.Web.PixivProxyEndpoints"/>) unconditionally — a simplification
/// over the Rust version, which only rewrote URLs when a <c>PIXIV_PROXY_BASE_URL</c> env var
/// pointing at some externally-hosted proxy was configured. Since this port always ships its own
/// proxy endpoint, there's no reason to make that configurable or optional.
/// </summary>
public sealed class PixivClient(HttpClient httpClient, PixivAuthClient auth)
{
    private const string ApiBase = "https://app-api.pixiv.net";

    private async Task<T> AuthenticatedRequestAsync<T>(string path, Dictionary<string, string> query, CancellationToken ct)
    {
        var accessToken = await auth.GetAccessTokenAsync(ct);

        var clientTime = DateTimeOffset.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss+00:00");
        var clientHash = PixivAuthClient.ComputeClientHash(clientTime);

        var queryString = string.Join('&', query.Select(kv => $"{HttpUtility.UrlEncode(kv.Key)}={HttpUtility.UrlEncode(kv.Value)}"));
        using var request = new HttpRequestMessage(HttpMethod.Get, $"{ApiBase}{path}?{queryString}");
        request.Headers.TryAddWithoutValidation("Authorization", $"Bearer {accessToken}");
        request.Headers.TryAddWithoutValidation("X-Client-Time", clientTime);
        request.Headers.TryAddWithoutValidation("X-Client-Hash", clientHash);
        request.Headers.TryAddWithoutValidation("App-OS", "android");
        request.Headers.TryAddWithoutValidation("App-OS-Version", "9.0");
        request.Headers.TryAddWithoutValidation("App-Version", "5.0.64");
        request.Headers.TryAddWithoutValidation("User-Agent", PixivAuthClient.UserAgent);

        using var response = await httpClient.SendAsync(request, ct);

        if (response.StatusCode == HttpStatusCode.TooManyRequests)
        {
            throw new PixivRateLimitException();
        }

        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<T>(cancellationToken: ct);
        return result ?? throw new InvalidOperationException("Empty Pixiv API response");
    }

    public async Task<Artwork> GetArtworkInfoAsync(long artworkId, CancellationToken ct)
    {
        var response = await AuthenticatedRequestAsync<PixivResponse<Dictionary<string, JsonElement>>>(
            "/v1/illust/detail", new Dictionary<string, string> { ["illust_id"] = artworkId.ToString(), ["filter"] = "for_ios" }, ct);

        var body = response.Body ?? throw new InvalidOperationException("Empty artwork response");
        var artwork = body["illust"].Deserialize<Artwork>() ?? throw new InvalidOperationException("Failed to parse artwork");
        return ProcessArtwork(artwork);
    }

    public async Task<UserProfile> GetUserInfoAsync(long userId, CancellationToken ct)
    {
        var response = await AuthenticatedRequestAsync<UserProfile>(
            "/v1/user/detail",
            new Dictionary<string, string> { ["user_id"] = userId.ToString(), ["filter"] = "for_ios", ["illust_id"] = "0" },
            ct);
        return response;
    }

    public async Task<List<Artwork>> SearchArtworksAsync(string keyword, int limit, CancellationToken ct)
    {
        var response = await AuthenticatedRequestAsync<SearchResults>(
            "/v1/search/illust",
            new Dictionary<string, string>
            {
                ["word"] = keyword,
                ["search_target"] = "partial_match_for_tags",
                ["sort"] = "date_desc",
                ["filter"] = "for_ios",
            },
            ct);

        var all = response.Illusts.Concat(response.Manga).Select(ProcessArtwork).Take(limit).ToList();
        return all;
    }

    public async Task<List<Artwork>> GetRankingAsync(string mode, int limit, CancellationToken ct)
    {
        var response = await AuthenticatedRequestAsync<RankingResults>(
            $"/v1/ranking/{mode}", new Dictionary<string, string> { ["filter"] = "for_ios" }, ct);

        var artworks = new List<Artwork>();
        foreach (var item in response.Contents.Take(limit))
        {
            try
            {
                artworks.Add(await GetArtworkInfoAsync(item.IllustId, ct));
            }
            catch
            {
                // Same best-effort behavior as the Rust version: a failed detail fetch for one
                // ranking entry just drops that entry rather than failing the whole ranking.
            }
        }

        return artworks;
    }

    public async Task<List<Artwork>> GetUserIllustsAsync(long userId, int limit, CancellationToken ct)
    {
        var response = await AuthenticatedRequestAsync<UserArtworks>(
            "/v1/user/illusts", new Dictionary<string, string> { ["user_id"] = userId.ToString(), ["filter"] = "for_ios" }, ct);

        return response.Illusts.Concat(response.Manga).Select(ProcessArtwork).Take(limit).ToList();
    }

    private static Artwork ProcessArtwork(Artwork artwork) => artwork with
    {
        ImageUrls = new ImageUrls(
            ProcessImageUrl(artwork.ImageUrls.SquareMedium),
            ProcessImageUrl(artwork.ImageUrls.Medium),
            ProcessImageUrl(artwork.ImageUrls.Large)),
        MetaPages = [.. artwork.MetaPages.Select(p => p with
        {
            ImageUrls = new ImageUrls(
                ProcessImageUrl(p.ImageUrls.SquareMedium),
                ProcessImageUrl(p.ImageUrls.Medium),
                ProcessImageUrl(p.ImageUrls.Large)),
        })],
    };

    private static string ProcessImageUrl(string url)
    {
        const string pximgPrefix = "https://i.pximg.net/";
        return url.StartsWith(pximgPrefix, StringComparison.Ordinal)
            ? $"/pixiv-proxy/{url[pximgPrefix.Length..]}"
            : url;
    }
}
