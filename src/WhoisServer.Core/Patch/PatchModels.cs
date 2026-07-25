using System.Text.Json.Serialization;

namespace WhoisServer.Core.Patch;

/// <summary>One replacement operation within a patch file.</summary>
public sealed record DiffHunk(IReadOnlyList<string> RemoveLines, IReadOnlyList<string> AddLines);

public enum ContextDirection
{
    Before,
    After,
}

public enum ContextAction
{
    Skip,
    Only,
}

/// <summary>SKIP_AFTER / SKIP_BEFORE / ONLY_AFTER / ONLY_BEFORE directive.</summary>
public sealed record ContextRule(string Pattern, ContextDirection Direction, int Lines, ContextAction Action);

public enum ConditionType
{
    QueryContains,
    ResponseContains,
    QueryMatches,
    ResponseMatches,
}

public sealed record PatchCondition(ConditionType ConditionType, string Value, System.Text.RegularExpressions.Regex? Regex);

/// <summary>A complete patch: conditions (OR'd) gate whether it applies at all, then each hunk is
/// applied in order, subject to <see cref="Excludes"/> and <see cref="ContextRules"/>.
/// Named <c>PatchRule</c> rather than <c>Patch</c> to avoid colliding with the
/// <c>WhoisServer.Core.Patch</c> namespace itself (C# namespace/type names share one
/// case-sensitive symbol table, unlike Rust's separate module vs. type namespaces).</summary>
public sealed record PatchRule(
    IReadOnlyList<PatchCondition> Conditions,
    IReadOnlyList<string> Excludes,
    IReadOnlyList<ContextRule> ContextRules,
    IReadOnlyList<DiffHunk> Hunks);

public sealed record PatchFile(string Filename, IReadOnlyList<PatchRule> Patches);

public sealed record PatchInfo(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("description")] string Description,
    [property: JsonPropertyName("url")] string Url,
    [property: JsonPropertyName("sha1")] string Sha1,
    [property: JsonPropertyName("size")] long Size,
    [property: JsonPropertyName("priority")] int Priority,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("modified")] string Modified);

public sealed record MetadataInfo(
    [property: JsonPropertyName("format_version")] string FormatVersion,
    [property: JsonPropertyName("update_url")] string UpdateUrl,
    [property: JsonPropertyName("checksum_algorithm")] string? ChecksumAlgorithm,
    [property: JsonPropertyName("documentation")] string? Documentation);

public sealed record PatchMetadata(
    [property: JsonPropertyName("version")] string Version,
    [property: JsonPropertyName("last_updated")] string LastUpdated,
    [property: JsonPropertyName("repository")] string Repository,
    [property: JsonPropertyName("patches")] IReadOnlyList<PatchInfo> Patches,
    [property: JsonPropertyName("metadata")] MetadataInfo Metadata);
