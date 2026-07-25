namespace WhoisServer.Services;

/// <summary>
/// Built-in HELP response. The Rust version (services/help.rs) hand-builds one large string with
/// every section behind the same <c>#[cfg(feature = "...")]</c> gates as the rest of the codebase,
/// so the text always matches whatever was actually compiled in. This port has no compile-time
/// feature gates to mirror in the same way; each phase should extend
/// <see cref="AdditionalSections"/> with the suffixes it adds, so the response keeps reflecting
/// what this build can actually do without hand-editing one giant string per phase.
/// </summary>
public static class HelpService
{
    /// <summary>Registered by later phases (one entry per feature area) — appended after the
    /// core section in registration order.</summary>
    public static readonly List<string> AdditionalSections = [];

    public static string GenerateResponse()
    {
        var sections = new List<string>
        {
            """
            % Help Information
            %
            % Standard WHOIS Queries
            %   example.com              Domain WHOIS lookup
            %   8.8.8.8                  IPv4 address lookup
            %   2001:4860:4860::8888     IPv6 address lookup
            %   192.0.2.0/24             CIDR block lookup
            %   AS213605                 ASN lookup
            %   213605                   Bare ASN lookup
            %
            % Category: Core
            %   HELP                     Show this help text
            %   UPDATE-PATCH             Refresh response patches from the remote repository
            %   <query>-DESC             Description/remarks-only summary of <query>
            %   <query>-RDAP             Explicit RDAP lookup for <query>
            %
            % Example: whois example.com-DESC
            % Usage: whois <query> or whois <query>-<SUFFIX>
            """,
        };

        sections.AddRange(AdditionalSections);

        return string.Join("\n%\n", sections) + "\n";
    }
}
