using WhoisServer.Core.Query;

namespace WhoisServer.Services;

/// <summary>
/// Registers all Phase 5 "entertainment" query suffixes into <see cref="QueryAnalyzer"/>'s suffix
/// registry — same pattern as <see cref="NetworkServicesRegistration"/>. Meal/MealCN are NOT here:
/// they have no base query to carry (trigger is an exact Chinese phrase or a bare suffix), so
/// they're special-cased directly inside <see cref="QueryAnalyzer.Analyze"/> instead (gated there
/// by <c>FEATURE_MEAL</c> directly).
/// </summary>
public static class EntertainmentServicesRegistration
{
    public static void RegisterQueryTypes()
    {
        var help = new List<string> { "% Category: Entertainment" };

#if FEATURE_STEAM
        QueryAnalyzer.RegisterSuffix("-STEAMSEARCH", baseQuery => new QueryType.SteamSearch(baseQuery));
        QueryAnalyzer.RegisterSuffix("-STEAM", baseQuery => new QueryType.Steam(baseQuery));
        help.Add("%   <name>-STEAMSEARCH       Steam app/user search");
        help.Add("%   <id>-STEAM               Steam app/user info");
#endif
#if FEATURE_IMDB
        QueryAnalyzer.RegisterSuffix("-IMDBSEARCH", baseQuery => new QueryType.ImdbSearch(baseQuery));
        QueryAnalyzer.RegisterSuffix("-IMDB", baseQuery => new QueryType.Imdb(baseQuery));
        help.Add("%   <title>-IMDBSEARCH       IMDb title search");
        help.Add("%   <id>-IMDB                IMDb title info");
#endif
#if FEATURE_MINECRAFT
        QueryAnalyzer.RegisterSuffix("-MINECRAFT", baseQuery => new QueryType.Minecraft(baseQuery));
        QueryAnalyzer.RegisterSuffix("-MCU", baseQuery => new QueryType.MinecraftUser(baseQuery));
        QueryAnalyzer.RegisterSuffix("-MC", baseQuery => new QueryType.Minecraft(baseQuery));
        help.Add("%   <host>-MINECRAFT/-MC     Minecraft server status");
        help.Add("%   <username>-MCU           Minecraft user profile");
#endif
#if FEATURE_ACGC
        QueryAnalyzer.RegisterSuffix("-ACGC", baseQuery => new QueryType.Acgc(baseQuery));
        help.Add("%   <character>-ACGC        Anime/game character lookup");
#endif
#if FEATURE_LYRIC
        QueryAnalyzer.RegisterSuffix("-LYRIC", baseQuery => new QueryType.Lyric(baseQuery));
        help.Add("%   <title>-LYRIC           Song lyric search");
#endif
#if FEATURE_WIKIPEDIA
        QueryAnalyzer.RegisterSuffix("-WIKIPEDIA", baseQuery => new QueryType.Wikipedia(baseQuery));
        help.Add("%   <topic>-WIKIPEDIA       Wikipedia article summary");
#endif
#if FEATURE_GITHUB
        QueryAnalyzer.RegisterSuffix("-GITHUB", baseQuery => new QueryType.GitHub(baseQuery));
        help.Add("%   <user/repo>-GITHUB      GitHub user/repository info");
#endif
#if FEATURE_MEAL
        help.Add("%   吃什么 / MEALCN          Random meal suggestion (Chinese)");
#endif

        if (help.Count > 1)
        {
            HelpService.AdditionalSections.Add(string.Join("\n", help));
        }
    }
}
