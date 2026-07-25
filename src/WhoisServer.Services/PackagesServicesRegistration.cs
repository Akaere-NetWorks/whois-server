using WhoisServer.Core.Query;

namespace WhoisServer.Services;

/// <summary>
/// Registers all Phase 5 "package registry" query suffixes into <see cref="QueryAnalyzer"/>'s
/// suffix registry — same pattern as <see cref="NetworkServicesRegistration"/>. Each of the 14 is
/// its own atomic Cargo feature upstream (<c>pkg-alma</c>, <c>pkg-npm</c>, ...), so each gets its
/// own <c>FEATURE_PKG_*</c> gate here rather than one umbrella (see Directory.Build.props's
/// Feature_Packages/Feature_Pkg* comment for how the convenience preset expands).
/// </summary>
public static class PackagesServicesRegistration
{
    public static void RegisterQueryTypes()
    {
        var help = new List<string> { "% Category: Package Registries" };

#if FEATURE_PKG_ALMA
        QueryAnalyzer.RegisterSuffix("-ALMA", baseQuery => new QueryType.Alma(baseQuery));
        help.Add("%   <pkg>-ALMA               AlmaLinux package info");
#endif
#if FEATURE_PKG_AOSC
        QueryAnalyzer.RegisterSuffix("-AOSC", baseQuery => new QueryType.Aosc(baseQuery));
        help.Add("%   <pkg>-AOSC               AOSC OS package info");
#endif
#if FEATURE_PKG_AUR
        QueryAnalyzer.RegisterSuffix("-AUR", baseQuery => new QueryType.Aur(baseQuery));
        help.Add("%   <pkg>-AUR                Arch User Repository package info");
#endif
#if FEATURE_PKG_CARGO
        QueryAnalyzer.RegisterSuffix("-CARGO", baseQuery => new QueryType.Cargo(baseQuery));
        help.Add("%   <crate>-CARGO            crates.io package info");
#endif
#if FEATURE_PKG_CURSEFORGE
        QueryAnalyzer.RegisterSuffix("-CURSEFORGE", baseQuery => new QueryType.CurseForge(baseQuery));
        help.Add("%   <mod>-CURSEFORGE         CurseForge project info");
#endif
#if FEATURE_PKG_DEBIAN
        QueryAnalyzer.RegisterSuffix("-DEBIAN", baseQuery => new QueryType.Debian(baseQuery));
        help.Add("%   <pkg>-DEBIAN             Debian package info");
#endif
#if FEATURE_PKG_EPEL
        QueryAnalyzer.RegisterSuffix("-EPEL", baseQuery => new QueryType.Epel(baseQuery));
        help.Add("%   <pkg>-EPEL               EPEL package info");
#endif
#if FEATURE_PKG_MODRINTH
        QueryAnalyzer.RegisterSuffix("-MODRINTH", baseQuery => new QueryType.Modrinth(baseQuery));
        help.Add("%   <mod>-MODRINTH           Modrinth project info");
#endif
#if FEATURE_PKG_NIXOS
        QueryAnalyzer.RegisterSuffix("-NIXOS", baseQuery => new QueryType.NixOs(baseQuery));
        help.Add("%   <pkg>-NIXOS              NixOS package info");
#endif
#if FEATURE_PKG_NPM
        QueryAnalyzer.RegisterSuffix("-NPM", baseQuery => new QueryType.Npm(baseQuery));
        help.Add("%   <pkg>-NPM                npm package info");
#endif
#if FEATURE_PKG_OPENSUSE
        QueryAnalyzer.RegisterSuffix("-OPENSUSE", baseQuery => new QueryType.OpenSuse(baseQuery));
        help.Add("%   <pkg>-OPENSUSE           openSUSE package info");
#endif
#if FEATURE_PKG_OPENWRT
        QueryAnalyzer.RegisterSuffix("-OPENWRT", baseQuery => new QueryType.OpenWrt(baseQuery));
        help.Add("%   <pkg>-OPENWRT            OpenWrt package info");
#endif
#if FEATURE_PKG_PYPI
        QueryAnalyzer.RegisterSuffix("-PYPI", baseQuery => new QueryType.Pypi(baseQuery));
        help.Add("%   <pkg>-PYPI               PyPI package info");
#endif
#if FEATURE_PKG_UBUNTU
        QueryAnalyzer.RegisterSuffix("-UBUNTU", baseQuery => new QueryType.Ubuntu(baseQuery));
        help.Add("%   <pkg>-UBUNTU             Ubuntu package info");
#endif

        if (help.Count > 1)
        {
            HelpService.AdditionalSections.Add(string.Join("\n", help));
        }
    }
}
