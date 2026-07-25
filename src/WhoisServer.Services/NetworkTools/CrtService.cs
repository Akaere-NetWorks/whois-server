using System.Net.Http.Json;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/crt.rs — the <c>-CRT</c> suffix: queries crt.sh's
/// Certificate Transparency log search, keeping only currently-valid (non-expired) certificates.</summary>
public sealed class CrtService(HttpClient httpClient)
{
    public async Task<string> ProcessAsync(string domain, CancellationToken ct = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, $"https://crt.sh/json?q={Uri.EscapeDataString(domain)}");
            using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            timeoutCts.CancelAfter(TimeSpan.FromSeconds(20));

            using var response = await httpClient.SendAsync(request, timeoutCts.Token);
            response.EnsureSuccessStatusCode();

            var entries = await response.Content.ReadFromJsonAsync<List<CrtEntry>>(cancellationToken: timeoutCts.Token) ?? [];
            var valid = FilterValid(entries);
            return Format(valid, domain);
        }
        catch (Exception e)
        {
            return $"""
                Certificate Transparency Query Failed for {domain}
                Error: {e.Message}

                Note: crt.sh API is known to be unstable and may timeout frequently.
                Please try again or use alternative certificate lookup methods.

                """;
        }
    }

    private static List<CrtEntry> FilterValid(List<CrtEntry> entries)
    {
        var now = DateTimeOffset.UtcNow;
        var valid = new List<CrtEntry>();

        foreach (var entry in entries)
        {
            if (!DateTimeOffset.TryParse(entry.NotBefore, out var notBefore)
                || !DateTimeOffset.TryParse(entry.NotAfter, out var notAfter))
            {
                continue;
            }

            if (now < notBefore || now > notAfter)
            {
                continue;
            }

            valid.Add(entry);
        }

        return valid
            .OrderByDescending(e => e.NotAfter, StringComparer.Ordinal)
            .DistinctBy(e => (e.SerialNumber, e.IssuerName))
            .ToList();
    }

    private static string Format(List<CrtEntry> certificates, string domain)
    {
        if (certificates.Count == 0)
        {
            return $"""
                Certificate Transparency Query Results for: {domain}

                No valid (non-expired) certificates found in Certificate Transparency logs.
                This could mean:
                - Domain has no certificates
                - All certificates are expired
                - Domain is not publicly accessible
                - crt.sh may not have indexed this domain yet

                """;
        }

        var sb = new StringBuilder();
        sb.Append($"Certificate Transparency Query Results for: {domain}\n");
        sb.Append($"Found {certificates.Count} valid (non-expired) certificates from CT logs\n");
        sb.Append(new string('=', 80)).Append('\n');

        for (var i = 0; i < certificates.Count; i++)
        {
            var cert = certificates[i];
            sb.Append($"\n[{i + 1}] Certificate #{cert.Id}\n");
            sb.Append($"Common Name: {cert.CommonName ?? "Unknown"}\n");
            sb.Append($"Issuer: {cert.IssuerName}\n");
            sb.Append($"Serial Number: {cert.SerialNumber}\n");
            sb.Append($"Valid From: {cert.NotBefore}\n");
            sb.Append($"Valid Until: {cert.NotAfter}\n");
            sb.Append($"CT Log Entry: {cert.EntryTimestamp}\n");

            if (i < certificates.Count - 1)
            {
                sb.Append(new string('-', 40)).Append('\n');
            }
        }

        sb.Append("\nNote: Data sourced from Certificate Transparency logs via crt.sh\n");
        sb.Append("Only currently valid (non-expired) certificates are shown\n");
        return sb.ToString();
    }

    private sealed record CrtEntry(
        [property: JsonPropertyName("issuer_name")] string IssuerName,
        [property: JsonPropertyName("common_name")] string? CommonName,
        [property: JsonPropertyName("id")] long Id,
        [property: JsonPropertyName("entry_timestamp")] string EntryTimestamp,
        [property: JsonPropertyName("not_before")] string NotBefore,
        [property: JsonPropertyName("not_after")] string NotAfter,
        [property: JsonPropertyName("serial_number")] string SerialNumber);
}
