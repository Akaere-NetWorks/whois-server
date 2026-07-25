using System.Text.Json.Serialization;

namespace WhoisServer.Services.Pixiv;

/// <summary>Data models ported from src/services/pixiv/models.rs — only the fields this port's
/// text formatting actually uses are kept (not a 1:1 field-for-field port).</summary>
public sealed record PixivResponse<T>([property: JsonPropertyName("body")] T? Body);

public sealed record ImageUrls(
    [property: JsonPropertyName("square_medium")] string SquareMedium,
    [property: JsonPropertyName("medium")] string Medium,
    [property: JsonPropertyName("large")] string Large);

public sealed record PixivTag(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("translated_name")] string? TranslatedName);

public sealed record PixivUser(
    [property: JsonPropertyName("id")] long Id,
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("account")] string Account);

public sealed record MetaSinglePage([property: JsonPropertyName("original_image_url")] string? OriginalImageUrl);

public sealed record MetaPage([property: JsonPropertyName("image_urls")] ImageUrls ImageUrls);

public sealed record Artwork(
    [property: JsonPropertyName("id")] long Id,
    [property: JsonPropertyName("title")] string Title,
    [property: JsonPropertyName("type")] string ArtworkType,
    [property: JsonPropertyName("image_urls")] ImageUrls ImageUrls,
    [property: JsonPropertyName("caption")] string? Description,
    [property: JsonPropertyName("user")] PixivUser User,
    [property: JsonPropertyName("tags")] List<PixivTag> Tags,
    [property: JsonPropertyName("create_date")] DateTimeOffset CreatedAt,
    [property: JsonPropertyName("page_count")] int PageCount,
    [property: JsonPropertyName("width")] int Width,
    [property: JsonPropertyName("height")] int Height,
    [property: JsonPropertyName("meta_single_page")] MetaSinglePage MetaSinglePage,
    [property: JsonPropertyName("meta_pages")] List<MetaPage> MetaPages,
    [property: JsonPropertyName("total_view")] int? TotalView,
    [property: JsonPropertyName("total_bookmarks")] int? TotalBookmarks);

public sealed record Profile(
    [property: JsonPropertyName("webpage")] string? Webpage,
    [property: JsonPropertyName("region")] string? Region,
    [property: JsonPropertyName("total_illusts")] int? TotalIllusts,
    [property: JsonPropertyName("total_manga")] int? TotalManga,
    [property: JsonPropertyName("total_novels")] int? TotalNovels,
    [property: JsonPropertyName("total_follow_users")] int? TotalFollowUsers,
    [property: JsonPropertyName("twitter_account")] string? TwitterAccount);

public sealed record UserProfile(
    [property: JsonPropertyName("user")] PixivUser User,
    [property: JsonPropertyName("profile")] Profile Profile);

public sealed record SearchResults(
    [property: JsonPropertyName("illusts")] List<Artwork> Illusts,
    [property: JsonPropertyName("manga")] List<Artwork> Manga);

public sealed record UserArtworks(
    [property: JsonPropertyName("illusts")] List<Artwork> Illusts,
    [property: JsonPropertyName("manga")] List<Artwork> Manga);

public sealed record RankingItem([property: JsonPropertyName("illust_id")] long IllustId);

public sealed record RankingResults([property: JsonPropertyName("contents")] List<RankingItem> Contents);
