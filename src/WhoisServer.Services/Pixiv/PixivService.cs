using System.Text;

namespace WhoisServer.Services.Pixiv;

/// <summary>
/// Ported from src/services/pixiv/pixiv_impl.rs's <c>process_pixiv_query</c>. Query grammar for
/// the <c>-PIXIV</c> suffix: a bare number is an artwork ID; <c>user:ID</c>, <c>search:keyword</c>,
/// <c>ranking</c>/<c>ranking:mode</c>, and <c>illusts:ID</c> select the other lookup kinds.
/// </summary>
public sealed class PixivService(PixivClient client)
{
    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        try
        {
            if (query.StartsWith("user:", StringComparison.Ordinal))
            {
                return await ProcessUserAsync(query["user:".Length..], ct);
            }

            if (query.StartsWith("search:", StringComparison.Ordinal))
            {
                return await ProcessSearchAsync(query["search:".Length..], ct);
            }

            if (query.StartsWith("ranking", StringComparison.Ordinal))
            {
                var colonIndex = query.IndexOf(':');
                var mode = colonIndex >= 0 && colonIndex + 1 < query.Length ? query[(colonIndex + 1)..] : "day";
                return await ProcessRankingAsync(mode, ct);
            }

            if (query.StartsWith("illusts:", StringComparison.Ordinal))
            {
                return await ProcessUserIllustsAsync(query["illusts:".Length..], ct);
            }

            return await ProcessArtworkAsync(query, ct);
        }
        catch (PixivAuthException e)
        {
            return $"% Pixiv Error: {e.Message}\n";
        }
        catch (PixivRateLimitException)
        {
            return "% Pixiv Error: rate limited, try again later\n";
        }
        catch (Exception e)
        {
            return $"% Error querying Pixiv: {e.Message}\n";
        }
    }

    private async Task<string> ProcessArtworkAsync(string artworkId, CancellationToken ct)
    {
        if (!long.TryParse(artworkId, out var id))
        {
            return $"% Invalid artwork ID: {artworkId}\n";
        }

        var artwork = await client.GetArtworkInfoAsync(id, ct);

        var sb = new StringBuilder();
        sb.AppendLine("PIXIV ARTWORK INFORMATION");
        sb.AppendLine(new string('=', 60));
        sb.AppendLine();
        sb.AppendLine($"Artwork ID:      {artwork.Id}");
        sb.AppendLine($"Title:           {artwork.Title}");
        sb.AppendLine($"Type:            {artwork.ArtworkType}");
        sb.AppendLine($"Artist:          {artwork.User.Name}");
        sb.AppendLine($"Artist ID:       {artwork.User.Id}");
        sb.AppendLine($"Created:         {artwork.CreatedAt:yyyy-MM-dd HH:mm:ss}");
        sb.AppendLine($"Dimensions:      {artwork.Width}x{artwork.Height}");
        sb.AppendLine($"Pages:           {artwork.PageCount}");
        if (artwork.TotalView is not null) sb.AppendLine($"Total Views:     {artwork.TotalView}");
        if (artwork.TotalBookmarks is not null) sb.AppendLine($"Bookmarks:       {artwork.TotalBookmarks}");
        if (artwork.Tags.Count > 0) sb.AppendLine($"Tags:            {string.Join(", ", artwork.Tags.Select(t => t.Name))}");

        sb.AppendLine();
        sb.AppendLine("Image URLs:");
        sb.AppendLine($"  Large:         {artwork.ImageUrls.Large}");
        sb.AppendLine($"  Medium:        {artwork.ImageUrls.Medium}");

        if (artwork.MetaPages.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine($"Multi-page Artwork ({artwork.MetaPages.Count} pages):");
            for (var i = 0; i < artwork.MetaPages.Count; i++)
            {
                sb.AppendLine($"  Page {i + 1}: {artwork.MetaPages[i].ImageUrls.Large}");
            }
        }

        if (!string.IsNullOrEmpty(artwork.Description))
        {
            sb.AppendLine();
            sb.AppendLine($"Caption: {artwork.Description}");
        }

        return sb.ToString();
    }

    private async Task<string> ProcessUserAsync(string userId, CancellationToken ct)
    {
        if (!long.TryParse(userId, out var id))
        {
            return $"% Invalid user ID: {userId}\n";
        }

        var profile = await client.GetUserInfoAsync(id, ct);

        var sb = new StringBuilder();
        sb.AppendLine("PIXIV USER INFORMATION");
        sb.AppendLine(new string('=', 60));
        sb.AppendLine();
        sb.AppendLine($"User ID:         {profile.User.Id}");
        sb.AppendLine($"Name:            {profile.User.Name}");
        sb.AppendLine($"Account:         {profile.User.Account}");
        if (profile.Profile.TotalIllusts is not null) sb.AppendLine($"Total Illusts:   {profile.Profile.TotalIllusts}");
        if (profile.Profile.TotalManga is not null) sb.AppendLine($"Total Manga:     {profile.Profile.TotalManga}");
        if (profile.Profile.TotalNovels is not null) sb.AppendLine($"Total Novels:    {profile.Profile.TotalNovels}");
        if (!string.IsNullOrEmpty(profile.Profile.Webpage)) sb.AppendLine($"Webpage:         {profile.Profile.Webpage}");
        if (!string.IsNullOrEmpty(profile.Profile.TwitterAccount)) sb.AppendLine($"Twitter:         @{profile.Profile.TwitterAccount}");

        return sb.ToString();
    }

    private async Task<string> ProcessSearchAsync(string keyword, CancellationToken ct)
    {
        var results = await client.SearchArtworksAsync(keyword, 10, ct);
        return FormatArtworkList("PIXIV SEARCH RESULTS", $"Keyword:         {keyword}", results);
    }

    private async Task<string> ProcessRankingAsync(string mode, CancellationToken ct)
    {
        var results = await client.GetRankingAsync(mode, 10, ct);
        return FormatArtworkList("PIXIV RANKING", $"Mode:            {mode}", results);
    }

    private async Task<string> ProcessUserIllustsAsync(string userId, CancellationToken ct)
    {
        if (!long.TryParse(userId, out var id))
        {
            return $"% Invalid user ID: {userId}\n";
        }

        var results = await client.GetUserIllustsAsync(id, 10, ct);
        return FormatArtworkList("PIXIV USER ARTWORKS", $"User ID:         {userId}", results);
    }

    private static string FormatArtworkList(string heading, string subheading, List<Artwork> results)
    {
        var sb = new StringBuilder();
        sb.AppendLine(heading);
        sb.AppendLine(new string('=', 60));
        sb.AppendLine();
        sb.AppendLine(subheading);
        sb.AppendLine($"Results:         {results.Count}");
        sb.AppendLine();

        for (var i = 0; i < results.Count; i++)
        {
            var artwork = results[i];
            sb.AppendLine($"{i + 1}. {artwork.Title} (ID: {artwork.Id})");
            sb.AppendLine($"   Artist: {artwork.User.Name}");
            if (artwork.TotalBookmarks is not null) sb.AppendLine($"   Bookmarks: {artwork.TotalBookmarks}");
            sb.AppendLine($"   URL: {artwork.ImageUrls.Large}");
            sb.AppendLine();
        }

        return sb.ToString();
    }
}
