using System.Text.RegularExpressions;

namespace WhoisServer.Core.Patch;

/// <summary>
/// Applies loaded patches to a WHOIS response. Ported from core/patch.rs's
/// <c>apply_patches</c>/<c>apply_hunk</c>/<c>check_context_rules</c>/<c>should_replace_source_in_block</c>.
/// </summary>
public static class PatchApplier
{
    private static readonly Regex AnsiRegex = new(@"\x1b\[[0-9;]*m", RegexOptions.Compiled);

    private static string StripAnsiCodes(string s) => AnsiRegex.Replace(s, string.Empty);

    public static string ApplyPatches(string query, string response, IReadOnlyList<PatchFile> patchFiles)
    {
        foreach (var patchFile in patchFiles)
        {
            foreach (var patch in patchFile.Patches)
            {
                if (CheckConditions(query, response, patch.Conditions))
                {
                    response = ApplyPatch(response, patch);
                }
            }
        }

        return response;
    }

    private static bool CheckConditions(string query, string response, IReadOnlyList<PatchCondition> conditions)
    {
        if (conditions.Count == 0)
        {
            return true; // No conditions means always apply.
        }

        // OR logic: any condition being true is enough.
        foreach (var condition in conditions)
        {
            var matches = condition.ConditionType switch
            {
                ConditionType.QueryContains => query.Contains(condition.Value, StringComparison.Ordinal),
                ConditionType.ResponseContains => response.Contains(condition.Value, StringComparison.Ordinal),
                ConditionType.QueryMatches => condition.Regex?.IsMatch(query) ?? false,
                ConditionType.ResponseMatches => condition.Regex?.IsMatch(response) ?? false,
                _ => false,
            };

            if (matches)
            {
                return true;
            }
        }

        return false;
    }

    private static string ApplyPatch(string response, PatchRule patch)
    {
        foreach (var hunk in patch.Hunks)
        {
            response = ApplyHunk(response, hunk, patch.Excludes, patch.ContextRules);
        }

        return response;
    }

    private enum ContextCheckResult
    {
        Allow,
        Skip,
        OnlyButNotFound,
    }

    private static ContextCheckResult CheckContextRules(string[] lines, int lineIdx, IReadOnlyList<ContextRule> rules)
    {
        if (rules.Count == 0)
        {
            return ContextCheckResult.Allow;
        }

        var hasOnlyRule = false;
        var onlyRuleSatisfied = false;

        foreach (var rule in rules)
        {
            int start, end;
            if (rule.Direction == ContextDirection.Before)
            {
                start = Math.Max(0, lineIdx - rule.Lines);
                end = lineIdx;
            }
            else
            {
                start = lineIdx + 1;
                end = Math.Min(lines.Length, lineIdx + rule.Lines + 1);
            }

            var patternFound = false;
            for (var i = start; i < end; i++)
            {
                if (i >= 0 && i < lines.Length && StripAnsiCodes(lines[i]).Contains(rule.Pattern, StringComparison.Ordinal))
                {
                    patternFound = true;
                    break;
                }
            }

            if (rule.Action == ContextAction.Skip)
            {
                if (patternFound)
                {
                    return ContextCheckResult.Skip;
                }
            }
            else
            {
                hasOnlyRule = true;
                if (patternFound)
                {
                    onlyRuleSatisfied = true;
                }
            }
        }

        if (hasOnlyRule && !onlyRuleSatisfied)
        {
            return ContextCheckResult.OnlyButNotFound;
        }

        return ContextCheckResult.Allow;
    }

    /// <summary>Only replace `source:` inside user-editable objects (aut-num/organisation/person/role),
    /// never inside registry-authoritative blocks (as-block/route/route6/inet6num/inetnum).</summary>
    private static bool ShouldReplaceSourceInBlock(string[] lines, int lineIdx)
    {
        var start = Math.Max(0, lineIdx - 50);

        for (var i = lineIdx - 1; i >= start; i--)
        {
            var trimmed = StripAnsiCodes(lines[i]).Trim();

            if (trimmed.Length == 0 || trimmed.StartsWith('%'))
            {
                return false;
            }

            if (trimmed.StartsWith("aut-num:", StringComparison.Ordinal)
                || trimmed.StartsWith("organisation:", StringComparison.Ordinal)
                || trimmed.StartsWith("person:", StringComparison.Ordinal)
                || trimmed.StartsWith("role:", StringComparison.Ordinal))
            {
                return true;
            }

            if (trimmed.StartsWith("as-block:", StringComparison.Ordinal)
                || trimmed.StartsWith("route:", StringComparison.Ordinal)
                || trimmed.StartsWith("route6:", StringComparison.Ordinal)
                || trimmed.StartsWith("inet6num:", StringComparison.Ordinal)
                || trimmed.StartsWith("inetnum:", StringComparison.Ordinal))
            {
                return false;
            }
        }

        return false;
    }

    private static string ApplyHunk(string response, DiffHunk hunk, IReadOnlyList<string> excludes, IReadOnlyList<ContextRule> contextRules)
    {
        if (hunk.RemoveLines.Count == 0)
        {
            return response;
        }

        if (hunk.RemoveLines.Count == 1 && hunk.AddLines.Count == 1)
        {
            var old = hunk.RemoveLines[0];
            var replacement = hunk.AddLines[0];

            var lineEnding = response.Contains("\r\n") ? "\r\n" : "\n";

            var isLineStartMatch = old.StartsWith('^');
            var matchPrefix = isLineStartMatch ? old[1..] : old;

            var lines = response.Replace("\r\n", "\n").Split('\n');
            var resultLines = new List<string>(lines.Length);

            for (var idx = 0; idx < lines.Length; idx++)
            {
                var line = lines[idx];

                if (excludes.Any(pattern => line.Contains(pattern, StringComparison.Ordinal)))
                {
                    resultLines.Add(line);
                    continue;
                }

                var contextCheck = CheckContextRules(lines, idx, contextRules);
                if (contextCheck is ContextCheckResult.Skip or ContextCheckResult.OnlyButNotFound)
                {
                    resultLines.Add(line);
                    continue;
                }

                if (isLineStartMatch && matchPrefix == "source:" && !ShouldReplaceSourceInBlock(lines, idx))
                {
                    resultLines.Add(line);
                    continue;
                }

                if (isLineStartMatch)
                {
                    var strippedLine = StripAnsiCodes(line);
                    resultLines.Add(strippedLine.TrimStart().StartsWith(matchPrefix, StringComparison.Ordinal)
                        ? replacement
                        : line);
                }
                else
                {
                    resultLines.Add(line.Replace(old, replacement));
                }
            }

            return string.Join(lineEnding, resultLines);
        }

        // Multi-line replacement: naive whole-string substitution.
        var oldText = string.Join('\n', hunk.RemoveLines);
        var newText = hunk.AddLines.Count == 0 ? string.Empty : string.Join('\n', hunk.AddLines);

        if (excludes.Any(pattern => oldText.Contains(pattern, StringComparison.Ordinal)))
        {
            return response;
        }

        return response.Replace(oldText, newText);
    }
}
