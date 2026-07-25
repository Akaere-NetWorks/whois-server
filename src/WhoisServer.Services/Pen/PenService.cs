using System.Text.Json;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Pen;

/// <summary>IANA Private Enterprise Number registry entry.</summary>
public sealed record PenEntry(int Number, string Organization, string Contact, string Email)
{
    public string Oid => $"1.3.6.1.4.1.{Number}";

    public string ToWhoisFormat(DateTimeOffset cachedAt) =>
        $"""
        % IANA Private Enterprise Number (PEN) Information
        % https://www.iana.org/assignments/enterprise-numbers

        Enterprise-Number: {Number}
        OID: {Oid}
        OID-Prefix: iso.org.dod.internet.private.enterprise (1.3.6.1.4.1)
        Organization: {Organization}
        Contact: {Contact}
        Email: {Email}

        % This information is provided for informational purposes only.
        % Data source: IANA Enterprise Numbers Registry
        % Last updated: {cachedAt:yyyy-MM-dd HH:mm:ss} UTC
        """;
}

/// <summary>
/// Ported from src/services/pen.rs — the <c>-PEN</c> suffix. Downloads and parses IANA's
/// enterprise-numbers.txt registry (~60k entries) and caches each entry individually for 30 days
/// via <see cref="IKvStore"/>, refreshed at most once a day. Uses distinct key prefixes for
/// entries vs. metadata (<c>pen:entry:*</c> / <c>pen:meta:*</c>) rather than the Rust version's
/// single <c>pen_*</c> prefix for everything, which relied on JSON-parse failure to silently skip
/// the two non-entry keys during a prefix scan — avoiding that fragility entirely instead of
/// porting it.
/// </summary>
public sealed class PenService(HttpClient httpClient, IKvStore store)
{
    private const string DataUrl = "https://www.iana.org/assignments/enterprise-numbers.txt";
    private const string LastUpdateKey = "pen:meta:last_update";
    private const string FileContentKey = "pen:meta:file_content";
    private const string EntryPrefix = "pen:entry:";
    private static readonly TimeSpan UpdateInterval = TimeSpan.FromDays(1);
    private static readonly TimeSpan EntryTtl = TimeSpan.FromDays(30);

    public async Task<bool> NeedsUpdateAsync(CancellationToken ct = default)
    {
        var raw = await store.GetAsync(LastUpdateKey, ct);
        if (!long.TryParse(raw, out var lastUpdate))
        {
            return true;
        }

        return DateTimeOffset.UtcNow.ToUnixTimeSeconds() - lastUpdate > UpdateInterval.TotalSeconds;
    }

    public async Task ForceUpdateAsync(CancellationToken ct = default)
    {
        // iana.org 403s a request with no User-Agent at all (same anti-bot behavior hit earlier
        // in this port by RIPE's RDAP endpoint and PeeringDB) — matches the Rust version, which
        // set a browser-like UA on its HTTP_CLIENT for exactly this reason.
        using var request = new HttpRequestMessage(HttpMethod.Get, DataUrl);
        request.Headers.TryAddWithoutValidation("User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36");

        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var content = await response.Content.ReadAsStringAsync(ct);

        await store.PutAsync(FileContentKey, content, ct: ct);
        await ParseAndStoreAsync(content, ct);
        await store.PutAsync(LastUpdateKey, DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString(), ct: ct);
    }

    public async Task<string?> QueryByNumberAsync(int number, CancellationToken ct = default)
    {
        var cached = await GetEntryAsync(number, ct);
        if (cached is not null)
        {
            return cached.Value.Entry.ToWhoisFormat(cached.Value.CachedAt);
        }

        await EnsureDataAvailableAsync(ct);

        cached = await GetEntryAsync(number, ct);
        return cached?.Entry.ToWhoisFormat(cached.Value.CachedAt);
    }

    public async Task<IReadOnlyList<string>> SearchByNameAsync(string query, CancellationToken ct = default)
    {
        await EnsureDataAvailableAsync(ct);

        const int maxResults = 20;
        var queryLower = query.ToLowerInvariant();
        var results = new List<string>();
        var now = DateTimeOffset.UtcNow;

        await foreach (var (_, json) in store.IteratePrefixAsync(EntryPrefix, ct))
        {
            var entry = JsonSerializer.Deserialize<PenEntry>(json);
            if (entry is null)
            {
                continue;
            }

            if (entry.Organization.ToLowerInvariant().Contains(queryLower)
                || entry.Contact.ToLowerInvariant().Contains(queryLower)
                || entry.Email.ToLowerInvariant().Contains(queryLower))
            {
                results.Add(entry.ToWhoisFormat(now));
                if (results.Count > maxResults)
                {
                    break;
                }
            }
        }

        if (results.Count > maxResults)
        {
            results = [.. results.Take(maxResults)];
            results.Add($"\n% Search limited to {maxResults} results. Please refine your query for more specific results.");
        }

        if (results.Count == 0)
        {
            return [$"% No IANA Private Enterprise Numbers found matching: {query}\n% Please try a different search term or use exact PEN number query."];
        }

        return results;
    }

    public async Task<string> HandleQueryAsync(string query, CancellationToken ct = default)
    {
        query = query.Trim();

        if (int.TryParse(query, out var number))
        {
            var result = await QueryByNumberAsync(number, ct);
            return result ?? $"""
                % IANA Private Enterprise Number {number} not found.
                % The number may not be assigned yet, or the database needs updating.
                """;
        }

        var results = await SearchByNameAsync(query, ct);
        return results.Count == 0
            ? $"""
                % No results found for query: {query}
                % Try searching by enterprise number or organization name.
                """
            : string.Join("\n\n", results);
    }

    private async Task<(PenEntry Entry, DateTimeOffset CachedAt)?> GetEntryAsync(int number, CancellationToken ct)
    {
        var json = await store.GetAsync($"{EntryPrefix}{number}", ct);
        if (json is null)
        {
            return null;
        }

        var entry = JsonSerializer.Deserialize<PenEntry>(json);
        return entry is null ? null : (entry, DateTimeOffset.UtcNow);
    }

    private async Task EnsureDataAvailableAsync(CancellationToken ct)
    {
        if (await store.ExistsAsync($"{EntryPrefix}1", ct))
        {
            return;
        }

        var cachedFile = await store.GetAsync(FileContentKey, ct);
        if (cachedFile is not null)
        {
            await ParseAndStoreAsync(cachedFile, ct);
            return;
        }

        await ForceUpdateAsync(ct);
    }

    private async Task ParseAndStoreAsync(string content, CancellationToken ct)
    {
        var lines = content.Split('\n');
        var i = 0;

        while (i < lines.Length && !int.TryParse(lines[i].Trim(), out _))
        {
            i++;
        }

        while (i < lines.Length)
        {
            var line = lines[i].TrimEnd('\r').Trim();

            if (int.TryParse(line, out var number) && i + 3 < lines.Length)
            {
                var orgLine = lines[i + 1].TrimEnd('\r');
                var contactLine = lines[i + 2].TrimEnd('\r');
                var emailLine = lines[i + 3].TrimEnd('\r');

                if (orgLine.StartsWith("  ", StringComparison.Ordinal)
                    && contactLine.StartsWith("    ", StringComparison.Ordinal)
                    && emailLine.StartsWith("      ", StringComparison.Ordinal))
                {
                    var entry = new PenEntry(number, orgLine.Trim(), contactLine.Trim(), emailLine.Trim().Replace('&', '@'));
                    await store.PutAsync($"{EntryPrefix}{number}", JsonSerializer.Serialize(entry), EntryTtl, ct);
                    i += 4;
                    continue;
                }
            }

            i++;
        }
    }
}
