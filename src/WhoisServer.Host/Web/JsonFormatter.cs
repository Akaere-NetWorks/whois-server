using System.Reflection;
using System.Text.Json.Serialization;

namespace WhoisServer.Host.Web;

/// <summary>Ported from src/web/json_formatter.rs — wraps a raw WHOIS-style text response in a
/// structured JSON envelope for <c>/api/whois</c>. <c>/raw/{query}</c> bypasses this entirely and
/// returns the same raw text a terminal `whois` client would see.</summary>
public static class JsonFormatter
{
    private static readonly string Version =
        Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "0.0.0";

    public static WhoisApiResponse FormatResponse(string query, string rawOutput, string queryType, double processingTimeMs) =>
        new(
            Success: !string.IsNullOrWhiteSpace(rawOutput),
            Query: query,
            QueryType: queryType,
            RawOutput: rawOutput,
            Fields: ParseFields(rawOutput),
            Error: null,
            Metadata: new ResponseMetadata(DateTimeOffset.UtcNow, processingTimeMs, "whois-server", Version));

    public static WhoisApiResponse FormatError(string query, string queryType, string message, double processingTimeMs) =>
        new(
            Success: false,
            Query: query,
            QueryType: queryType,
            RawOutput: null,
            Fields: null,
            Error: message,
            Metadata: new ResponseMetadata(DateTimeOffset.UtcNow, processingTimeMs, "whois-server", Version));

    /// <summary>Naive line-oriented parse: skips blank lines and WHOIS comment lines
    /// (<c>%</c>/<c>#</c>), splits everything else on the first <c>:</c>. Repeated keys just
    /// produce multiple entries — no nested/multi-value handling, matching the Rust version.</summary>
    private static List<WhoisField> ParseFields(string rawOutput)
    {
        var fields = new List<WhoisField>();

        foreach (var rawLine in rawOutput.Split('\n'))
        {
            var line = rawLine.TrimEnd('\r').Trim();
            if (line.Length == 0 || line.StartsWith('%') || line.StartsWith('#'))
            {
                continue;
            }

            var colonIndex = line.IndexOf(':');
            if (colonIndex < 0)
            {
                continue;
            }

            var name = line[..colonIndex].Trim();
            var value = line[(colonIndex + 1)..].Trim();
            if (name.Length > 0)
            {
                fields.Add(new WhoisField(name, value));
            }
        }

        return fields;
    }
}

public sealed record WhoisField(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("value")] string Value);

public sealed record ResponseMetadata(
    [property: JsonPropertyName("timestamp")] DateTimeOffset Timestamp,
    [property: JsonPropertyName("processing_time_ms")] double ProcessingTimeMs,
    [property: JsonPropertyName("source")] string Source,
    [property: JsonPropertyName("version")] string Version);

public sealed record WhoisApiResponse(
    [property: JsonPropertyName("success")] bool Success,
    [property: JsonPropertyName("query")] string Query,
    [property: JsonPropertyName("query_type")] string QueryType,
    [property: JsonPropertyName("raw_output")] string? RawOutput,
    [property: JsonPropertyName("fields")] List<WhoisField>? Fields,
    [property: JsonPropertyName("error")] string? Error,
    [property: JsonPropertyName("metadata")] ResponseMetadata Metadata);
