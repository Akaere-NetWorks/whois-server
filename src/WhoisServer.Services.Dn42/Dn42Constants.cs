#if FEATURE_DN42
namespace WhoisServer.Services.Dn42;

/// <summary>Ported from src/dn42/git_backend.rs.</summary>
public static class Dn42Constants
{
    public const string RegistryUrl = "https://git.pysio.online/pysio/mirrors-dn42.git";
    public static readonly TimeSpan SyncInterval = TimeSpan.FromHours(1);
}
#endif
