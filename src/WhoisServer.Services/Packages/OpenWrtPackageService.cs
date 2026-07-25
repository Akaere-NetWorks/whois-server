using System.Text;
using System.Text.Json;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/openwrt.rs — the <c>-OPENWRT</c> suffix.
///
/// Deliberate fix, not a byte-for-byte port: the Rust version's <c>check_openwrt_package_feeds</c>
/// builds candidate <c>Packages.gz</c> URLs, logs "would check: {url}" for each, and then always
/// returns one hardcoded generic <c>OpenWrtPackage</c> ("version": "Available", "license": "Various")
/// regardless of the query — it never fetches or parses anything real. This port instead downloads
/// and parses the real, Debian-control-file-formatted <c>Packages</c> index
/// (<c>downloads.openwrt.org/releases/23.05.5/packages/x86_64/{packages,luci,routing}/Packages</c>,
/// confirmed live during this port) for OpenWrt 23.05 — the last release still shipping the classic
/// ipk/opkg "Packages" control-file format before OpenWrt's 24.x+ move to the apk package manager's
/// APKINDEX format, which isn't ported here (documented limitation, not silently dropped). Since
/// each feed file is a couple of megabytes, the parsed name→package index is cached whole via
/// <see cref="IKvStore"/> (mirrors <c>ManrsService</c>'s whole-list-refresh cache pattern) rather
/// than re-downloaded per query.
/// </summary>
public sealed class OpenWrtPackageService(HttpClient httpClient, IKvStore store)
{
    private const string Release = "23.05.5";
    private const string Arch = "x86_64";
    private static readonly string[] Feeds = ["packages", "luci", "routing"];
    private const string IndexCacheKey = "openwrt_package_index_v1";
    private static readonly TimeSpan CacheTtl = TimeSpan.FromHours(24);

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidPackageName(packageName))
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var index = await GetOrBuildIndexAsync(ct);
            return index.TryGetValue(packageName, out var entry)
                ? FormatResponse(entry, packageName)
                : FormatNotFound(packageName);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    /// <summary>Ported verbatim from the Rust version: max 100 chars, ASCII alphanumeric plus
    /// <c>+-._</c>, no spaces.</summary>
    private static bool IsValidPackageName(string packageName) =>
        packageName.Length is > 0 and <= 100
        && !packageName.Contains(' ')
        && packageName.All(c => char.IsAsciiLetterOrDigit(c) || c is '+' or '-' or '.' or '_');

    private async Task<Dictionary<string, OpenWrtEntry>> GetOrBuildIndexAsync(CancellationToken ct)
    {
        var cached = await store.GetAsync(IndexCacheKey, ct);
        if (cached is not null)
        {
            var deserialized = JsonSerializer.Deserialize<Dictionary<string, OpenWrtEntry>>(cached);
            if (deserialized is not null)
            {
                return deserialized;
            }
        }

        var index = new Dictionary<string, OpenWrtEntry>(StringComparer.Ordinal);
        var feedTexts = await Task.WhenAll(Feeds.Select(feed => FetchFeedAsync(feed, ct)));

        for (var i = 0; i < Feeds.Length; i++)
        {
            if (feedTexts[i] is not { } text)
            {
                continue;
            }

            foreach (var entry in ParseControlFile(text, Feeds[i]))
            {
                index.TryAdd(entry.Name, entry);
            }
        }

        await store.PutAsync(IndexCacheKey, JsonSerializer.Serialize(index), CacheTtl, ct);
        return index;
    }

    private async Task<string?> FetchFeedAsync(string feed, CancellationToken ct)
    {
        var url = $"https://downloads.openwrt.org/releases/{Release}/packages/{Arch}/{feed}/Packages";
        try
        {
            using var response = await httpClient.GetAsync(url, ct);
            return response.IsSuccessStatusCode ? await response.Content.ReadAsStringAsync(ct) : null;
        }
        catch
        {
            return null;
        }
    }

    private static IEnumerable<OpenWrtEntry> ParseControlFile(string text, string feed)
    {
        string? name = null, version = null, depends = null, license = null, section = null;
        string? filename = null, description = null;
        long? size = null, installedSize = null;
        string? lastKey = null;

        void Flush(List<OpenWrtEntry> results)
        {
            if (name is not null)
            {
                results.Add(new OpenWrtEntry(name, version, depends, license, section, filename, description, size, installedSize, feed));
            }

            name = version = depends = license = section = filename = description = null;
            size = installedSize = null;
            lastKey = null;
        }

        var results = new List<OpenWrtEntry>();
        foreach (var rawLine in text.Split('\n'))
        {
            var line = rawLine.TrimEnd('\r');

            if (line.Length == 0)
            {
                Flush(results);
                continue;
            }

            if (line.StartsWith(' ') || line.StartsWith('\t'))
            {
                if (lastKey == "Description" && description is not null)
                {
                    description += " " + line.Trim();
                }

                continue;
            }

            var separatorIndex = line.IndexOf(':');
            if (separatorIndex < 0)
            {
                continue;
            }

            var key = line[..separatorIndex];
            var value = line[(separatorIndex + 1)..].Trim();
            lastKey = key;

            switch (key)
            {
                case "Package": name = value; break;
                case "Version": version = value; break;
                case "Depends": depends = value; break;
                case "License": license = value; break;
                case "Section": section = value; break;
                case "Filename": filename = value; break;
                case "Description": description = value; break;
                case "Size": size = long.TryParse(value, out var s) ? s : null; break;
                case "Installed-Size": installedSize = long.TryParse(value, out var i) ? i : null; break;
            }
        }

        Flush(results);
        return results;
    }

    private static string FormatResponse(OpenWrtEntry entry, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"OpenWrt Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        sb.Append($"package: {entry.Name}\n");
        if (entry.Version is { } version) sb.Append($"version: {version}\n");
        sb.Append($"feed: {entry.Feed}\n");
        if (entry.Section is { } section) sb.Append($"section: {section}\n");
        if (entry.Description is { } description) sb.Append($"description: {description}\n");
        sb.Append($"architecture: {Arch}\n");
        if (entry.License is { } license) sb.Append($"license: {license}\n");
        if (entry.Depends is { } depends) sb.Append($"depends: {depends}\n");
        if (entry.InstalledSize is { } installedSize) sb.Append($"installed-size: {installedSize / 1024.0:F2} KB\n");
        if (entry.Size is { } size) sb.Append($"download-size: {size / 1024.0:F2} KB\n");
        if (entry.Filename is { } filename) sb.Append($"filename: {filename}\n");

        sb.Append($"distribution: OpenWrt {Release}\n");
        sb.Append("package-format: IPK (opkg)\n");
        sb.Append('\n');
        sb.Append("% Installation: opkg update && opkg install ").Append(query).Append('\n');
        sb.Append("% Information retrieved from the OpenWrt package feed index\n");
        return sb.ToString();
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            OpenWrt Package Not Found: {packageName}
            No package with this name was found in the OpenWrt {Release} packages/luci/routing feeds
            for {Arch}.

            % Search manually: https://openwrt.org/packages
            % opkg update && opkg search {packageName}
            %
            % Note: only the {Release} release (classic ipk/opkg "Packages" index format) is
            % checked here — OpenWrt 24.x+ switched to the apk package manager's APKINDEX format,
            % which this lookup does not parse.

            """;
    }

    private sealed record OpenWrtEntry(
        string Name, string? Version, string? Depends, string? License, string? Section,
        string? Filename, string? Description, long? Size, long? InstalledSize, string Feed);
}
