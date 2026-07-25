namespace WhoisServer.Host;

/// <summary>Mirrors src/config.rs's <c>Cli</c> struct, minus every SSH-related flag (the SSH
/// server is not being ported — plan.md's locked-in decision).</summary>
public sealed record CliOptions
{
    public string Host { get; init; } = "0.0.0.0";
    public int Port { get; init; } = 43;
    public bool Debug { get; init; }
    public bool Trace { get; init; }
    public int MaxConnections { get; init; } = 100;
    public int TimeoutSeconds { get; init; } = 10;
    public bool DumpTraffic { get; init; }
    public string DumpDir { get; init; } = "dumps";
    public int WebPort { get; init; } = 9999;
    public bool EnableColor { get; init; } = true;
}
