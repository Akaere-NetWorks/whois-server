using System.Text;
using WhoisServer.Core.Query;

namespace WhoisServer.Services;

/// <summary>Ported from src/services/desc.rs — the <c>-DESC</c> suffix re-runs the normal
/// resolution pipeline and extracts only <c>descr:</c>/<c>description:</c>/<c>remarks:</c> fields,
/// preserving their original order.</summary>
public sealed class DescService(ResolutionOrchestrator resolution)
{
    private sealed record DescField(string FieldType, string Value);

    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        var innerType = QueryAnalyzer.Analyze(baseQuery);
        var rawResponse = await resolution.ResolveAsync(baseQuery, innerType, ct);

        var fields = ExtractDescAndRemarks(rawResponse);
        return FormatResponse(baseQuery, fields);
    }

    private static List<DescField> ExtractDescAndRemarks(string response)
    {
        var fields = new List<DescField>();

        foreach (var rawLine in response.Split('\n'))
        {
            var line = rawLine.Trim();

            if (TryExtractFieldValue(line, "descr", out var descr))
            {
                fields.Add(new DescField("descr", descr));
            }
            else if (TryExtractFieldValue(line, "remarks", out var remarks))
            {
                fields.Add(new DescField("remarks", remarks));
            }
            else if (TryExtractFieldValue(line, "description", out var description))
            {
                fields.Add(new DescField("description", description));
            }
        }

        return fields;
    }

    private static bool TryExtractFieldValue(string line, string fieldName, out string value)
    {
        value = string.Empty;
        if (!line.StartsWith(fieldName, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var colonIndex = line.IndexOf(':');
        if (colonIndex < 0)
        {
            return false;
        }

        var extracted = line[(colonIndex + 1)..].Trim();
        if (extracted.Length == 0)
        {
            return false;
        }

        value = extracted;
        return true;
    }

    private static string FormatResponse(string query, IReadOnlyList<DescField> fields)
    {
        if (fields.Count == 0)
        {
            return $"% Description Query Results for: {query}\n% No description or remarks fields found\n";
        }

        var sb = new StringBuilder();
        sb.AppendLine($"% Description Query Results for: {query}");

        var descrCount = fields.Count(f => f.FieldType is "descr" or "description");
        var remarksCount = fields.Count(f => f.FieldType == "remarks");

        if (descrCount > 0 && remarksCount > 0)
        {
            sb.AppendLine($"% {descrCount} description(s) and {remarksCount} remarks found");
        }
        else if (descrCount > 0)
        {
            sb.AppendLine($"% {descrCount} description(s) found");
        }
        else
        {
            sb.AppendLine($"% {remarksCount} remarks found");
        }

        sb.AppendLine();

        foreach (var field in fields)
        {
            sb.AppendLine($"{field.FieldType}:             {field.Value}");
        }

        sb.AppendLine();
        sb.AppendLine($"% Total fields: {fields.Count} (descriptions: {descrCount}, remarks: {remarksCount})");

        return sb.ToString();
    }
}
