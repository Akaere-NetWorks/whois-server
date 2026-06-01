/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::core::QueryType;
use crate::core::color::scheme::ColorScheme;
use regex::Regex;
use std::sync::LazyLock;

// Pre-compiled regex patterns for colorization
// These are hardcoded patterns validated at compile time by tests
static ASN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(AS\d+)").expect("Invalid ASN regex"));
static IP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+\.\d+\.\d+\.\d+(?:/\d+)?|[0-9a-fA-F:]+::[0-9a-fA-F:]*(?:/\d+)?)")
        .expect("Invalid IP regex")
});
static DOMAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}")
        .expect("Invalid domain regex")
});
static ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)").expect("Invalid ID regex"));
static URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(https?://[^\s]+)").expect("Invalid URL regex"));
static SIZE_BYTES_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)\s*bytes").expect("Invalid size bytes regex"));
static MS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)\s*ms").expect("Invalid ms regex"));
static NEGATIVE_MS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(-?\d+\.?\d*)\s*ms").expect("Invalid negative ms regex"));
static PRICE_DISCOUNT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\$[\d,]+\.?\d*|Free)").expect("Invalid price discount regex"));
static PRICE_ONLY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\$[\d,]+\.?\d*)").expect("Invalid price-only regex"));
static DISCOUNT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+%↓)").expect("Invalid discount regex"));
static IMDB_ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(tt\d+)").expect("Invalid IMDB ID regex"));
static YEAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})").expect("Invalid year regex"));
static MEASUREMENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+[\.\d]*\s*(cm|kg|m|ft|in))").expect("Invalid measurement regex")
});
static DURATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+\s*min|\d+\s*hours?)").expect("Invalid duration regex"));
static RATING_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+\.\d+/10)").expect("Invalid rating regex"));
static IPV4_ONLY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+\.\d+\.\d+\.\d+)").expect("Invalid IPv4 regex"));
static IPV6_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([0-9a-fA-F:]+::[0-9a-fA-F:]*)").expect("Invalid IPv6 regex"));

pub struct Colorizer {
    scheme: ColorScheme,
}

impl Colorizer {
    pub fn new(scheme: ColorScheme) -> Self {
        Self { scheme }
    }

    pub fn colorize_response(&self, response: &str, query_type: &QueryType) -> String {
        match self.scheme {
            ColorScheme::Ripe => self.colorize_ripe_style(response, query_type, true), // 深色字符
            ColorScheme::RipeDark => self.colorize_ripe_style(response, query_type, false), // 浅色字符
            ColorScheme::BgpTools => self.colorize_bgptools_style(response, query_type, true), // 深色字符
            ColorScheme::BgpToolsDark => self.colorize_bgptools_style(response, query_type, false), // 浅色字符
        }
    }

    // RIPE Style Colorization
    fn colorize_ripe_style(
        &self,
        response: &str,
        query_type: &QueryType,
        bold_colors: bool,
    ) -> String {
        let mut colorized = String::new();

        for line in response.lines() {
            let colored_line = if line.starts_with('%') {
                // Comments
                if bold_colors {
                    format!("\x1b[90m{}\x1b[0m", line) // Bright black for bold colors
                } else {
                    format!("\x1b[37m{}\x1b[0m", line) // Dim white for normal colors
                }
            } else if line.contains(':') && !line.starts_with(' ') {
                self.colorize_ripe_attributes(line, bold_colors)
            } else {
                self.colorize_query_type_content(line, query_type, bold_colors, false)
            };

            colorized.push_str(&colored_line);
            colorized.push_str("\r\n");
        }

        // Remove last CRLF if added
        if colorized.ends_with("\r\n") {
            colorized.truncate(colorized.len() - 2);
        }

        colorized
    }

    // BGPTools Style Colorization
    fn colorize_bgptools_style(
        &self,
        response: &str,
        query_type: &QueryType,
        bold_colors: bool,
    ) -> String {
        let mut colorized = String::new();

        for line in response.lines() {
            let colored_line = if line.starts_with('%') {
                // Comments
                if bold_colors {
                    format!("\x1b[90m{}\x1b[0m", line) // Bright black for bold colors
                } else {
                    format!("\x1b[37m{}\x1b[0m", line) // Dim white for normal colors
                }
            } else if line.contains(':') && !line.starts_with(' ') {
                self.colorize_bgptools_attributes(line, bold_colors)
            } else {
                self.colorize_query_type_content(line, query_type, bold_colors, true)
            };

            colorized.push_str(&colored_line);
            colorized.push_str("\r\n");
        }

        // Remove last CRLF if added
        if colorized.ends_with("\r\n") {
            colorized.truncate(colorized.len() - 2);
        }

        colorized
    }

    // RIPE Attribute Colorization
    fn colorize_ripe_attributes(&self, line: &str, bold_colors: bool) -> String {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return line.to_string();
        }

        let attr = parts[0].trim();
        let value = parts[1];

        match attr {
            // Network resources
            "inetnum" | "inet6num" | "route" | "route6" | "network" | "prefix" => {
                if bold_colors {
                    format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Bright cyan for bold colors
                } else {
                    format!("\x1b[36m{}:\x1b[0m \x1b[36m{}\x1b[0m", attr, value) // Cyan for normal colors
                }
            }
            // Domain related
            "domain" | "nserver" | "dns" => {
                if bold_colors {
                    format!("\x1b[1;96m{}:\x1b[0m \x1b[1;96m{}\x1b[0m", attr, value) // Bright cyan for bold colors
                } else {
                    format!("\x1b[36m{}:\x1b[0m \x1b[36m{}\x1b[0m", attr, value) // Cyan for normal colors
                }
            }
            // ASN info
            "origin" | "aut-num" | "as-name" | "asn" => {
                format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value)
            }
            // Contact info
            "person" | "admin-c" | "tech-c" | "mnt-by" | "contact" | "email" => {
                format!("\x1b[32m{}:\x1b[0m \x1b[32m{}\x1b[0m", attr, value)
            }
            // Name fields
            "netname" | "name" => {
                format!("\x1b[1;92m{}:\x1b[0m \x1b[1;92m{}\x1b[0m", attr, value)
            }
            // Organization
            "org" | "orgname" | "org-name" | "organisation" => {
                if bold_colors {
                    format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for dark mode
                } else {
                    format!("\x1b[33m{}:\x1b[0m \x1b[33m{}\x1b[0m", attr, value) // Orange for light mode
                }
            }
            // Description
            "descr" | "description" => {
                if bold_colors {
                    format!("\x1b[37m{}:\x1b[0m \x1b[37m{}\x1b[0m", attr, value) // Dim white for dark mode
                } else {
                    format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan for light mode
                }
            }
            // Geographic info
            "country" | "address" | "city" | "region" | "geoloc" => {
                if bold_colors {
                    format!("\x1b[35m{}:\x1b[0m \x1b[35m{}\x1b[0m", attr, value) // Magenta for dark mode
                } else {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta for light mode
                }
            }
            // Registrar info
            "registrar" | "sponsoring-registrar" | "registrant" => {
                if bold_colors {
                    format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue for dark mode
                } else {
                    format!("\x1b[1;94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Bright blue for light mode
                }
            }
            // Status/state - conditional colors
            "status" | "state" | "rpki-status" | "validation" => {
                let value_lower = value.trim().to_lowercase();
                if value_lower.contains("valid") && !value_lower.contains("invalid") {
                    format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Bright green for valid
                } else if value_lower.contains("invalid") {
                    format!("\x1b[1;91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Bright red for invalid
                } else {
                    format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Bright yellow for unknown
                }
            }
            // Dates
            "created" | "changed" | "last-modified" | "expires" | "updated" | "created-at"
            | "updated-at" | "pushed-at" => {
                if bold_colors {
                    format!("\x1b[35m{}:\x1b[0m \x1b[35m{}\x1b[0m", attr, value) // Magenta for dark mode
                } else {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta for light mode
                }
            }
            // Package managers
            "package" | "package-name" | "version" | "latest-version" | "stable-version"
            | "package-base" | "source-package" | "attribute-name" | "attribute-set" => {
                if bold_colors {
                    format!("\x1b[37m{}:\x1b[0m \x1b[37m{}\x1b[0m", attr, value) // Dim white for dark
                } else {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta for light
                }
            }
            "summary" | "long-description" | "nixpkgs-position" => {
                if bold_colors {
                    format!("\x1b[36m{}:\x1b[0m \x1b[36m{}\x1b[0m", attr, value) // Cyan for dark
                } else {
                    format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan for light
                }
            }
            "license" | "distribution" => {
                format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green
            }
            "size" | "filename" | "modified-time" | "unpacked-size" | "file-count"
            | "total-size" | "package-size" | "wheel-size" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow
            }
            // Statistics and metrics
            "popularity" | "votes" | "rating" | "score" | "stars" | "watchers" | "forks"
            | "open-issues" | "downloads" | "total-downloads" | "recent-downloads"
            | "followers" | "following" | "views" | "likes" | "bookmarks" | "reposts" => {
                format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta
            }
            // URLs
            "aur-url" | "upstream-url" | "url" | "homepage" | "ubuntu-url" | "nixos-url"
            | "opensuse-url" | "npm-url" | "registry-url" | "pypi-url" | "crates-io-url"
            | "docs-rs-url" | "api-url" | "github-url" | "clone-url" | "ssh-url" | "avatar-url"
            | "profileurl" | "steam-url" | "website" | "metacritic-url" | "wikipedia-url"
            | "edit-url" => {
                let colored_value = URL_REGEX
                    .replace_all(value, "\x1b[4;94m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;94m{}:\x1b[0m {}", attr, colored_value)
            }
            // Gaming specific
            "app-id" | "steamid" | "game-id" | "metacritic-score" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;93m{}:\x1b[0m {}", attr, colored_value)
            }
            "price" | "original-price" => {
                if value.contains("(%↓)") || value.contains("Free") {
                    // Green for discounted games and free games
                    let colored_value = PRICE_DISCOUNT_REGEX
                        .replace_all(value, "\x1b[1;92m$1\x1b[0m")
                        .to_string();
                    let final_value = DISCOUNT_REGEX
                        .replace_all(&colored_value, "\x1b[1;92m$1\x1b[0m")
                        .to_string();
                    format!("\x1b[1;95m{}:\x1b[0m{}", attr, final_value)
                } else {
                    // White for full-price games (no discount)
                    let colored_value = PRICE_ONLY_REGEX
                        .replace_all(value, "\x1b[97m$1\x1b[0m")
                        .to_string();
                    format!("\x1b[1;95m{}:\x1b[0m{}", attr, colored_value)
                }
            }
            "players" | "players-online" | "max-players" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;95m$1\x1b[0m")
                    .to_string();
                format!("\x1b[95m{}:\x1b[0m {}", attr, colored_value)
            }
            "latency" | "ping" | "round-trip" => {
                let colored_value = MS_REGEX
                    .replace_all(value, |caps: &regex::Captures| {
                        let ms: u32 = caps[1].parse().unwrap_or(0);
                        if ms < 50 {
                            format!("\x1b[1;92m{}ms\x1b[0m", ms) // Green for good latency
                        } else if ms < 150 {
                            format!("\x1b[1;93m{}ms\x1b[0m", ms) // Yellow for moderate latency
                        } else {
                            format!("\x1b[1;91m{}ms\x1b[0m", ms) // Red for high latency
                        }
                    })
                    .to_string();
                format!("\x1b[93m{}:\x1b[0m {}", attr, colored_value)
            }
            // IMDb specific
            "imdb-id" | "tt-id" => {
                let colored_value = IMDB_ID_REGEX
                    .replace_all(value, "\x1b[1;93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;93m{}:\x1b[0m {}", attr, colored_value)
            }
            "movie-title" | "series-title" | "game-title" => {
                if bold_colors {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta for dark
                } else {
                    format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan for light
                }
            }
            "year" | "release-year" | "release-date" => {
                let colored_value = YEAR_REGEX
                    .replace_all(value, "\x1b[1;93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[93m{}:\x1b[0m {}", attr, colored_value)
            }
            "metascore" | "box-office" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value)
            }
            "director" | "writer" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue
            }
            "actors" | "cast" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan
            }
            "genre" | "genres" | "categories" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta
            }
            "awards" => {
                format!("\x1b[1;93m{}:\x1b[0m \x1b[1;93m{}\x1b[0m", attr, value) // Bright yellow
            }
            "rated" | "mpaa-rating" => {
                if value.contains("PG") || value.contains("G") {
                    format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green for family-friendly
                } else if value.contains("R") || value.contains("NC-17") {
                    format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red for mature content
                } else {
                    format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for other ratings
                }
            }
            // GitHub specific
            "repository-name" | "repo-name" | "full-name" | "repo" => {
                format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Bright cyan
            }
            "owner" | "username" | "user" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta
            }
            "language" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue
            }
            "default-branch" | "branch" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow
            }
            "visibility" | "private" | "public" => {
                if value.contains("Public") || value.contains("false") {
                    format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green for public
                } else {
                    format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red for private
                }
            }
            // Wikipedia specific
            "page-id" | "article-id" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;93m{}:\x1b[0m {}", attr, colored_value)
            }
            "article-length" | "page-length" => {
                let colored_value = SIZE_BYTES_REGEX
                    .replace_all(value, "\x1b[1;93m$1 bytes\x1b[0m")
                    .to_string();
                format!("\x1b[95m{}:\x1b[0m {}", attr, colored_value)
            }
            "last-edited" => {
                if bold_colors {
                    format!("\x1b[90m{}:\x1b[0m \x1b[90m{}\x1b[0m", attr, value) // Gray for timestamps
                } else {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta for light
                }
            }
            "languages" => {
                format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green
            }
            // Pixiv specific
            "artwork-id" | "illust-id" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;94m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;94m{}:\x1b[0m {}", attr, colored_value) // Bright blue
            }
            "user-id" | "artist-id" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;95m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;95m{}:\x1b[0m {}", attr, colored_value) // Bright magenta
            }
            "artwork-title" => {
                format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Bright cyan
            }
            "artwork-url" | "profile-url" => {
                let colored_value = URL_REGEX
                    .replace_all(value, "\x1b[4;94m$1\x1b[0m")
                    .to_string();
                format!("\x1b[1;94m{}:\x1b[0m {}", attr, colored_value)
            }
            "content-rating" => {
                let value_lower = value.to_lowercase();
                if value_lower.contains("safe") {
                    format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green for safe
                } else if value_lower.contains("r-18") || value_lower.contains("r18") {
                    format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red for R-18
                } else {
                    format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for other ratings
                }
            }
            // ACGC (Anime/Comic/Game Characters)
            "character-name" | "character" => {
                format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta
            }
            "voice-actor" | "cv" | "seiyuu" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue
            }
            "source-work" | "series" | "anime" | "manga" | "game" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan
            }
            "personality" | "traits" | "moe-points" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta
            }
            "species" | "race" | "identity" | "class" | "level" => {
                format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green
            }
            "ability" | "skill" | "power" | "weapon" | "equipment" => {
                format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red
            }
            "alias" | "nickname" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow
            }
            "age" | "birthday" => {
                let colored_value = ID_REGEX
                    .replace_all(value, "\x1b[1;93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[95m{}:\x1b[0m {}", attr, colored_value)
            }
            "height" | "weight" | "bwh" => {
                let colored_value = MEASUREMENT_REGEX
                    .replace_all(value, "\x1b[1;92m$1\x1b[0m")
                    .to_string();
                format!("\x1b[95m{}:\x1b[0m {}", attr, colored_value)
            }
            "hair-color" | "eye-color" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow
            }
            "gender" => {
                let value_lower = value.to_lowercase();
                if value.contains("女") || value_lower.contains("female") {
                    format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta for female
                } else if value.contains("男") || value_lower.contains("male") {
                    format!("\x1b[1;94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Bright blue for male
                } else {
                    format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Bright cyan for other
                }
            }
            // Lyric specific
            "song-name" | "song" | "track" => {
                format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Bright magenta
            }
            "singer" | "artist" | "vocalist" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan
            }
            "author" | "lyricist" | "composer" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue
            }
            "lyric-content" | "lyrics" => {
                format!("\x1b[1;37m{}:\x1b[0m \x1b[1;37m{}\x1b[0m", attr, value) // Bold white
            }
            // Meal specific
            "meal-name" | "dish" => {
                if bold_colors {
                    format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for dark
                } else {
                    format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green for light
                }
            }
            "category" | "meal-type" => {
                format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green
            }
            "cuisine" | "cooking-style" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta
            }
            "ingredient" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue
            }
            "instruction" | "step" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan
            }
            "cooking-time" | "prep-time" => {
                let colored_value = DURATION_REGEX
                    .replace_all(value, "\x1b[93m$1\x1b[0m")
                    .to_string();
                format!("\x1b[93m{}:\x1b[0m {}", attr, colored_value)
            }
            // Network and routing
            "mp-import" | "mp-export" | "import" | "export" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for routing policies
            }
            "policy" | "filter" | "pref" | "med" | "local-pref" => {
                format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value) // Magenta for BGP attributes
            }
            "member-of" | "members" | "as-set" | "route-set" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue for sets
            }
            "mnt-lower" | "mnt-routes" | "mnt-domains" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value) // Cyan for maintainers
            }
            // NTP specific
            "stratum" => {
                let colored_value = ID_REGEX
                    .replace_all(value, |caps: &regex::Captures| {
                        let stratum: u32 = caps[1].parse().unwrap_or(16);
                        if stratum <= 2 {
                            format!("\x1b[1;92m{}\x1b[0m", stratum) // Green for stratum 1-2
                        } else if stratum <= 4 {
                            format!("\x1b[1;93m{}\x1b[0m", stratum) // Yellow for stratum 3-4
                        } else {
                            format!("\x1b[1;37m{}\x1b[0m", stratum) // White for others
                        }
                    })
                    .to_string();
                format!("\x1b[95m{}:\x1b[0m {}", attr, colored_value)
            }
            "offset" | "root-delay" | "root-dispersion" => {
                let colored_value = NEGATIVE_MS_REGEX
                    .replace_all(value, |caps: &regex::Captures| {
                        let offset: f64 = caps[1].parse().unwrap_or(999.0);
                        let abs_offset = offset.abs();
                        if abs_offset < 10.0 {
                            format!("\x1b[1;92m{}ms\x1b[0m", offset) // Green for <10ms
                        } else if abs_offset < 100.0 {
                            format!("\x1b[1;93m{}ms\x1b[0m", offset) // Yellow for 10-100ms
                        } else {
                            format!("\x1b[1;91m{}ms\x1b[0m", offset) // Red for >100ms
                        }
                    })
                    .to_string();
                format!("\x1b[94m{}:\x1b[0m {}", attr, colored_value)
            }
            "delay" | "reach" | "jitter" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow
            }
            // Default - rainbow gradient effect for unknown attributes
            _ => {
                let hash = attr.chars().map(|c| c as u32).sum::<u32>();
                let color_code = 31 + (hash % 6); // Rotate through 31-36 (red to cyan)
                format!(
                    "\x1b[{}m{}:\x1b[0m \x1b[{}m{}\x1b[0m",
                    color_code, attr, color_code, value
                )
            }
        }
    }

    // BGPTools Attribute Colorization
    fn colorize_bgptools_attributes(&self, line: &str, bold_colors: bool) -> String {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return line.to_string();
        }

        let attr = parts[0].trim();
        let value = parts[1];

        let asn_color = "\x1b[93m"; // Yellow
        let ip_color = "\x1b[92m"; // Green
        let domain_color = "\x1b[94m"; // Blue

        let mut styled_value = value.to_string();
        styled_value = ASN_REGEX
            .replace_all(&styled_value, format!("{}$1\x1b[0m", asn_color).as_str())
            .to_string();
        styled_value = IP_REGEX
            .replace_all(&styled_value, format!("{}$1\x1b[0m", ip_color).as_str())
            .to_string();
        styled_value = DOMAIN_REGEX
            .replace_all(&styled_value, format!("{}$1\x1b[0m", domain_color).as_str())
            .to_string();

        match attr {
            // AS related - bright red (AS column in reference)
            "origin" | "aut-num" | "as-name" | "asn" => {
                format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, styled_value)
            }
            // Network/IP info - bright cyan (IP/Prefix column in reference)
            "route" | "route6" | "inetnum" | "inet6num" | "prefix" | "network" => {
                if bold_colors {
                    format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value) // Bright cyan
                } else {
                    format!("\x1b[36m{}:\x1b[0m \x1b[36m{}\x1b[0m", attr, styled_value) // Cyan
                }
            }
            // Status/validation - conditional colors
            "status" | "rpki-status" | "validation" => {
                let value_lower = value.trim().to_lowercase();
                if value_lower.contains("valid") && !value_lower.contains("invalid") {
                    format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Bright green
                } else if value_lower.contains("invalid") {
                    format!("\x1b[1;91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Bright red
                } else {
                    format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Bright yellow
                }
            }
            // Country info - bright yellow (Country Code column in reference)
            "country" | "country-code" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value)
            }
            // Registry info - bright blue (Registry column in reference)
            "registry" | "rir" | "source" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, styled_value)
            }
            // Allocation info - bright magenta (Allocated column in reference)
            "allocated" | "assigned" | "created" | "changed" => {
                if bold_colors {
                    format!("\x1b[35m{}:\x1b[0m \x1b[35m{}\x1b[0m", attr, styled_value) // Magenta
                } else {
                    format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, styled_value) // Bright magenta
                }
            }
            // AS Names and org names - bright white bold (AS Name column in reference)
            "netname" | "orgname" | "org-name" => {
                if bold_colors {
                    format!("\x1b[97m{}:\x1b[0m \x1b[97m{}\x1b[0m", attr, styled_value) // White
                } else {
                    format!(
                        "\x1b[1;97m{}:\x1b[0m \x1b[1;97m{}\x1b[0m",
                        attr, styled_value
                    ) // Bright white
                }
            }
            // Dates - gray (non-allocation dates)
            "last-modified" | "expires" | "updated" => {
                format!("\x1b[90m{}:\x1b[0m \x1b[90m{}\x1b[0m", attr, styled_value)
            }
            // Package info - bright cyan
            "package" | "package-name" | "depends" | "makedepends" | "optdepends"
            | "checkdepends" | "provides" | "conflicts" | "replaces" | "architecture"
            | "license" | "maintainer" | "packager" => {
                if bold_colors {
                    format!("\x1b[36m{}:\x1b[0m \x1b[36m{}\x1b[0m", attr, styled_value) // Cyan
                } else {
                    format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value) // Bright cyan
                }
            }
            // Package descriptions - bright magenta
            "description" | "pkgdesc" | "summary" => {
                if bold_colors {
                    format!("\x1b[35m{}:\x1b[0m \x1b[35m{}\x1b[0m", attr, styled_value) // Magenta
                } else {
                    format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, styled_value) // Bright magenta
                }
            }
            // Size and metadata - bright yellow
            "size" | "installed-size" | "compress-size" | "download-size" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value)
            }
            // URLs - underlined blue
            "url" | "homepage" | "aur-url" | "upstream-url" => {
                format!(
                    "\x1b[1;94m{}:\x1b[0m \x1b[4;94m{}\x1b[0m",
                    attr, styled_value
                )
            }
            // Priority - conditional colors
            "priority" => {
                let value_lower = value.to_lowercase();
                if value_lower.contains("required") || value_lower.contains("important") {
                    format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red for critical
                } else if value_lower.contains("standard") {
                    format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for standard
                } else {
                    format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value) // Blue for optional
                }
            }
            // Dependencies - different colors for different types
            "pre-depends" => {
                format!("\x1b[94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, styled_value) // Blue for required deps
            }
            "recommends" => {
                format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value) // Yellow for recommends
            }
            "suggests" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value) // Cyan for suggests
            }
            "breaks" => {
                format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, styled_value) // Red for conflicts
            }
            // Maintainers and packagers
            "contributor" => {
                format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value) // Cyan
            }
            // Build and test status
            "build-status" | "test-status" => {
                let value_lower = value.to_lowercase();
                if value_lower.contains("pass") || value_lower.contains("success") {
                    format!("\x1b[92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Green for success
                } else if value_lower.contains("fail") || value_lower.contains("error") {
                    format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Red for failure
                } else {
                    format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Yellow for unknown/pending
                }
            }
            // Version info
            "epoch" | "release" | "pkgver" | "pkgrel" => {
                format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value) // Bright yellow
            }
            // Default - gradient rainbow
            _ => {
                let hash = attr.chars().map(|c| c as u32).sum::<u32>();
                let color_code = if bold_colors {
                    31 + (hash % 6) // Normal colors 31-36 for dark mode
                } else {
                    91 + (hash % 6) // Bright colors 91-96 for light mode
                };
                format!(
                    "\x1b[{}m{}:\x1b[0m \x1b[{}m{}\x1b[0m",
                    color_code, attr, color_code, styled_value
                )
            }
        }
    }

    // Query Type Specific Content Colorization
    fn colorize_query_type_content(
        &self,
        line: &str,
        query_type: &QueryType,
        bold_colors: bool,
        _is_bgptools: bool,
    ) -> String {
        let comment_color = if bold_colors { "\x1b[37m" } else { "\x1b[90m" }; // Dim white vs bright black

        match query_type {
            QueryType::Domain(_)
            | QueryType::IPv4(_)
            | QueryType::IPv6(_)
            | QueryType::ASN(_)
            | QueryType::BareASN(_)
            | QueryType::Unknown(_) => {
                // General network highlighting for domain/IP/ASN queries
                let mut result = ASN_REGEX.replace_all(line, "\x1b[93m$1\x1b[0m").to_string();
                result = IP_REGEX
                    .replace_all(&result, "\x1b[92m$1\x1b[0m")
                    .to_string();
                result = DOMAIN_REGEX
                    .replace_all(&result, "\x1b[94m$1\x1b[0m")
                    .to_string();

                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, result)
                } else {
                    result
                }
            }

            QueryType::Desc(_) => {
                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else if line.contains("descr:")
                    || line.contains("description:")
                    || line.contains("remarks:")
                {
                    if bold_colors {
                        format!("\x1b[37m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[92m{}\x1b[0m", line)
                    }
                } else {
                    line.to_string()
                }
            }

            QueryType::Help => {
                if line.contains("Help Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("Category:") {
                    format!("\x1b[1;95m{}\x1b[0m", line)
                } else if line.contains("Example:") || line.contains("Usage:") {
                    format!("\x1b[92m{}\x1b[0m", line)
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            QueryType::UpdatePatch => {
                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            QueryType::Rdap(_) => {
                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "geo")]
            QueryType::Geo(_) | QueryType::RirGeo(_) => {
                if line.contains("latitude")
                    || line.contains("longitude")
                    || line.contains("coordinates")
                {
                    if bold_colors {
                        format!("\x1b[35m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[95m{}\x1b[0m", line)
                    }
                } else if line.contains("location")
                    || line.contains("city")
                    || line.contains("region")
                {
                    format!("\x1b[94m{}\x1b[0m", line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "bgp")]
            QueryType::BGPTool(_)
            | QueryType::Irr(_)
            | QueryType::LookingGlass(_)
            | QueryType::Rpki(_, _) => {
                let mut result = ASN_REGEX.replace_all(line, "\x1b[93m$1\x1b[0m").to_string();
                result = IP_REGEX
                    .replace_all(&result, "\x1b[92m$1\x1b[0m")
                    .to_string();
                result
            }

            #[cfg(feature = "bgp-prefixes")]
            QueryType::Prefixes(_) => {
                let mut result = ASN_REGEX.replace_all(line, "\x1b[93m$1\x1b[0m").to_string();
                result = IP_REGEX
                    .replace_all(&result, "\x1b[92m$1\x1b[0m")
                    .to_string();
                result
            }

            #[cfg(feature = "dns")]
            QueryType::Dns(_) => {
                if line.contains("DNS Resolution Results") || line.contains("Query:") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains(" A ") && !line.contains("AAAA") {
                    IPV4_ONLY_REGEX
                        .replace_all(line, "\x1b[92m$1\x1b[0m")
                        .to_string()
                } else if line.contains(" AAAA ") {
                    IPV6_REGEX
                        .replace_all(line, "\x1b[92m$1\x1b[0m")
                        .to_string()
                } else if line.contains(" CNAME ") || line.contains(" DNAME ") {
                    format!("\x1b[94m{}\x1b[0m", line)
                } else if line.contains(" MX ") {
                    format!("\x1b[95m{}\x1b[0m", line)
                } else if line.contains(" NS ") {
                    format!("\x1b[96m{}\x1b[0m", line)
                } else if line.contains(" TXT ") || line.contains(" SPF ") {
                    format!("\x1b[93m{}\x1b[0m", line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "ssl")]
            QueryType::Ssl(_) => {
                if line.contains("Certificate Information") || line.contains("SSL Certificate") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("Certificate Status:") {
                    if line.contains("Valid") {
                        format!("\x1b[92m{}\x1b[0m", line)
                    } else if line.contains("Expired") || line.contains("Invalid") {
                        format!("\x1b[91m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[93m{}\x1b[0m", line)
                    }
                } else if line.contains("Subject:") || line.contains("Issuer:") {
                    if bold_colors {
                        format!("\x1b[35m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[95m{}\x1b[0m", line)
                    }
                } else if line.contains("Not Before:") || line.contains("Not After:") {
                    if line.contains("Not After:") && line.contains("202") {
                        format!("\x1b[93m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[90m{}\x1b[0m", line)
                    }
                } else if line.contains("SHA") || line.contains("Fingerprint") {
                    format!("\x1b[96m{}\x1b[0m", line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "steam")]
            QueryType::Steam(_) | QueryType::SteamSearch(_) => {
                if line.contains("Steam Application Information")
                    || line.contains("Steam Game Search Results")
                {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("price:") {
                    if line.contains("(%↓)") || line.contains("Free") {
                        let colored = PRICE_DISCOUNT_REGEX
                            .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                            .to_string();
                        DISCOUNT_REGEX
                            .replace_all(&colored, "\x1b[1;92m$1\x1b[0m")
                            .to_string()
                    } else {
                        if bold_colors {
                            PRICE_ONLY_REGEX
                                .replace_all(line, "\x1b[91m$1\x1b[0m")
                                .to_string()
                        } else {
                            PRICE_ONLY_REGEX
                                .replace_all(line, "\x1b[97m$1\x1b[0m")
                                .to_string()
                        }
                    }
                } else if line.contains("Status:") {
                    if line.contains("Online") {
                        format!("\x1b[1;92m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;91m{}\x1b[0m", line)
                    }
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "imdb")]
            QueryType::Imdb(_) | QueryType::ImdbSearch(_) => {
                if line.contains("IMDb") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("imdb-rating:") {
                    if line.contains("8.") || line.contains("9.") {
                        RATING_REGEX
                            .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                            .to_string()
                    } else if line.contains("7.") {
                        RATING_REGEX
                            .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                            .to_string()
                    } else {
                        RATING_REGEX
                            .replace_all(line, "\x1b[1;91m$1\x1b[0m")
                            .to_string()
                    }
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "minecraft")]
            QueryType::Minecraft(_) => {
                if line.contains("Minecraft Server Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("status:") || line.contains("Status:") {
                    if line.to_lowercase().contains("online") {
                        format!("\x1b[1;92m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;91m{}\x1b[0m", line)
                    }
                } else if line.contains("players:") || line.contains("Players:") {
                    ID_REGEX
                        .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                        .to_string()
                } else if line.contains("latency:") || line.contains("ms") {
                    MS_REGEX
                        .replace_all(line, |caps: &regex::Captures| {
                            let ms: u32 = caps[1].parse().unwrap_or(0);
                            if ms < 50 {
                                format!("\x1b[1;92m{}ms\x1b[0m", ms)
                            } else if ms < 150 {
                                format!("\x1b[1;93m{}ms\x1b[0m", ms)
                            } else {
                                format!("\x1b[1;91m{}ms\x1b[0m", ms)
                            }
                        })
                        .to_string()
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "github")]
            QueryType::GitHub(_) => {
                if line.contains("GitHub Repository Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("visibility:") {
                    if line.contains("Public") {
                        format!("\x1b[92m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[91m{}\x1b[0m", line)
                    }
                } else if line.contains("stars:")
                    || line.contains("watchers:")
                    || line.contains("forks:")
                {
                    ID_REGEX
                        .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                        .to_string()
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "wikipedia")]
            QueryType::Wikipedia(_) => {
                if line.contains("Wikipedia Article Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("article-length:") {
                    SIZE_BYTES_REGEX
                        .replace_all(line, "\x1b[1;93m$1 bytes\x1b[0m")
                        .to_string()
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "pixiv")]
            QueryType::Pixiv(_) => {
                if line.contains("Pixiv Artwork Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("rating:") {
                    let line_lower = line.to_lowercase();
                    if line_lower.contains("safe") {
                        format!("\x1b[92m{}\x1b[0m", line)
                    } else if line_lower.contains("r-18") {
                        format!("\x1b[91m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[93m{}\x1b[0m", line)
                    }
                } else if line.contains("views:")
                    || line.contains("likes:")
                    || line.contains("bookmarks:")
                {
                    ID_REGEX
                        .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                        .to_string()
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "acgc")]
            QueryType::Acgc(_) => {
                if line.contains("ACGC Character Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("character-name:") {
                    format!("\x1b[1;95m{}\x1b[0m", line)
                } else if line.contains("voice-actor:") || line.contains("cv:") {
                    format!("\x1b[94m{}\x1b[0m", line)
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "lyric")]
            QueryType::Lyric(_) => {
                if line.contains("Luotianyi Random Lyric") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("lyric-content:") {
                    format!("\x1b[1;37m{}\x1b[0m", line)
                } else if !line.trim().is_empty() && !line.contains(":") && !line.starts_with("%") {
                    format!("\x1b[1;92m{}\x1b[0m", line)
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "meal")]
            QueryType::Meal | QueryType::MealCN => {
                if line.contains("Meal Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("meal-name:") {
                    format!("\x1b[1;92m{}\x1b[0m", line)
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "ntp")]
            QueryType::Ntp(_) => {
                if line.contains("NTP Time Information") {
                    if bold_colors {
                        format!("\x1b[36m{}\x1b[0m", line)
                    } else {
                        format!("\x1b[1;96m{}\x1b[0m", line)
                    }
                } else if line.contains("stratum:") {
                    ID_REGEX
                        .replace_all(line, |caps: &regex::Captures| {
                            let stratum: u32 = caps[1].parse().unwrap_or(16);
                            if stratum <= 2 {
                                format!("\x1b[1;92m{}\x1b[0m", stratum)
                            } else if stratum <= 4 {
                                format!("\x1b[1;93m{}\x1b[0m", stratum)
                            } else {
                                format!("\x1b[1;37m{}\x1b[0m", stratum)
                            }
                        })
                        .to_string()
                } else if line.contains("offset:") {
                    NEGATIVE_MS_REGEX
                        .replace_all(line, |caps: &regex::Captures| {
                            let offset: f64 = caps[1].parse().unwrap_or(999.0);
                            let abs_offset = offset.abs();
                            if abs_offset < 10.0 {
                                format!("\x1b[1;92m{}ms\x1b[0m", offset)
                            } else if abs_offset < 100.0 {
                                format!("\x1b[1;93m{}ms\x1b[0m", offset)
                            } else {
                                format!("\x1b[1;91m{}ms\x1b[0m", offset)
                            }
                        })
                        .to_string()
                } else if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(feature = "plugins")]
            QueryType::Plugin(_, _) => {
                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, line)
                } else {
                    line.to_string()
                }
            }

            #[cfg(any(
                feature = "email",
                feature = "bgp-irr",
                feature = "bgp-manrs",
                feature = "peeringdb",
                feature = "bgp-prefixes",
                feature = "ping",
                feature = "traceroute",
                feature = "crt",
                feature = "cfstatus",
                feature = "pkg-alma",
                feature = "pkg-aosc",
                feature = "pkg-aur",
                feature = "pkg-cargo",
                feature = "pkg-debian",
                feature = "pkg-epel",
                feature = "pkg-modrinth",
                feature = "pkg-nixos",
                feature = "pkg-npm",
                feature = "pkg-opensuse",
                feature = "pkg-openwrt",
                feature = "pkg-pypi",
                feature = "pkg-ubuntu",
                feature = "pkg-curseforge",
                feature = "icp",
                feature = "rdap-explicit",
            ))]
            _ => {
                // General network highlighting for feature-gated query types
                let mut result = ASN_REGEX.replace_all(line, "\x1b[93m$1\x1b[0m").to_string();
                result = IP_REGEX
                    .replace_all(&result, "\x1b[92m$1\x1b[0m")
                    .to_string();
                result = DOMAIN_REGEX
                    .replace_all(&result, "\x1b[94m$1\x1b[0m")
                    .to_string();

                if line.starts_with("%") {
                    format!("{}{}\x1b[0m", comment_color, result)
                } else {
                    result
                }
            }
        }
    }
}
