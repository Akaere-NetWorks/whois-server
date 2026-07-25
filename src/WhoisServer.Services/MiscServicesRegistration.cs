using WhoisServer.Core.Query;

namespace WhoisServer.Services;

/// <summary>
/// Registers the Phase 5 query suffixes that don't fit neatly into "package registry" or
/// "entertainment" (ICP, PEN, Pixiv) — same pattern as <see cref="NetworkServicesRegistration"/>.
/// </summary>
public static class MiscServicesRegistration
{
    public static void RegisterQueryTypes()
    {
        var help = new List<string> { "% Category: Miscellaneous" };

#if FEATURE_ICP
        QueryAnalyzer.RegisterSuffix("-ICP", baseQuery => new QueryType.Icp(baseQuery));
        help.Add("%   <domain>-ICP             China ICP filing lookup");
#endif
#if FEATURE_PEN
        QueryAnalyzer.RegisterSuffix("-PEN", baseQuery => new QueryType.Pen(baseQuery));
        help.Add("%   <n>-PEN / <name>-PEN     IANA Private Enterprise Number lookup");
#endif
#if FEATURE_PIXIV
        QueryAnalyzer.RegisterSuffix("-PIXIV", baseQuery => new QueryType.Pixiv(baseQuery));
        help.Add("%   <id>-PIXIV               Pixiv artwork proxy info");
#endif

        if (help.Count > 1)
        {
            HelpService.AdditionalSections.Add(string.Join("\n", help));
        }
    }
}
