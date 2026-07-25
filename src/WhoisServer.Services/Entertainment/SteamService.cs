using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/steam.rs — the <c>-STEAM</c> (game or user lookup) and
/// <c>-STEAMSEARCH</c> (fuzzy game search) suffixes. Games are looked up via the public (unofficial)
/// Steam Store API; user profiles require a Steam Web API key (<c>STEAM_API_KEY</c>) and degrade
/// gracefully with a configuration message when none is supplied, matching the Rust version's own
/// behavior rather than throwing.
/// </summary>
public sealed class SteamService(HttpClient httpClient, string? apiKey)
{
    private const string UserAgent = "WhoisServer/1.0 Steam API Client";

    /// <summary>Handles the <c>-STEAM</c> suffix. <paramref name="baseQuery"/> is already
    /// suffix-stripped. Numeric values under 10 million are treated as Steam App IDs (mirroring the
    /// Rust heuristic — real Steam user IDs are 17-digit numbers); everything else is treated as a
    /// user ID/vanity name.</summary>
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid Steam query format. Use: <app_id>-STEAM or <steam_id>-STEAM\n";
        }

        if (IsLikelyAppId(baseQuery) && uint.TryParse(baseQuery, out var appId))
        {
            return await QueryAppInfoAsync(appId, ct);
        }

        return await QueryUserInfoAsync(baseQuery, ct);
    }

    /// <summary>Handles the <c>-STEAMSEARCH</c> suffix: fuzzy game-name search, limited to 10 results.
    /// Tries the Steam Store's unofficial search endpoint first (richer results: price, platforms,
    /// metascore), falling back to a full app-list substring/prefix scan if that fails.</summary>
    public async Task<string> ProcessSearchAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid Steam search query. Please provide a search term.\nExample: Counter-Strike-STEAMSEARCH\n";
        }

        try
        {
            var storeResults = await SearchViaStoreApiAsync(baseQuery, 10, ct);
            if (storeResults is not null)
            {
                return storeResults;
            }
        }
        catch
        {
            // fall through to app-list search
        }

        return await SearchViaAppListAsync(baseQuery, 10, ct);
    }

    private static bool IsLikelyAppId(string query) =>
        ulong.TryParse(query, out var num) && num < 10_000_000;

    private async Task<string> QueryAppInfoAsync(uint appId, CancellationToken ct)
    {
        var url = $"https://store.steampowered.com/api/appdetails?appids={appId}&l=english";

        HttpResponseMessage response;
        try
        {
            response = await httpClient.SendAsync(CreateRequest(url), ct);
        }
        catch (Exception e)
        {
            return $"Steam App Query Failed for ID: {appId}\nError: {e.Message}\n";
        }

        using (response)
        {
            if (!response.IsSuccessStatusCode)
            {
                return $"Steam App Query Failed for ID: {appId}\nHTTP Status: {(int)response.StatusCode}\n";
            }

            Dictionary<string, SteamAppDetails>? parsed;
            try
            {
                parsed = await response.Content.ReadFromJsonAsync<Dictionary<string, SteamAppDetails>>(cancellationToken: ct);
            }
            catch (Exception e)
            {
                return $"Steam App Query Failed for ID: {appId}\nData parsing error: {e.Message}\n";
            }

            if (parsed is null || !parsed.TryGetValue(appId.ToString(), out var details))
            {
                return $"Steam App Not Found for ID: {appId}\nNo data returned from Steam API.\n";
            }

            if (!details.Success || details.Data is null)
            {
                return $"Steam App Not Found for ID: {appId}\nThe application may not exist or may be private.\n";
            }

            return FormatAppInfo(details.Data);
        }
    }

    private async Task<string> QueryUserInfoAsync(string steamId, CancellationToken ct)
    {
        if (string.IsNullOrEmpty(apiKey))
        {
            return $"""
                Steam User Query Failed for ID: {steamId}
                Steam API key not configured.
                To enable user profile queries, set the STEAM_API_KEY environment variable.
                You can get an API key from: https://steamcommunity.com/dev/apikey

                """;
        }

        var url = $"https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={apiKey}&steamids={Uri.EscapeDataString(steamId)}";

        HttpResponseMessage response;
        try
        {
            response = await httpClient.SendAsync(CreateRequest(url), ct);
        }
        catch (Exception e)
        {
            return $"Steam User Query Failed for ID: {steamId}\nError: {e.Message}\n";
        }

        using (response)
        {
            if (!response.IsSuccessStatusCode)
            {
                return $"Steam User Query Failed for ID: {steamId}\nHTTP Status: {(int)response.StatusCode}\n";
            }

            SteamUserResponse? userResponse;
            try
            {
                userResponse = await response.Content.ReadFromJsonAsync<SteamUserResponse>(cancellationToken: ct);
            }
            catch (Exception e)
            {
                return $"Steam User Query Failed for ID: {steamId}\nData parsing error: {e.Message}\n";
            }

            var profile = userResponse?.Response?.Players?.FirstOrDefault();
            if (profile is null)
            {
                return $"Steam User Not Found for ID: {steamId}\nProfile may not exist or may be private.\n";
            }

            return FormatUserInfo(profile);
        }
    }

    /// <summary>Returns null (rather than throwing) when the store search API's response shape
    /// doesn't contain an "items" array, signaling the caller to fall back to the app-list search —
    /// mirrors the Rust version's Result-based fallback chain.</summary>
    private async Task<string?> SearchViaStoreApiAsync(string query, int limit, CancellationToken ct)
    {
        var url = $"https://store.steampowered.com/api/storesearch/?term={Uri.EscapeDataString(query)}&l=english&cc=US";
        using var response = await httpClient.SendAsync(CreateRequest(url), ct);
        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(await response.Content.ReadAsStreamAsync(ct), cancellationToken: ct);
        if (!doc.RootElement.TryGetProperty("items", out var items) || items.ValueKind != JsonValueKind.Array)
        {
            return null;
        }

        var results = new List<(uint AppId, string Name, string Type, string? Price, string Platforms, bool ComingSoon)>();
        foreach (var item in items.EnumerateArray().Take(limit))
        {
            if (!item.TryGetProperty("id", out var idProp) || !item.TryGetProperty("name", out var nameProp))
            {
                continue;
            }

            var appId = (uint)idProp.GetInt64();
            var name = nameProp.GetString() ?? "";
            var type = item.TryGetProperty("type", out var typeProp) ? typeProp.GetString() ?? "game" : "game";
            var comingSoon = item.TryGetProperty("coming_soon", out var csProp) && csProp.ValueKind == JsonValueKind.True;
            var price = ExtractPrice(item);
            var platforms = ExtractPlatforms(item);

            results.Add((appId, name, type, price, platforms, comingSoon));
        }

        if (results.Count == 0)
        {
            return $"No Steam games found matching: {query}\n";
        }

        return FormatSearchResults(query, results);
    }

    private async Task<string> SearchViaAppListAsync(string query, int limit, CancellationToken ct)
    {
        const string url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";

        using var response = await httpClient.SendAsync(CreateRequest(url), ct);
        if (!response.IsSuccessStatusCode)
        {
            return $"No Steam games found matching: {query}\n";
        }

        var appList = await response.Content.ReadFromJsonAsync<SteamAppListResponse>(cancellationToken: ct);
        var apps = appList?.AppList?.Apps ?? [];

        var queryLower = query.ToLowerInvariant();
        var matches = new List<(uint AppId, string Name, int Score)>();

        foreach (var app in apps)
        {
            var nameLower = app.Name.ToLowerInvariant();
            var score = nameLower == queryLower ? 100
                : nameLower.StartsWith(queryLower, StringComparison.Ordinal) ? 50
                : nameLower.Contains(queryLower, StringComparison.Ordinal) ? 25
                : 0;

            if (score > 0)
            {
                matches.Add(((uint)app.AppId, app.Name, score));
            }
        }

        var top = matches.OrderByDescending(m => m.Score).Take(limit).ToList();
        if (top.Count == 0)
        {
            return $"No Steam games found matching: {query}\n";
        }

        var results = top.Select(m => (m.AppId, m.Name, "app", (string?)null, "N/A", false)).ToList();
        return FormatSearchResults(query, results);
    }

    private static string? ExtractPrice(JsonElement item)
    {
        if (!item.TryGetProperty("price", out var price) || price.ValueKind != JsonValueKind.Object)
        {
            return null;
        }

        var currency = price.TryGetProperty("currency", out var c) ? c.GetString() : null;
        var final = price.TryGetProperty("final", out var f) && f.ValueKind == JsonValueKind.Number ? f.GetInt64() : (long?)null;

        if (currency == "USD" && final == 0)
        {
            return "Free";
        }

        var finalFormatted = price.TryGetProperty("final_formatted", out var ff) ? ff.GetString() : null;
        if (finalFormatted is null)
        {
            return null;
        }

        var discountPercent = price.TryGetProperty("discount_percent", out var dp) && dp.ValueKind == JsonValueKind.Number ? dp.GetInt64() : 0;
        return discountPercent > 0 ? $"{finalFormatted} ({discountPercent}%↓)" : finalFormatted;
    }

    private static string ExtractPlatforms(JsonElement item)
    {
        if (!item.TryGetProperty("platforms", out var platforms) || platforms.ValueKind != JsonValueKind.Object)
        {
            return "N/A";
        }

        var list = new List<string>();
        if (platforms.TryGetProperty("windows", out var w) && w.ValueKind == JsonValueKind.True)
        {
            list.Add("Windows");
        }

        if (platforms.TryGetProperty("mac", out var m) && m.ValueKind == JsonValueKind.True)
        {
            list.Add("macOS");
        }

        if (platforms.TryGetProperty("linux", out var l) && l.ValueKind == JsonValueKind.True)
        {
            list.Add("Linux");
        }

        return list.Count == 0 ? "N/A" : string.Join(", ", list);
    }

    private static string FormatSearchResults(
        string query, List<(uint AppId, string Name, string Type, string? Price, string Platforms, bool ComingSoon)> results)
    {
        var sb = new StringBuilder();
        sb.Append($"Steam Game Search Results for: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"Found {results.Count} games:\n\n");

        for (var i = 0; i < results.Count; i++)
        {
            var (appId, name, type, price, platforms, comingSoon) = results[i];
            sb.Append($"{i + 1}. Game Information\n");
            sb.Append(new string('-', 25)).Append('\n');
            sb.Append($"app-id: {appId}\n");
            sb.Append($"name: {name}\n");
            sb.Append($"type: {type}\n");

            if (price is not null)
            {
                sb.Append($"price: {price}\n");
            }

            sb.Append($"platforms: {platforms}\n");

            if (comingSoon)
            {
                sb.Append("status: Coming Soon\n");
            }

            sb.Append($"steam-url: https://store.steampowered.com/app/{appId}/\n\n");
        }

        sb.Append($"% Use '{results[0].AppId}-STEAM' to get detailed information for a specific game\n");
        sb.Append("% Search limited to top 10 results\n");
        return sb.ToString();
    }

    private static string FormatAppInfo(SteamAppData app)
    {
        var sb = new StringBuilder();
        sb.Append($"Steam Application Information for ID: {app.SteamAppId}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"app-id: {app.SteamAppId}\n");
        sb.Append($"name: {app.Name}\n");
        sb.Append($"type: {app.Type}\n");
        sb.Append($"is-free: {app.IsFree}\n");

        if (app.Developers is { Count: > 0 })
        {
            sb.Append($"developers: {string.Join(", ", app.Developers)}\n");
        }

        if (app.Publishers is { Count: > 0 })
        {
            sb.Append($"publishers: {string.Join(", ", app.Publishers)}\n");
        }

        if (app.ReleaseDate is { } releaseDate)
        {
            sb.Append($"release-date: {releaseDate.Date}\n");
            sb.Append($"coming-soon: {releaseDate.ComingSoon}\n");
        }

        if (app.Platforms is { } platforms)
        {
            var list = new List<string>();
            if (platforms.Windows) list.Add("Windows");
            if (platforms.Mac) list.Add("macOS");
            if (platforms.Linux) list.Add("Linux");
            sb.Append($"platforms: {string.Join(", ", list)}\n");
        }

        if (app.PriceOverview is { } price)
        {
            sb.Append(price.DiscountPercent > 0
                ? $"price: {price.FinalFormatted} ({price.DiscountPercent}%↓)\noriginal-price: {price.InitialFormatted}\n"
                : $"price: {price.FinalFormatted}\n");
            sb.Append($"currency: {price.Currency}\n");
        }

        if (app.Metacritic is { } metacritic)
        {
            sb.Append($"metacritic-score: {metacritic.Score}\n");
            sb.Append($"metacritic-url: {metacritic.Url}\n");
        }

        if (app.Recommendations is { } recommendations)
        {
            sb.Append($"recommendations: {recommendations.Total}\n");
        }

        if (app.Achievements is { } achievements)
        {
            sb.Append($"achievements: {achievements.Total}\n");
        }

        if (app.Website is { Length: > 0 } website)
        {
            sb.Append($"website: {website}\n");
        }

        if (app.Genres is { Count: > 0 })
        {
            sb.Append($"genres: {string.Join(", ", app.Genres.Select(g => g.Description))}\n");
        }

        if (app.Categories is { Count: > 0 })
        {
            sb.Append($"categories: {string.Join(", ", app.Categories.Select(c => c.Description))}\n");
        }

        if (app.SupportedLanguages is { Length: > 0 } languages)
        {
            var cleaned = languages.Replace("<br>", ", ").Replace("<strong>", "").Replace("</strong>", "");
            sb.Append($"supported-languages: {cleaned}\n");
        }

        if (app.ShortDescription is { Length: > 0 } description)
        {
            sb.Append($"description: {description.Replace("\r\n", " ").Replace('\n', ' ')}\n");
        }

        sb.Append($"steam-url: https://store.steampowered.com/app/{app.SteamAppId}/\n");
        return sb.ToString();
    }

    private static string FormatUserInfo(SteamUserProfile profile)
    {
        var sb = new StringBuilder();
        sb.Append($"Steam User Profile Information for ID: {profile.SteamId}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"steamid: {profile.SteamId}\n");
        sb.Append($"personaname: {profile.PersonaName}\n");

        if (profile.RealName is { Length: > 0 } realName)
        {
            sb.Append($"realname: {realName}\n");
        }

        sb.Append($"profileurl: {profile.ProfileUrl}\n");

        var visibility = profile.CommunityVisibilityState switch
        {
            1 => "Private",
            3 => "Friends Only",
            _ => "Public",
        };
        sb.Append($"visibility: {visibility}\n");

        var profileState = profile.ProfileState switch
        {
            0 => "Not Configured",
            1 => "Configured",
            _ => "Unknown",
        };
        sb.Append($"profile-state: {profileState}\n");

        var personaState = profile.PersonaState switch
        {
            0 => "Offline",
            1 => "Online",
            2 => "Busy",
            3 => "Away",
            4 => "Snooze",
            5 => "Looking to trade",
            6 => "Looking to play",
            _ => "Unknown",
        };
        sb.Append($"status: {personaState}\n");

        if (profile.TimeCreated is { } created)
        {
            var dt = DateTimeOffset.FromUnixTimeSeconds(created).UtcDateTime;
            sb.Append($"created: {dt:yyyy-MM-dd HH:mm:ss} UTC ({created})\n");
        }

        if (profile.LocCountryCode is { Length: > 0 } country)
        {
            sb.Append($"country: {country}\n");
        }

        if (profile.PrimaryClanId is { Length: > 0 } clanId)
        {
            sb.Append($"primary-clan-id: {clanId}\n");
        }

        sb.Append($"avatar: {profile.Avatar}\n");
        sb.Append($"avatar-medium: {profile.AvatarMedium}\n");
        sb.Append($"avatar-full: {profile.AvatarFull}\n");
        return sb.ToString();
    }

    private static HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);
        return request;
    }

    private sealed record SteamAppDetails(
        [property: JsonPropertyName("success")] bool Success,
        [property: JsonPropertyName("data")] SteamAppData? Data);

    private sealed record SteamAppData(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("steam_appid")] uint SteamAppId,
        [property: JsonPropertyName("type")] string Type,
        [property: JsonPropertyName("is_free")] bool IsFree,
        [property: JsonPropertyName("short_description")] string? ShortDescription,
        [property: JsonPropertyName("supported_languages")] string? SupportedLanguages,
        [property: JsonPropertyName("developers")] List<string>? Developers,
        [property: JsonPropertyName("publishers")] List<string>? Publishers,
        [property: JsonPropertyName("platforms")] SteamPlatforms? Platforms,
        [property: JsonPropertyName("categories")] List<SteamCategory>? Categories,
        [property: JsonPropertyName("genres")] List<SteamGenre>? Genres,
        [property: JsonPropertyName("release_date")] SteamReleaseDate? ReleaseDate,
        [property: JsonPropertyName("price_overview")] SteamPriceOverview? PriceOverview,
        [property: JsonPropertyName("website")] string? Website,
        [property: JsonPropertyName("metacritic")] SteamMetacritic? Metacritic,
        [property: JsonPropertyName("recommendations")] SteamRecommendations? Recommendations,
        [property: JsonPropertyName("achievements")] SteamAchievements? Achievements);

    private sealed record SteamPlatforms(
        [property: JsonPropertyName("windows")] bool Windows,
        [property: JsonPropertyName("mac")] bool Mac,
        [property: JsonPropertyName("linux")] bool Linux);

    private sealed record SteamCategory([property: JsonPropertyName("description")] string Description);

    private sealed record SteamGenre([property: JsonPropertyName("description")] string Description);

    private sealed record SteamReleaseDate(
        [property: JsonPropertyName("coming_soon")] bool ComingSoon,
        [property: JsonPropertyName("date")] string Date);

    private sealed record SteamPriceOverview(
        [property: JsonPropertyName("currency")] string Currency,
        [property: JsonPropertyName("initial_formatted")] string InitialFormatted,
        [property: JsonPropertyName("final_formatted")] string FinalFormatted,
        [property: JsonPropertyName("discount_percent")] int DiscountPercent);

    private sealed record SteamMetacritic(
        [property: JsonPropertyName("score")] int Score,
        [property: JsonPropertyName("url")] string Url);

    private sealed record SteamRecommendations([property: JsonPropertyName("total")] int Total);

    private sealed record SteamAchievements([property: JsonPropertyName("total")] int Total);

    private sealed record SteamAppListResponse([property: JsonPropertyName("applist")] SteamAppList? AppList);

    private sealed record SteamAppList([property: JsonPropertyName("apps")] List<SteamAppListItem>? Apps);

    private sealed record SteamAppListItem(
        [property: JsonPropertyName("appid")] long AppId,
        [property: JsonPropertyName("name")] string Name);

    private sealed record SteamUserResponse([property: JsonPropertyName("response")] SteamUserResponseData? Response);

    private sealed record SteamUserResponseData([property: JsonPropertyName("players")] List<SteamUserProfile>? Players);

    private sealed record SteamUserProfile(
        [property: JsonPropertyName("steamid")] string SteamId,
        [property: JsonPropertyName("communityvisibilitystate")] int CommunityVisibilityState,
        [property: JsonPropertyName("profilestate")] int ProfileState,
        [property: JsonPropertyName("personaname")] string PersonaName,
        [property: JsonPropertyName("profileurl")] string ProfileUrl,
        [property: JsonPropertyName("avatar")] string Avatar,
        [property: JsonPropertyName("avatarmedium")] string AvatarMedium,
        [property: JsonPropertyName("avatarfull")] string AvatarFull,
        [property: JsonPropertyName("personastate")] int PersonaState,
        [property: JsonPropertyName("realname")] string? RealName,
        [property: JsonPropertyName("primaryclanid")] string? PrimaryClanId,
        [property: JsonPropertyName("timecreated")] long? TimeCreated,
        [property: JsonPropertyName("loccountrycode")] string? LocCountryCode);
}
