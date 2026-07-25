using System.Text.RegularExpressions;
using WhoisServer.Core.Query;

namespace WhoisServer.Core.Color;

/// <summary>
/// Ported from src/core/color/colorizer.rs. This is a large, literal attribute-name -> ANSI-code
/// lookup table plus a handful of numeric-threshold rules (latency, NTP stratum/offset) — there is
/// no cleaner abstraction to extract, so it is transcribed close to verbatim rather than redesigned.
/// Golden tests in WhoisServer.Core.Tests/ColorizerTests.cs (ported from core/color/tests.rs) guard
/// against regressions; run those before touching attribute mappings here.
/// </summary>
public sealed class Colorizer(ColorScheme scheme)
{
    private static readonly Regex AsnRegex = new(@"(AS\d+)", RegexOptions.Compiled);
    private static readonly Regex IpRegex = new(
        @"(\d+\.\d+\.\d+\.\d+(?:/\d+)?|[0-9a-fA-F:]+::[0-9a-fA-F:]*(?:/\d+)?)", RegexOptions.Compiled);
    private static readonly Regex DomainRegex = new(
        @"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}", RegexOptions.Compiled);
    private static readonly Regex IdRegex = new(@"(\d+)", RegexOptions.Compiled);
    private static readonly Regex UrlRegex = new(@"(https?://[^\s]+)", RegexOptions.Compiled);
    private static readonly Regex MsRegex = new(@"(\d+)\s*ms", RegexOptions.Compiled);
    private static readonly Regex NegativeMsRegex = new(@"(-?\d+\.?\d*)\s*ms", RegexOptions.Compiled);

    public string ColorizeResponse(string response, QueryType queryType) => scheme switch
    {
        ColorScheme.Ripe => ColorizeRipeStyle(response, queryType, boldColors: true),
        ColorScheme.RipeDark => ColorizeRipeStyle(response, queryType, boldColors: false),
        ColorScheme.BgpTools => ColorizeBgpToolsStyle(response, queryType, boldColors: true),
        ColorScheme.BgpToolsDark => ColorizeBgpToolsStyle(response, queryType, boldColors: false),
        _ => throw new ArgumentOutOfRangeException(nameof(queryType)),
    };

    private string ColorizeRipeStyle(string response, QueryType queryType, bool boldColors)
    {
        var lines = new List<string>();
        foreach (var line in SplitLines(response))
        {
            string colored;
            if (line.StartsWith('%'))
            {
                colored = boldColors ? $"\x1b[90m{line}\x1b[0m" : $"\x1b[37m{line}\x1b[0m";
            }
            else if (line.Contains(':') && !line.StartsWith(' '))
            {
                colored = ColorizeRipeAttributes(line, boldColors);
            }
            else
            {
                colored = ColorizeQueryTypeContent(line, queryType, boldColors);
            }

            lines.Add(colored);
        }

        return string.Join("\r\n", lines);
    }

    private string ColorizeBgpToolsStyle(string response, QueryType queryType, bool boldColors)
    {
        var lines = new List<string>();
        foreach (var line in SplitLines(response))
        {
            string colored;
            if (line.StartsWith('%'))
            {
                colored = boldColors ? $"\x1b[90m{line}\x1b[0m" : $"\x1b[37m{line}\x1b[0m";
            }
            else if (line.Contains(':') && !line.StartsWith(' '))
            {
                colored = ColorizeBgpToolsAttributes(line, boldColors);
            }
            else
            {
                colored = ColorizeQueryTypeContent(line, queryType, boldColors);
            }

            lines.Add(colored);
        }

        return string.Join("\r\n", lines);
    }

    private static IEnumerable<string> SplitLines(string text)
    {
        // Mirrors Rust's str::lines(): splits on \n, trims a trailing \r per line, no trailing
        // empty entry for a final newline.
        var normalized = text.Replace("\r\n", "\n");
        if (normalized.EndsWith('\n'))
        {
            normalized = normalized[..^1];
        }

        return normalized.Length == 0 ? [] : normalized.Split('\n');
    }

    private static string ColorizeRipeAttributes(string line, bool boldColors)
    {
        var colonIndex = line.IndexOf(':');
        if (colonIndex < 0)
        {
            return line;
        }

        var attr = line[..colonIndex].Trim();
        var value = line[(colonIndex + 1)..];

        return attr switch
        {
            "inetnum" or "inet6num" or "route" or "route6" or "network" or "prefix" =>
                boldColors
                    ? $"\x1b[1;96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m"
                    : $"\x1b[36m{attr}:\x1b[0m \x1b[36m{value}\x1b[0m",
            "domain" or "nserver" or "dns" =>
                boldColors
                    ? $"\x1b[1;96m{attr}:\x1b[0m \x1b[1;96m{value}\x1b[0m"
                    : $"\x1b[36m{attr}:\x1b[0m \x1b[36m{value}\x1b[0m",
            "origin" or "aut-num" or "as-name" or "asn" =>
                $"\x1b[1;93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "person" or "admin-c" or "tech-c" or "mnt-by" or "contact" or "email" =>
                $"\x1b[32m{attr}:\x1b[0m \x1b[32m{value}\x1b[0m",
            "netname" or "name" =>
                $"\x1b[1;92m{attr}:\x1b[0m \x1b[1;92m{value}\x1b[0m",
            "org" or "orgname" or "org-name" or "organisation" =>
                boldColors
                    ? $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m"
                    : $"\x1b[33m{attr}:\x1b[0m \x1b[33m{value}\x1b[0m",
            "descr" or "description" =>
                boldColors
                    ? $"\x1b[37m{attr}:\x1b[0m \x1b[37m{value}\x1b[0m"
                    : $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "country" or "address" or "city" or "region" or "geoloc" =>
                boldColors
                    ? $"\x1b[35m{attr}:\x1b[0m \x1b[35m{value}\x1b[0m"
                    : $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "registrar" or "sponsoring-registrar" or "registrant" =>
                boldColors
                    ? $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m"
                    : $"\x1b[1;94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "status" or "state" or "rpki-status" or "validation" => ColorizeStatusValue(attr, value),
            "created" or "changed" or "last-modified" or "expires" or "updated" or "created-at"
                or "updated-at" or "pushed-at" =>
                boldColors
                    ? $"\x1b[35m{attr}:\x1b[0m \x1b[35m{value}\x1b[0m"
                    : $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "package" or "package-name" or "version" or "latest-version" or "stable-version"
                or "package-base" or "source-package" or "attribute-name" or "attribute-set" =>
                boldColors
                    ? $"\x1b[37m{attr}:\x1b[0m \x1b[37m{value}\x1b[0m"
                    : $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "summary" or "long-description" or "nixpkgs-position" =>
                boldColors
                    ? $"\x1b[36m{attr}:\x1b[0m \x1b[36m{value}\x1b[0m"
                    : $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "license" or "distribution" =>
                $"\x1b[1;92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m",
            "size" or "filename" or "modified-time" or "unpacked-size" or "file-count"
                or "total-size" or "package-size" or "wheel-size" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "popularity" or "votes" or "rating" or "score" or "stars" or "watchers" or "forks"
                or "open-issues" or "downloads" or "total-downloads" or "recent-downloads"
                or "followers" or "following" or "views" or "likes" or "bookmarks" or "reposts" =>
                $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "aur-url" or "upstream-url" or "url" or "homepage" or "ubuntu-url" or "nixos-url"
                or "opensuse-url" or "npm-url" or "registry-url" or "pypi-url" or "crates-io-url"
                or "docs-rs-url" or "api-url" or "github-url" or "clone-url" or "ssh-url" or "avatar-url"
                or "profileurl" or "steam-url" or "website" or "metacritic-url" or "wikipedia-url"
                or "edit-url" =>
                $"\x1b[1;94m{attr}:\x1b[0m {UrlRegex.Replace(value, "\x1b[4;94m$1\x1b[0m")}",
            "app-id" or "steamid" or "game-id" or "metacritic-score" =>
                $"\x1b[1;93m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;93m$1\x1b[0m")}",
            "price" or "original-price" => ColorizePriceValue(attr, value),
            "players" or "players-online" or "max-players" =>
                $"\x1b[95m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;95m$1\x1b[0m")}",
            "latency" or "ping" or "round-trip" => ColorizeLatencyValue(attr, value),
            "imdb-id" or "tt-id" =>
                $"\x1b[1;93m{attr}:\x1b[0m {Regex.Replace(value, @"(tt\d+)", "\x1b[1;93m$1\x1b[0m")}",
            "movie-title" or "series-title" or "game-title" =>
                boldColors
                    ? $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m"
                    : $"\x1b[1;96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "year" or "release-year" or "release-date" =>
                $"\x1b[93m{attr}:\x1b[0m {Regex.Replace(value, @"(\d{4})", "\x1b[1;93m$1\x1b[0m")}",
            "metascore" or "box-office" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "director" or "writer" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "actors" or "cast" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "genre" or "genres" or "categories" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "awards" =>
                $"\x1b[1;93m{attr}:\x1b[0m \x1b[1;93m{value}\x1b[0m",
            "rated" or "mpaa-rating" => ColorizeRatedValue(attr, value),
            "repository-name" or "repo-name" or "full-name" or "repo" =>
                $"\x1b[1;96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "owner" or "username" or "user" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "language" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "default-branch" or "branch" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "visibility" or "private" or "public" => ColorizeVisibilityValue(attr, value),
            "page-id" or "article-id" =>
                $"\x1b[1;93m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;93m$1\x1b[0m")}",
            "article-length" or "page-length" =>
                $"\x1b[95m{attr}:\x1b[0m {Regex.Replace(value, @"(\d+)\s*bytes", "\x1b[1;93m$1 bytes\x1b[0m")}",
            "last-edited" =>
                boldColors
                    ? $"\x1b[90m{attr}:\x1b[0m \x1b[90m{value}\x1b[0m"
                    : $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "languages" =>
                $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m",
            "artwork-id" or "illust-id" =>
                $"\x1b[1;94m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;94m$1\x1b[0m")}",
            "user-id" or "artist-id" =>
                $"\x1b[1;95m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;95m$1\x1b[0m")}",
            "artwork-title" =>
                $"\x1b[1;96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "artwork-url" or "profile-url" =>
                $"\x1b[1;94m{attr}:\x1b[0m {UrlRegex.Replace(value, "\x1b[4;94m$1\x1b[0m")}",
            "content-rating" => ColorizeContentRatingValue(attr, value),
            "character-name" or "character" =>
                $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "voice-actor" or "cv" or "seiyuu" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "source-work" or "series" or "anime" or "manga" or "game" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "personality" or "traits" or "moe-points" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "species" or "race" or "identity" or "class" or "level" =>
                $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m",
            "ability" or "skill" or "power" or "weapon" or "equipment" =>
                $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m",
            "alias" or "nickname" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "age" or "birthday" =>
                $"\x1b[95m{attr}:\x1b[0m {IdRegex.Replace(value, "\x1b[1;93m$1\x1b[0m")}",
            "height" or "weight" or "bwh" =>
                $"\x1b[95m{attr}:\x1b[0m {Regex.Replace(value, @"(\d+[\.\d]*\s*(cm|kg|m|ft|in))", "\x1b[1;92m$1\x1b[0m")}",
            "hair-color" or "eye-color" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "gender" => ColorizeGenderValue(attr, value),
            "song-name" or "song" or "track" =>
                $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "singer" or "artist" or "vocalist" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "author" or "lyricist" or "composer" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "lyric-content" or "lyrics" =>
                $"\x1b[1;37m{attr}:\x1b[0m \x1b[1;37m{value}\x1b[0m",
            "meal-name" or "dish" =>
                boldColors
                    ? $"\x1b[1;93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m"
                    : $"\x1b[1;92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m",
            "category" or "meal-type" =>
                $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m",
            "cuisine" or "cooking-style" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "ingredient" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "instruction" or "step" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "cooking-time" or "prep-time" =>
                $"\x1b[93m{attr}:\x1b[0m {Regex.Replace(value, @"(\d+\s*min|\d+\s*hours?)", "\x1b[93m$1\x1b[0m")}",
            "mp-import" or "mp-export" or "import" or "export" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            "policy" or "filter" or "pref" or "med" or "local-pref" =>
                $"\x1b[95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m",
            "member-of" or "members" or "as-set" or "route-set" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m",
            "mnt-lower" or "mnt-routes" or "mnt-domains" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m",
            "stratum" => ColorizeStratumValue(attr, value),
            "offset" or "root-delay" or "root-dispersion" => ColorizeOffsetValue(attr, value),
            "delay" or "reach" or "jitter" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m",
            _ => ColorizeDefaultRainbow(attr, value),
        };
    }

    private static string ColorizeStatusValue(string attr, string value)
    {
        var lower = value.Trim().ToLowerInvariant();
        if (lower.Contains("valid") && !lower.Contains("invalid"))
        {
            return $"\x1b[1;92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m";
        }

        if (lower.Contains("invalid"))
        {
            return $"\x1b[1;91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
        }

        return $"\x1b[1;93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m";
    }

    private static string ColorizePriceValue(string attr, string value)
    {
        if (value.Contains("(%↓)") || value.Contains("Free"))
        {
            var colored = Regex.Replace(value, @"(\$[\d,]+\.?\d*|Free)", "\x1b[1;92m$1\x1b[0m");
            colored = Regex.Replace(colored, @"(\d+%↓)", "\x1b[1;92m$1\x1b[0m");
            return $"\x1b[1;95m{attr}:\x1b[0m{colored}";
        }

        var priceOnly = Regex.Replace(value, @"(\$[\d,]+\.?\d*)", "\x1b[97m$1\x1b[0m");
        return $"\x1b[1;95m{attr}:\x1b[0m{priceOnly}";
    }

    private static string ColorizeLatencyValue(string attr, string value)
    {
        var colored = MsRegex.Replace(value, m =>
        {
            var ms = int.Parse(m.Groups[1].Value);
            return ms < 50 ? $"\x1b[1;92m{ms}ms\x1b[0m" : ms < 150 ? $"\x1b[1;93m{ms}ms\x1b[0m" : $"\x1b[1;91m{ms}ms\x1b[0m";
        });
        return $"\x1b[93m{attr}:\x1b[0m {colored}";
    }

    private static string ColorizeRatedValue(string attr, string value)
    {
        if (value.Contains("PG") || value.Contains('G'))
        {
            return $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m";
        }

        if (value.Contains('R') || value.Contains("NC-17"))
        {
            return $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
        }

        return $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m";
    }

    private static string ColorizeVisibilityValue(string attr, string value)
    {
        return value.Contains("Public") || value.Contains("false")
            ? $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m"
            : $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
    }

    private static string ColorizeContentRatingValue(string attr, string value)
    {
        var lower = value.ToLowerInvariant();
        if (lower.Contains("safe"))
        {
            return $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m";
        }

        if (lower.Contains("r-18") || lower.Contains("r18"))
        {
            return $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
        }

        return $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m";
    }

    private static string ColorizeGenderValue(string attr, string value)
    {
        var lower = value.ToLowerInvariant();
        if (value.Contains('女') || lower.Contains("female"))
        {
            return $"\x1b[1;95m{attr}:\x1b[0m \x1b[95m{value}\x1b[0m";
        }

        if (value.Contains('男') || lower.Contains("male"))
        {
            return $"\x1b[1;94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m";
        }

        return $"\x1b[1;96m{attr}:\x1b[0m \x1b[96m{value}\x1b[0m";
    }

    private static string ColorizeStratumValue(string attr, string value)
    {
        var colored = IdRegex.Replace(value, m =>
        {
            var stratum = int.Parse(m.Groups[1].Value);
            return stratum <= 2 ? $"\x1b[1;92m{stratum}\x1b[0m" : stratum <= 4 ? $"\x1b[1;93m{stratum}\x1b[0m" : $"\x1b[1;37m{stratum}\x1b[0m";
        });
        return $"\x1b[95m{attr}:\x1b[0m {colored}";
    }

    private static string ColorizeOffsetValue(string attr, string value)
    {
        var colored = NegativeMsRegex.Replace(value, m =>
        {
            var offset = double.Parse(m.Groups[1].Value, System.Globalization.CultureInfo.InvariantCulture);
            var abs = Math.Abs(offset);
            return abs < 10.0 ? $"\x1b[1;92m{offset}ms\x1b[0m" : abs < 100.0 ? $"\x1b[1;93m{offset}ms\x1b[0m" : $"\x1b[1;91m{offset}ms\x1b[0m";
        });
        return $"\x1b[94m{attr}:\x1b[0m {colored}";
    }

    /// <summary>Deterministic (not cryptographic) fallback for unrecognized attribute names —
    /// sum of char codepoints mod 6, matching the Rust version exactly so unknown fields keep the
    /// same color across the port.</summary>
    private static string ColorizeDefaultRainbow(string attr, string value)
    {
        var hash = attr.Sum(c => (uint)c);
        var colorCode = 31 + (hash % 6);
        return $"\x1b[{colorCode}m{attr}:\x1b[0m \x1b[{colorCode}m{value}\x1b[0m";
    }

    private static string ColorizeBgpToolsAttributes(string line, bool boldColors)
    {
        var colonIndex = line.IndexOf(':');
        if (colonIndex < 0)
        {
            return line;
        }

        var attr = line[..colonIndex].Trim();
        var value = line[(colonIndex + 1)..];

        const string asnColor = "\x1b[93m";
        const string ipColor = "\x1b[92m";
        const string domainColor = "\x1b[94m";

        var styledValue = AsnRegex.Replace(value, $"{asnColor}$1\x1b[0m");
        styledValue = IpRegex.Replace(styledValue, $"{ipColor}$1\x1b[0m");
        styledValue = DomainRegex.Replace(styledValue, $"{domainColor}$1\x1b[0m");

        return attr switch
        {
            "origin" or "aut-num" or "as-name" or "asn" =>
                $"\x1b[91m{attr}:\x1b[0m \x1b[91m{styledValue}\x1b[0m",
            "route" or "route6" or "inetnum" or "inet6num" or "prefix" or "network" =>
                boldColors
                    ? $"\x1b[96m{attr}:\x1b[0m \x1b[96m{styledValue}\x1b[0m"
                    : $"\x1b[36m{attr}:\x1b[0m \x1b[36m{styledValue}\x1b[0m",
            "status" or "rpki-status" or "validation" => ColorizeStatusValue(attr, value),
            "country" or "country-code" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{styledValue}\x1b[0m",
            "registry" or "rir" or "source" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{styledValue}\x1b[0m",
            "allocated" or "assigned" or "created" or "changed" =>
                boldColors
                    ? $"\x1b[35m{attr}:\x1b[0m \x1b[35m{styledValue}\x1b[0m"
                    : $"\x1b[95m{attr}:\x1b[0m \x1b[95m{styledValue}\x1b[0m",
            "netname" or "orgname" or "org-name" =>
                boldColors
                    ? $"\x1b[97m{attr}:\x1b[0m \x1b[97m{styledValue}\x1b[0m"
                    : $"\x1b[1;97m{attr}:\x1b[0m \x1b[1;97m{styledValue}\x1b[0m",
            "last-modified" or "expires" or "updated" =>
                $"\x1b[90m{attr}:\x1b[0m \x1b[90m{styledValue}\x1b[0m",
            "package" or "package-name" or "depends" or "makedepends" or "optdepends"
                or "checkdepends" or "provides" or "conflicts" or "replaces" or "architecture"
                or "license" or "maintainer" or "packager" =>
                boldColors
                    ? $"\x1b[36m{attr}:\x1b[0m \x1b[36m{styledValue}\x1b[0m"
                    : $"\x1b[96m{attr}:\x1b[0m \x1b[96m{styledValue}\x1b[0m",
            "description" or "pkgdesc" or "summary" =>
                boldColors
                    ? $"\x1b[35m{attr}:\x1b[0m \x1b[35m{styledValue}\x1b[0m"
                    : $"\x1b[95m{attr}:\x1b[0m \x1b[95m{styledValue}\x1b[0m",
            "size" or "installed-size" or "compress-size" or "download-size" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{styledValue}\x1b[0m",
            "url" or "homepage" or "aur-url" or "upstream-url" =>
                $"\x1b[1;94m{attr}:\x1b[0m \x1b[4;94m{styledValue}\x1b[0m",
            "priority" => ColorizePriorityValue(attr, value),
            "pre-depends" =>
                $"\x1b[94m{attr}:\x1b[0m \x1b[94m{styledValue}\x1b[0m",
            "recommends" =>
                $"\x1b[93m{attr}:\x1b[0m \x1b[93m{styledValue}\x1b[0m",
            "suggests" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{styledValue}\x1b[0m",
            "breaks" =>
                $"\x1b[91m{attr}:\x1b[0m \x1b[91m{styledValue}\x1b[0m",
            "contributor" =>
                $"\x1b[96m{attr}:\x1b[0m \x1b[96m{styledValue}\x1b[0m",
            "build-status" or "test-status" => ColorizeBuildStatusValue(attr, value),
            "epoch" or "release" or "pkgver" or "pkgrel" =>
                $"\x1b[1;93m{attr}:\x1b[0m \x1b[93m{styledValue}\x1b[0m",
            _ => ColorizeBgpToolsDefaultRainbow(attr, styledValue, boldColors),
        };
    }

    private static string ColorizePriorityValue(string attr, string value)
    {
        var lower = value.ToLowerInvariant();
        if (lower.Contains("required") || lower.Contains("important"))
        {
            return $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
        }

        if (lower.Contains("standard"))
        {
            return $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m";
        }

        return $"\x1b[94m{attr}:\x1b[0m \x1b[94m{value}\x1b[0m";
    }

    private static string ColorizeBuildStatusValue(string attr, string value)
    {
        var lower = value.ToLowerInvariant();
        if (lower.Contains("pass") || lower.Contains("success"))
        {
            return $"\x1b[92m{attr}:\x1b[0m \x1b[92m{value}\x1b[0m";
        }

        if (lower.Contains("fail") || lower.Contains("error"))
        {
            return $"\x1b[91m{attr}:\x1b[0m \x1b[91m{value}\x1b[0m";
        }

        return $"\x1b[93m{attr}:\x1b[0m \x1b[93m{value}\x1b[0m";
    }

    private static string ColorizeBgpToolsDefaultRainbow(string attr, string styledValue, bool boldColors)
    {
        var hash = attr.Sum(c => (uint)c);
        var colorCode = boldColors ? 31 + (hash % 6) : 91 + (hash % 6);
        return $"\x1b[{colorCode}m{attr}:\x1b[0m \x1b[{colorCode}m{styledValue}\x1b[0m";
    }

    private static string ColorizeQueryTypeContent(string line, QueryType queryType, bool boldColors)
    {
        var commentColor = boldColors ? "\x1b[37m" : "\x1b[90m";

        switch (queryType)
        {
            case QueryType.Domain or QueryType.IPv4 or QueryType.IPv6 or QueryType.Asn
                or QueryType.BareAsn or QueryType.Unknown:
                {
                    var result = AsnRegex.Replace(line, "\x1b[93m$1\x1b[0m");
                    result = IpRegex.Replace(result, "\x1b[92m$1\x1b[0m");
                    result = DomainRegex.Replace(result, "\x1b[94m$1\x1b[0m");
                    return line.StartsWith('%') ? $"{commentColor}{result}\x1b[0m" : result;
                }

            case QueryType.Desc:
                if (line.StartsWith('%'))
                {
                    return $"{commentColor}{line}\x1b[0m";
                }

                if (line.Contains("descr:") || line.Contains("description:") || line.Contains("remarks:"))
                {
                    return boldColors ? $"\x1b[37m{line}\x1b[0m" : $"\x1b[92m{line}\x1b[0m";
                }

                return line;

            case QueryType.Help:
                if (line.Contains("Help Information"))
                {
                    return boldColors ? $"\x1b[36m{line}\x1b[0m" : $"\x1b[1;96m{line}\x1b[0m";
                }

                if (line.Contains("Category:"))
                {
                    return $"\x1b[1;95m{line}\x1b[0m";
                }

                if (line.Contains("Example:") || line.Contains("Usage:"))
                {
                    return $"\x1b[92m{line}\x1b[0m";
                }

                return line.StartsWith('%') ? $"{commentColor}{line}\x1b[0m" : line;

            case QueryType.UpdatePatch:
            case QueryType.Rdap:
                return line.StartsWith('%') ? $"{commentColor}{line}\x1b[0m" : line;

            default:
                // Forward-compatible fallback for query types not yet given a dedicated arm here
                // (mirrors the Rust version's catch-all for feature-gated types) — general
                // ASN/IP/domain highlighting.
                {
                    var result = AsnRegex.Replace(line, "\x1b[93m$1\x1b[0m");
                    result = IpRegex.Replace(result, "\x1b[92m$1\x1b[0m");
                    result = DomainRegex.Replace(result, "\x1b[94m$1\x1b[0m");
                    return line.StartsWith('%') ? $"{commentColor}{result}\x1b[0m" : result;
                }
        }
    }
}
