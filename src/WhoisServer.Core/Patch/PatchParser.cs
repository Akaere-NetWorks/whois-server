using System.Text.RegularExpressions;

namespace WhoisServer.Core.Patch;

/// <summary>
/// Parses the bespoke pseudo-unified-diff patch DSL described in plan.md §7.1 — ported from
/// core/patch.rs's <c>parse_patch_content</c>/<c>parse_diff_hunk</c>. All directives across one
/// file accumulate into a single <see cref="Patch"/> (conditions/excludes/context-rules are
/// shared across every hunk in that file) — this is a direct port of the Rust behavior, not
/// a design choice made here.
/// </summary>
public static class PatchParser
{
    public static PatchFile ParsePatchContent(string filename, string content)
    {
        var lines = content.Replace("\r\n", "\n").Split('\n');
        var conditions = new List<PatchCondition>();
        var excludes = new List<string>();
        var contextRules = new List<ContextRule>();
        var hunks = new List<DiffHunk>();

        var i = 0;
        while (i < lines.Length)
        {
            var line = lines[i].Trim();

            if (line.Length == 0)
            {
                i++;
                continue;
            }

            if (line.StartsWith("# EXCLUDE:", StringComparison.Ordinal))
            {
                excludes.Add(line["# EXCLUDE:".Length..].Trim());
                i++;
                continue;
            }

            if (TryParseContextDirective(line, "# SKIP_AFTER:", ContextDirection.After, ContextAction.Skip, out var skipAfter))
            {
                contextRules.Add(skipAfter);
                i++;
                continue;
            }

            if (TryParseContextDirective(line, "# SKIP_BEFORE:", ContextDirection.Before, ContextAction.Skip, out var skipBefore))
            {
                contextRules.Add(skipBefore);
                i++;
                continue;
            }

            if (TryParseContextDirective(line, "# ONLY_AFTER:", ContextDirection.After, ContextAction.Only, out var onlyAfter))
            {
                contextRules.Add(onlyAfter);
                i++;
                continue;
            }

            if (TryParseContextDirective(line, "# ONLY_BEFORE:", ContextDirection.Before, ContextAction.Only, out var onlyBefore))
            {
                contextRules.Add(onlyBefore);
                i++;
                continue;
            }

            if (line.StartsWith("# QUERY_CONTAINS:", StringComparison.Ordinal))
            {
                conditions.Add(new PatchCondition(ConditionType.QueryContains, line["# QUERY_CONTAINS:".Length..].Trim(), null));
                i++;
                continue;
            }

            if (line.StartsWith("# RESPONSE_CONTAINS:", StringComparison.Ordinal))
            {
                conditions.Add(new PatchCondition(ConditionType.ResponseContains, line["# RESPONSE_CONTAINS:".Length..].Trim(), null));
                i++;
                continue;
            }

            if (line.StartsWith("# QUERY_MATCHES:", StringComparison.Ordinal))
            {
                var pattern = line["# QUERY_MATCHES:".Length..].Trim();
                conditions.Add(new PatchCondition(ConditionType.QueryMatches, pattern, new Regex(pattern)));
                i++;
                continue;
            }

            if (line.StartsWith("# RESPONSE_MATCHES:", StringComparison.Ordinal))
            {
                var pattern = line["# RESPONSE_MATCHES:".Length..].Trim();
                conditions.Add(new PatchCondition(ConditionType.ResponseMatches, pattern, new Regex(pattern)));
                i++;
                continue;
            }

            if (line.StartsWith('#'))
            {
                i++;
                continue;
            }

            if (line.StartsWith("---", StringComparison.Ordinal))
            {
                var hunk = ParseDiffHunk(lines, ref i);
                if (hunk is not null)
                {
                    hunks.Add(hunk);
                }

                continue;
            }

            i++;
        }

        var patches = new List<PatchRule>();
        if (hunks.Count > 0)
        {
            patches.Add(new PatchRule(conditions, excludes, contextRules, hunks));
        }

        return new PatchFile(filename, patches);
    }

    private static bool TryParseContextDirective(
        string line, string prefix, ContextDirection direction, ContextAction action, out ContextRule rule)
    {
        rule = null!;
        if (!line.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        var @params = line[prefix.Length..].Trim();
        var commaIndex = @params.IndexOf(',');
        if (commaIndex >= 0)
        {
            var pattern = @params[..commaIndex].Trim();
            var linesStr = @params[(commaIndex + 1)..].Trim();
            var lineCount = int.TryParse(linesStr, out var n) ? n : 10;
            rule = new ContextRule(pattern, direction, lineCount, action);
        }
        else if (action == ContextAction.Only)
        {
            // ONLY_AFTER/ONLY_BEFORE default to a 50-line window when no count is given.
            rule = new ContextRule(@params.Trim(), direction, 50, action);
        }
        else
        {
            // SKIP_AFTER/SKIP_BEFORE require "pattern, N" — silently ignored without a comma,
            // matching the Rust implementation (the `if let Some(...) = split_once(',')` guard).
            return false;
        }

        return true;
    }

    private static DiffHunk? ParseDiffHunk(string[] lines, ref int index)
    {
        if (index >= lines.Length || !lines[index].Trim().StartsWith("---", StringComparison.Ordinal))
        {
            return null;
        }

        index++;

        if (index >= lines.Length || !lines[index].Trim().StartsWith("+++", StringComparison.Ordinal))
        {
            return null;
        }

        index++;

        if (index >= lines.Length || !lines[index].Trim().StartsWith("@@", StringComparison.Ordinal))
        {
            return null;
        }

        index++;

        var removeLines = new List<string>();
        var addLines = new List<string>();
        var inRemoval = false;
        var inAddition = false;

        while (index < lines.Length)
        {
            var line = lines[index];
            var trimmed = line.Trim();

            if (trimmed.StartsWith("---", StringComparison.Ordinal)
                || trimmed.StartsWith("# QUERY_", StringComparison.Ordinal)
                || trimmed.StartsWith("# RESPONSE_", StringComparison.Ordinal))
            {
                break;
            }

            if (trimmed.Length == 0)
            {
                index++;
                break;
            }

            if (line.StartsWith('-'))
            {
                removeLines.Add(line[1..]);
                inRemoval = true;
                inAddition = false;
            }
            else if (line.StartsWith('+'))
            {
                addLines.Add(line[1..]);
                inRemoval = false;
                inAddition = true;
            }
            // Context lines (leading space) are parsed by the Rust version but never actually
            // used by apply_hunk (they're #[allow(dead_code)] there too) — skipped here.
            _ = inRemoval;
            _ = inAddition;

            index++;
        }

        return removeLines.Count > 0 || addLines.Count > 0 ? new DiffHunk(removeLines, addLines) : null;
    }
}
