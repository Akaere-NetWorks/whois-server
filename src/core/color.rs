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

use regex::Regex;
use crate::core::QueryType;

#[derive(Debug, Clone, PartialEq)]
pub enum ColorScheme {
    Ripe,
    BgpTools,
}

impl ColorScheme {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ripe" => Some(ColorScheme::Ripe),
            "bgptools" => Some(ColorScheme::BgpTools),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorProtocol {
    pub enabled: bool,
    pub scheme: Option<ColorScheme>,
    pub client_supports_color: bool,
}

impl Default for ColorProtocol {
    fn default() -> Self {
        Self {
            enabled: true,
            scheme: None,
            client_supports_color: false,
        }
    }
}

impl ColorProtocol {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_headers(&mut self, request: &str) -> bool {
        let lines: Vec<&str> = request.lines().collect();

        for line in &lines {
            let line = line.trim();

            // Check for capability probe
            if line.to_uppercase().starts_with("X-WHOIS-COLOR-PROBE:") {
                self.client_supports_color = true;
                return true; // This is a capability probe request
            }

            // Check for color scheme request
            if line.to_uppercase().starts_with("X-WHOIS-COLOR:") {
                if let Some(value_part) = line.split(':').nth(1) {
                    let value_part = value_part.trim();

                    // Support both formats: "ripe" and "scheme=ripe"
                    let scheme_str = if value_part.starts_with("scheme=") {
                        &value_part[7..] // Remove "scheme=" prefix
                    } else {
                        value_part
                    };

                    if let Some(scheme) = ColorScheme::from_string(scheme_str) {
                        self.scheme = Some(scheme);
                        self.client_supports_color = true;
                    }
                }
            }
        }

        false // Not a capability probe
    }

    pub fn should_colorize(&self) -> bool {
        self.enabled && self.client_supports_color && self.scheme.is_some()
    }

    pub fn get_capability_response(&self) -> String {
        if self.enabled {
            "X-WHOIS-COLOR-SUPPORT: 1.0 schemes=ripe,bgptools\r\n\r\n".to_string()
        } else {
            "X-WHOIS-COLOR-SUPPORT: no\r\n\r\n".to_string()
        }
    }
}

pub struct Colorizer {
    scheme: ColorScheme,
}

impl Colorizer {
    pub fn new(scheme: ColorScheme) -> Self {
        Self { scheme }
    }

    pub fn colorize_response(&self, response: &str, query_type: &QueryType) -> String {
        match self.scheme {
            ColorScheme::Ripe => self.colorize_ripe_style(response, query_type),
            ColorScheme::BgpTools => self.colorize_bgptools_style(response, query_type),
        }
    }

    fn colorize_ripe_style(&self, response: &str, query_type: &QueryType) -> String {
        let mut colorized = String::new();

        for line in response.lines() {
            let colored_line = if line.starts_with('%') {
                // Comments in bright black (gray) - matches reference implementation
                format!("\x1b[90m{}\x1b[0m", line)
            } else if line.contains(':') && !line.starts_with(' ') {
                // Attribute-value pairs with specific colors based on query type and attribute
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let attr = parts[0].trim();
                    let value = parts[1];

                    match attr {
                        // Network resources - bright cyan (matches reference)
                        "inetnum" | "inet6num" | "route" | "route6" | "network" | "prefix" => {
                            format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value)
                        }
                        // Domain related - bright cyan bold (matches reference)
                        "domain" | "nserver" | "dns" => {
                            format!("\x1b[1;96m{}:\x1b[0m \x1b[1;96m{}\x1b[0m", attr, value)
                        }
                        // ASN info - bright yellow (matches reference)
                        "origin" | "aut-num" | "as-name" | "asn" => {
                            format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value)
                        }
                        // Contact info - green (matches reference)
                        "person" | "admin-c" | "tech-c" | "mnt-by" | "contact" | "email" => {
                            format!("\x1b[32m{}:\x1b[0m \x1b[32m{}\x1b[0m", attr, value)
                        }
                        // Name fields - bright green bold (matches reference)
                        "netname" | "name" => {
                            format!("\x1b[1;92m{}:\x1b[0m \x1b[1;92m{}\x1b[0m", attr, value)
                        }
                        // Organization - yellow (matches reference)
                        "org" | "orgname" | "org-name" | "organisation" => {
                            format!("\x1b[33m{}:\x1b[0m \x1b[33m{}\x1b[0m", attr, value)
                        }
                        // Description - bright cyan
                        "descr" | "description" => {
                            format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value)
                        }
                        // Geographic info - bright magenta (date fields in reference)
                        "country" | "address" | "city" | "region" | "geoloc" => {
                            format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value)
                        }
                        // Registrar info - bright blue (matches reference)
                        "registrar" | "sponsoring-registrar" | "registrant" => {
                            format!("\x1b[1;94m{}:\x1b[0m \x1b[94m{}\x1b[0m", attr, value)
                        }
                        // Status/state - conditional colors
                        "status" | "state" | "rpki-status" | "validation" => {
                            if
                                value.trim().to_lowercase().contains("valid") &&
                                !value.trim().to_lowercase().contains("invalid")
                            {
                                format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Bright green for valid
                            } else if value.trim().to_lowercase().contains("invalid") {
                                format!("\x1b[1;91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Bright red for invalid
                            } else {
                                format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Bright yellow for unknown
                            }
                        }
                        // Dates - bright magenta (matches reference)
                        | "created"
                        | "changed"
                        | "last-modified"
                        | "expires"
                        | "updated"
                        | "created-at"
                        | "updated-at"
                        | "pushed-at" => {
                            format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value)
                        }
                        // Price information - conditional colors for Steam
                        "price" | "original-price" => {
                            if value.contains("(%↓)") || value.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let colored_value = price_regex
                                    .replace_all(value, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                let final_value = discount_regex
                                    .replace_all(&colored_value, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[1;95m{}:\x1b[0m{}", attr, final_value)
                            } else {
                                // White for full-price games (no discount)
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                let colored_value = price_regex
                                    .replace_all(value, "\x1b[97m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[1;95m{}:\x1b[0m{}", attr, colored_value)
                            }
                        }
                        // Package specific - bright magenta
                        | "version"
                        | "package"
                        | "package-base"
                        | "package-name"
                        | "attribute-name"
                        | "attribute-set"
                        | "component"
                        | "source-package"
                        | "source-version"
                        | "section"
                        | "priority"
                        | "project"
                        | "repository"
                        | "release"
                        | "architecture"
                        | "platforms"
                        | "outputs"
                        | "maintainers"
                        | "author"
                        | "depends"
                        | "replaces"
                        | "breaks"
                        | "provides"
                        | "suggests"
                        | "upstream"
                        | "upstream-version"
                        | "architectures"
                        | "aosc-url"
                        | "latest-version"
                        | "beta-version"
                        | "keywords"
                        | "dependencies"
                        | "dev-dependencies"
                        | "requires-python"
                        | "programming-languages"
                        | "development-status"
                        | "crate-name"
                        | "stable-version"
                        | "published-by"
                        | "yanked"
                        | "package-size"
                        | "total-downloads"
                        | "recent-downloads"
                        | "categories"
                        | "total-versions"
                        | "recent-versions"
                        | "registry"
                        | "username"
                        | "user-id"
                        | "user-type"
                        | "display-name"
                        | "bio"
                        | "company"
                        | "location"
                        | "twitter"
                        | "public-repos"
                        | "public-gists"
                        | "followers"
                        | "following"
                        | "repository-name"
                        | "full-name"
                        | "repository-id"
                        | "owner"
                        | "owner-type"
                        | "language"
                        | "default-branch"
                        | "stars"
                        | "watchers"
                        | "forks"
                        | "open-issues"
                        | "visibility"
                        | "features"
                        | "topics" => {
                            format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, value)
                        }
                        // Package descriptions - cyan
                        "summary" | "long-description" | "nixpkgs-position" => {
                            format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, value)
                        }
                        // License and legal - bright green
                        "license" | "distribution" => {
                            format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value)
                        }
                        // Size information - yellow
                        | "size"
                        | "filename"
                        | "modified-time"
                        | "unpacked-size"
                        | "file-count"
                        | "total-size"
                        | "wheel-files"
                        | "source-files" => {
                            format!("\x1b[93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value)
                        }
                        // URLs - underlined blue
                        | "aur-url"
                        | "upstream-url"
                        | "url"
                        | "homepage"
                        | "ubuntu-url"
                        | "nixos-url"
                        | "opensuse-url"
                        | "npm-url"
                        | "registry-url"
                        | "pypi-url"
                        | "crates-io-url"
                        | "docs-rs-url"
                        | "api-url"
                        | "github-url"
                        | "clone-url"
                        | "ssh-url"
                        | "avatar-url" => {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored_value = url_regex
                                .replace_all(value, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[1;94m{}:\x1b[0m {}", attr, colored_value)
                        }
                        // Default - rainbow gradient effect for unknown attributes
                        _ => {
                            let hash = attr
                                .chars()
                                .map(|c| c as u32)
                                .sum::<u32>();
                            let color_code = 31 + (hash % 6); // Rotate through 31-36 (red to cyan)
                            format!(
                                "\x1b[1;{}m{}:\x1b[0m \x1b[{}m{}\x1b[0m",
                                color_code,
                                attr,
                                color_code,
                                value
                            )
                        }
                    }
                } else {
                    line.to_string()
                }
            } else {
                // Handle special query type responses
                match query_type {
                    QueryType::Geo(_) | QueryType::RirGeo(_) => {
                        // Geo queries - highlight coordinates and locations
                        if
                            line.contains("latitude") ||
                            line.contains("longitude") ||
                            line.contains("coordinates")
                        {
                            format!("\x1b[95m{}\x1b[0m", line) // Bright magenta for coordinates
                        } else if
                            line.contains("location") ||
                            line.contains("city") ||
                            line.contains("region")
                        {
                            format!("\x1b[94m{}\x1b[0m", line) // Bright blue for locations
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::BGPTool(_) | QueryType::Prefixes(_) => {
                        // BGP/prefix queries - highlight network paths and ASNs
                        let asn_regex = Regex::new(r"(AS\d+)").unwrap();
                        let ip_regex = Regex::new(
                            r"(\d+\.\d+\.\d+\.\d+(?:/\d+)?|[0-9a-fA-F:]+::[0-9a-fA-F:]*(?:/\d+)?)"
                        ).unwrap();
                        let mut result = asn_regex
                            .replace_all(line, "\x1b[93m$1\x1b[0m")
                            .to_string();
                        result = ip_regex.replace_all(&result, "\x1b[92m$1\x1b[0m").to_string();
                        result
                    }
                    QueryType::Dns(_) => {
                        // DNS queries - comprehensive DNS record coloring
                        if line.contains("DNS Resolution Results") || line.contains("Query:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains(" A ") && !line.contains("AAAA") {
                            let ip_regex = Regex::new(r"(\d+\.\d+\.\d+\.\d+)").unwrap();
                            ip_regex.replace_all(line, "\x1b[92m$1\x1b[0m").to_string()
                        } else if line.contains(" AAAA ") {
                            let ipv6_regex = Regex::new(r"([0-9a-fA-F:]+::[0-9a-fA-F:]*)").unwrap();
                            ipv6_regex.replace_all(line, "\x1b[92m$1\x1b[0m").to_string()
                        } else if line.contains(" CNAME ") || line.contains(" DNAME ") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for aliases
                        } else if line.contains(" MX ") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for mail exchangers
                        } else if line.contains(" NS ") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for nameservers
                        } else if line.contains(" TXT ") || line.contains(" SPF ") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for text records
                        } else if line.contains("TTL:") || line.contains("ttl=") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for TTL
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Ssl(_) => {
                        // SSL queries - comprehensive certificate information coloring
                        if
                            line.contains("Certificate Information") ||
                            line.contains("SSL Certificate")
                        {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("Subject:") || line.contains("Issuer:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for cert subjects
                        } else if line.contains("Serial Number:") || line.contains("Version:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for identifiers
                        } else if line.contains("Not Before:") || line.contains("Not After:") {
                            if line.contains("Not After:") && line.contains("202") {
                                // Check if expires soon
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for expiry dates
                            } else {
                                format!("\x1b[90m{}\x1b[0m", line) // Gray for timestamps
                            }
                        } else if line.contains("Validity Period:") || line.contains("Algorithms:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for section headers
                        } else if line.contains("SHA") || line.contains("Fingerprint") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for fingerprints
                        } else if line.contains("Subject Alternative Names:") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for SAN section
                        } else if
                            line.contains("Key Usage:") ||
                            line.contains("Extended Key Usage:")
                        {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for usage info
                        } else if line.contains("Certificate Status:") {
                            if line.contains("Valid") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for valid
                            } else if line.contains("Expired") || line.contains("Invalid") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for invalid/expired
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other status
                            }
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Irr(_) => {
                        // IRR Explorer - routing registry analysis coloring
                        if line.contains("Route:") || line.contains("Prefix:") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for routes
                        } else if line.contains("Origin ASN:") || line.contains("AS-Path:") {
                            let asn_regex = Regex::new(r"(AS\d+)").unwrap();
                            asn_regex.replace_all(line, "\x1b[93m$1\x1b[0m").to_string()
                        } else if line.contains("RPKI Status:") {
                            if line.contains("Valid") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for valid RPKI
                            } else if line.contains("Invalid") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for invalid RPKI
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for unknown RPKI
                            }
                        } else if line.contains("IRR Sources:") || line.contains("Registered in:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for registry info
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::LookingGlass(_) => {
                        // Looking Glass - BGP routing data coloring
                        if line.contains("BGP Routing Table") || line.contains("Route Information") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("*>") || line.contains("best") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for best path
                        } else if line.contains("AS") && line.contains("Path") {
                            let asn_regex = Regex::new(r"(AS\d+|{\d+}|\d+)").unwrap();
                            asn_regex.replace_all(line, "\x1b[93m$1\x1b[0m").to_string()
                        } else if line.contains("Next Hop:") || line.contains("Nexthop:") {
                            let ip_regex = Regex::new(r"(\d+\.\d+\.\d+\.\d+)").unwrap();
                            ip_regex.replace_all(line, "\x1b[94m$1\x1b[0m").to_string()
                        } else if line.contains("MED:") || line.contains("Local Pref:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for BGP attributes
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Rpki(_, _) => {
                        // RPKI validation - security-focused coloring
                        if line.contains("RPKI Validation Result") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("Valid") && !line.contains("Invalid") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for valid
                        } else if line.contains("Invalid") {
                            format!("\x1b[91m{}\x1b[0m", line) // Red for invalid
                        } else if line.contains("Not Found") || line.contains("Unknown") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for unknown
                        } else if line.contains("ROA:") || line.contains("Certificate:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for certificate info
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Steam(_) => {
                        // Steam - game and user information coloring
                        if
                            line.contains("Steam Application Information") ||
                            line.contains("Steam User Profile Information")
                        {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if
                            line.contains("Status: Online") ||
                            line.contains("status: Online")
                        {
                            format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for online
                        } else if
                            line.contains("Status: Offline") ||
                            line.contains("status: Offline")
                        {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for offline
                        } else if line.contains("app-id:") || line.contains("steamid:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("name:") || line.contains("personaname:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bright cyan for names
                        } else if line.contains("price:") || line.contains("original-price:") {
                            if line.contains("(%↓)") || line.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let colored = price_regex
                                    .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                discount_regex
                                    .replace_all(&colored, "\x1b[1;92m$1\x1b[0m")
                                    .to_string()
                            } else {
                                // Red for full-price games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                price_regex.replace_all(line, "\x1b[1;91m$1\x1b[0m").to_string()
                            }
                        } else if line.contains("metacritic-score:") {
                            let score_regex = Regex::new(r"(\d+)").unwrap();
                            score_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("developers:") || line.contains("publishers:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for developers/publishers
                        } else if line.contains("genres:") || line.contains("categories:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for game categories
                        } else if line.contains("platforms:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for platforms
                        } else if line.contains("release-date:") || line.contains("created:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for dates
                        } else if
                            line.contains("steam-url:") ||
                            line.contains("profileurl:") ||
                            line.contains("website:") ||
                            line.contains("metacritic-url:")
                        {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else if line.contains("visibility:") || line.contains("profile-state:") {
                            if line.contains("Public") || line.contains("Configured") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for positive states
                            } else if line.contains("Private") || line.contains("Not Configured") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for restricted states
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other states
                            }
                        } else if line.contains("country:") || line.contains("state:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for location
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::SteamSearch(_) => {
                        // Steam search results - RIPE style coloring
                        if line.contains("Steam Game Search Results") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("Found") && line.contains("games:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for count
                        } else if line.contains(". Game Information") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for entry headers
                        } else if line.contains("app-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("name:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bright cyan for names
                        } else if line.contains("type:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for type
                        } else if line.contains("price:") {
                            if line.contains("(%↓)") || line.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let colored = price_regex
                                    .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                discount_regex
                                    .replace_all(&colored, "\x1b[1;92m$1\x1b[0m")
                                    .to_string()
                            } else {
                                // Red for full-price games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                price_regex.replace_all(line, "\x1b[1;91m$1\x1b[0m").to_string()
                            }
                        } else if line.contains("platforms:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for platforms
                        } else if line.contains("steam-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Imdb(_) => {
                        // IMDb - movie and TV show information coloring
                        if line.contains("IMDb Information for:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("imdb-id:") {
                            let id_regex = Regex::new(r"(tt\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for titles
                        } else if line.contains("year:") || line.contains("released:") {
                            let year_regex = Regex::new(r"(\d{4})").unwrap();
                            year_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("type:") {
                            if line.contains("movie") {
                                format!("\x1b[94m{}\x1b[0m", line) // Blue for movies
                            } else if line.contains("series") {
                                format!("\x1b[96m{}\x1b[0m", line) // Cyan for TV series
                            } else {
                                format!("\x1b[95m{}\x1b[0m", line) // Magenta for other types
                            }
                        } else if line.contains("imdb-rating:") {
                            let rating_regex = Regex::new(r"(\d+\.\d+/10)").unwrap();
                            if line.contains("8.") || line.contains("9.") {
                                rating_regex.replace_all(line, "\x1b[1;92m$1\x1b[0m").to_string() // Green for high ratings
                            } else if line.contains("7.") {
                                rating_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string() // Yellow for good ratings
                            } else {
                                rating_regex.replace_all(line, "\x1b[1;91m$1\x1b[0m").to_string() // Red for low ratings
                            }
                        } else if line.contains("metascore:") {
                            let score_regex = Regex::new(r"(\d+/100)").unwrap();
                            score_regex.replace_all(line, "\x1b[1;95m$1\x1b[0m").to_string()
                        } else if line.contains("box-office:") {
                            let money_regex = Regex::new(r"(\$[\d,]+)").unwrap();
                            money_regex.replace_all(line, "\x1b[1;92m$1\x1b[0m").to_string()
                        } else if line.contains("director:") || line.contains("writer:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for creative roles
                        } else if line.contains("actors:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for actors
                        } else if line.contains("genre:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for genres
                        } else if line.contains("awards:") && !line.contains("N/A") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for awards
                        } else if line.contains("rated:") {
                            if line.contains("PG") || line.contains("G") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for family-friendly
                            } else if line.contains("R") || line.contains("NC-17") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for mature content
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other ratings
                            }
                        } else if line.contains("imdb-url:") || line.contains("website:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else if line.contains("country:") || line.contains("language:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for location/language
                        } else if line.contains("runtime:") {
                            let time_regex = Regex::new(r"(\d+\s*min)").unwrap();
                            time_regex.replace_all(line, "\x1b[1;96m$1\x1b[0m").to_string()
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::ImdbSearch(_) => {
                        // IMDb search results - RIPE style coloring
                        if line.contains("IMDb Search Results for:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("Found") && line.contains("titles:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for count
                        } else if line.contains(". Title Information") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for entry headers
                        } else if line.contains("imdb-id:") {
                            let id_regex = Regex::new(r"(tt\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for titles
                        } else if line.contains("year:") {
                            let year_regex = Regex::new(r"(\d{4})").unwrap();
                            year_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("type:") {
                            if line.contains("movie") {
                                format!("\x1b[94m{}\x1b[0m", line) // Blue for movies
                            } else if line.contains("series") {
                                format!("\x1b[96m{}\x1b[0m", line) // Cyan for TV series
                            } else {
                                format!("\x1b[95m{}\x1b[0m", line) // Magenta for other types
                            }
                        } else if line.contains("imdb-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Wikipedia(_) => {
                        // Wikipedia - article information coloring
                        if line.contains("Wikipedia Article Information:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("page-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for article titles
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (Wikipedia)
                        } else if line.contains("article-length:") {
                            let size_regex = Regex::new(r"(\d+)\s*bytes").unwrap();
                            size_regex.replace_all(line, "\x1b[1;93m$1 bytes\x1b[0m").to_string()
                        } else if line.contains("last-modified:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for dates
                        } else if line.contains("categories:") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for categories
                        } else if line.contains("languages:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for language links
                        } else if line.contains("summary:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for article summary
                        } else if line.contains("wikipedia-url:") || line.contains("edit-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Lyric(_) => {
                        // Lyric - Luotianyi random lyrics coloring
                        if line.contains("Luotianyi Random Lyric:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("song-name:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for song names
                        } else if line.contains("singer:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for singer (Luotianyi)
                        } else if line.contains("author:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for authors
                        } else if line.contains("year:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for year
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (lty.vc)
                        } else if line.contains("lyric-content:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for lyric content header
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            // Lyric content lines - make them colorful
                            if
                                !line.trim().is_empty() &&
                                !line.contains(":") &&
                                !line.starts_with("=") &&
                                !line.starts_with("%")
                            {
                                format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for actual lyrics
                            } else {
                                line.to_string()
                            }
                        }
                    }
                    QueryType::Desc(_) => {
                        // Description query - highlight descriptions and headers
                        if line.contains("Description Query Results for:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("descriptions found") || line.contains("description found") || line.contains("remarks found") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for count info
                        } else if line.contains("descr:") || line.contains("descr[") || line.contains("remarks:") || line.contains("description:") {
                            // Extract and highlight the description value
                            if let Some(colon_pos) = line.find(':') {
                                let attr = &line[..=colon_pos];
                                let value = &line[colon_pos + 1..];
                                format!("\x1b[94m{}\x1b[92m{}\x1b[0m", attr, value) // Blue for attr, green for description
                            } else {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for description content
                            }
                        } else if line.contains("Total descriptions:") || line.contains("Total fields:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for summary
                        } else if line.contains("No description fields found") || line.contains("No description or remarks fields found") {
                            format!("\x1b[91m{}\x1b[0m", line) // Red for no results
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Meal => {
                        // Meal suggestions - food-themed colorization
                        if line.contains("Meal Information") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("meal-name:") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for meal names
                        } else if line.contains("category:") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for categories
                        } else if line.contains("cuisine:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for cuisine type
                        } else if line.contains("ingredient:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for ingredients
                        } else if line.contains("instruction-") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for cooking instructions
                        } else if line.contains("youtube-video:") || line.contains("meal-image:") {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for media links
                        } else if line.contains("tags:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for tags
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Acgc(_) => {
                        // ACGC - Anime/Comic/Game character information coloring
                        if line.contains("ACGC Character Information:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("page-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            id_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("character-name:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for character names
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (Moegirl Wiki)
                        } else if line.contains("description:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for character description
                        } else if
                            line.contains("voice-actor:") ||
                            line.contains("cv:") ||
                            line.contains("voice-actor-jp:") ||
                            line.contains("voice-actor-cn:")
                        {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for voice actors
                        } else if
                            line.contains("source-work:") ||
                            line.contains("series:") ||
                            line.contains("character-template:")
                        {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for series/work origin
                        } else if
                            line.contains("personality:") ||
                            line.contains("moe-points:") ||
                            line.contains("attributes:") ||
                            line.contains("traits:")
                        {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for personality traits
                        } else if
                            line.contains("species:") ||
                            line.contains("identity:") ||
                            line.contains("class:") ||
                            line.contains("level:")
                        {
                            format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for species/identity
                        } else if
                            line.contains("ability:") ||
                            line.contains("skill:") ||
                            line.contains("special-skill:") ||
                            line.contains("weapon:") ||
                            line.contains("equipment:")
                        {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for abilities/weapons
                        } else if
                            line.contains("title:") ||
                            line.contains("alias:") ||
                            line.contains("nickname:")
                        {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for titles/aliases
                        } else if
                            line.contains("family:") ||
                            line.contains("friends:") ||
                            line.contains("lover:") ||
                            line.contains("master:") ||
                            line.contains("subordinate:")
                        {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for relationships
                        } else if line.contains("categories:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for categories
                        } else if line.contains("clothing:") || line.contains("appearance:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for appearance/clothing
                        } else if line.contains("age:") || line.contains("birthday:") {
                            let number_regex = Regex::new(r"(\d+)").unwrap();
                            number_regex.replace_all(line, "\x1b[1;93m$1\x1b[0m").to_string()
                        } else if line.contains("height:") || line.contains("weight:") {
                            let measurement_regex = Regex::new(
                                r"(\d+[\.\d]*\s*[cm|kg|m])"
                            ).unwrap();
                            measurement_regex.replace_all(line, "\x1b[1;92m$1\x1b[0m").to_string()
                        } else if line.contains("hair-color:") || line.contains("eye-color:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for physical appearance
                        } else if line.contains("gender:") {
                            if line.contains("女") || line.contains("Female") {
                                format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for female
                            } else if line.contains("男") || line.contains("Male") {
                                format!("\x1b[1;94m{}\x1b[0m", line) // Bright blue for male
                            } else {
                                format!("\x1b[1;96m{}\x1b[0m", line) // Bright cyan for other
                            }
                        } else if
                            line.contains("occupation:") ||
                            line.contains("职业:") ||
                            line.contains("position:")
                        {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for occupation
                        } else if
                            line.contains("origin:") ||
                            line.contains("出身:") ||
                            line.contains("hobby:")
                        {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for origin/hobby
                        } else if line.contains("moegirl-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            url_regex.replace_all(line, "\x1b[4;94m$1\x1b[0m").to_string()
                        } else {
                            line.to_string()
                        }
                    }
                    _ => line.to_string(),
                }
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

    fn colorize_bgptools_style(&self, response: &str, query_type: &QueryType) -> String {
        let mut colorized = String::new();

        for line in response.lines() {
            let colored_line = if line.starts_with('%') {
                // Comments in bright black (gray) - matches reference implementation
                format!("\x1b[90m{}\x1b[0m", line)
            } else if line.contains(':') && !line.starts_with(' ') {
                // Attribute-value pairs with BGP Tools styling
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let attr = parts[0].trim();
                    let value = parts[1];

                    // Apply regex patterns to value for network elements
                    let asn_regex = Regex::new(r"(AS\d+)").unwrap();
                    let ip_regex = Regex::new(
                        r"(\d+\.\d+\.\d+\.\d+(?:/\d+)?|[0-9a-fA-F:]+::[0-9a-fA-F:]*(?:/\d+)?)"
                    ).unwrap();
                    let domain_regex = Regex::new(
                        r"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}"
                    ).unwrap();

                    let mut styled_value = value.to_string();
                    styled_value = asn_regex
                        .replace_all(&styled_value, "\x1b[93m$1\x1b[0m")
                        .to_string();
                    styled_value = ip_regex
                        .replace_all(&styled_value, "\x1b[92m$1\x1b[0m")
                        .to_string();
                    styled_value = domain_regex
                        .replace_all(&styled_value, "\x1b[94m$1\x1b[0m")
                        .to_string();

                    match attr {
                        // AS related - bright red (AS column in reference)
                        "origin" | "aut-num" | "as-name" | "asn" => {
                            format!("\x1b[91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, styled_value)
                        }
                        // Network/IP info - bright cyan (IP/Prefix column in reference)
                        "route" | "route6" | "inetnum" | "inet6num" | "prefix" | "network" => {
                            format!("\x1b[96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value)
                        }
                        // Status/validation - conditional colors
                        "status" | "rpki-status" | "validation" => {
                            if
                                value.trim().to_lowercase().contains("valid") &&
                                !value.trim().to_lowercase().contains("invalid")
                            {
                                format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, value) // Bright green for valid
                            } else if value.trim().to_lowercase().contains("invalid") {
                                format!("\x1b[1;91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, value) // Bright red for invalid
                            } else {
                                format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, value) // Bright yellow for unknown
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
                            format!("\x1b[95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, styled_value)
                        }
                        // AS Names and org names - bright white bold (AS Name column in reference)
                        "netname" | "orgname" | "org-name" => {
                            format!("\x1b[1;97m{}:\x1b[0m \x1b[1;97m{}\x1b[0m", attr, styled_value)
                        }
                        // Geographic/location info - default magenta
                        "city" | "region" | "geoloc" | "address" => {
                            format!("\x1b[35m{}:\x1b[0m \x1b[35m{}\x1b[0m", attr, styled_value)
                        }
                        // Contact info - default blue
                        "person" | "admin-c" | "tech-c" | "mnt-by" | "contact" => {
                            format!("\x1b[34m{}:\x1b[0m \x1b[34m{}\x1b[0m", attr, styled_value)
                        }
                        // Package info - bright cyan
                        | "package"
                        | "version"
                        | "depends"
                        | "makedepends"
                        | "package-name"
                        | "attribute-name"
                        | "attribute-set"
                        | "component"
                        | "source-package"
                        | "source-version"
                        | "section"
                        | "priority"
                        | "project"
                        | "repository"
                        | "release"
                        | "architecture"
                        | "platforms"
                        | "outputs"
                        | "maintainers"
                        | "author"
                        | "replaces"
                        | "breaks"
                        | "provides"
                        | "suggests"
                        | "upstream"
                        | "upstream-version"
                        | "architectures" => {
                            format!("\x1b[1;96m{}:\x1b[0m \x1b[96m{}\x1b[0m", attr, styled_value)
                        }
                        // Package descriptions - bright magenta
                        "summary" | "long-description" | "nixpkgs-position" => {
                            format!("\x1b[1;95m{}:\x1b[0m \x1b[95m{}\x1b[0m", attr, styled_value)
                        }
                        // License and legal - bright green
                        "license" | "distribution" => {
                            format!("\x1b[1;92m{}:\x1b[0m \x1b[92m{}\x1b[0m", attr, styled_value)
                        }
                        // Size and metadata - bright yellow
                        "size" | "filename" | "modified-time" => {
                            format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value)
                        }
                        // URLs - underlined blue
                        | "aur-url"
                        | "upstream-url"
                        | "url"
                        | "homepage"
                        | "ubuntu-url"
                        | "nixos-url"
                        | "opensuse-url"
                        | "aosc-url" => {
                            format!("\x1b[1;94m{}:\x1b[0m \x1b[4;94m{}\x1b[0m", attr, styled_value)
                        }
                        // Dates - gray (non-allocation dates)
                        "last-modified" | "expires" | "updated" | "first-submitted" => {
                            format!("\x1b[90m{}:\x1b[0m \x1b[90m{}\x1b[0m", attr, styled_value)
                        }
                        // Price information - conditional colors for Steam
                        "price" | "original-price" => {
                            if value.contains("(%↓)") || value.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let colored_value = price_regex
                                    .replace_all(value, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                let final_value = discount_regex
                                    .replace_all(&colored_value, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}:\x1b[0m{}", attr, final_value)
                            } else {
                                // White for full-price games (no discount)
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                let colored_value = price_regex
                                    .replace_all(value, "\x1b[97m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}:\x1b[0m{}", attr, colored_value)
                            }
                        }
                        // Security/crypto - bright red
                        "fingerprint" | "signature" | "certificate" | "ssl" => {
                            format!("\x1b[1;91m{}:\x1b[0m \x1b[91m{}\x1b[0m", attr, styled_value)
                        }
                        // Popularity/stats - bright yellow
                        "votes" | "popularity" | "players" | "rating" => {
                            format!("\x1b[1;93m{}:\x1b[0m \x1b[93m{}\x1b[0m", attr, styled_value)
                        }
                        // Default - gradient rainbow
                        _ => {
                            let hash = attr
                                .chars()
                                .map(|c| c as u32)
                                .sum::<u32>();
                            let color_code = 91 + (hash % 6); // Bright colors 91-96
                            format!(
                                "\x1b[1;{}m{}:\x1b[0m \x1b[{}m{}\x1b[0m",
                                color_code,
                                attr,
                                color_code,
                                styled_value
                            )
                        }
                    }
                } else {
                    line.to_string()
                }
            } else {
                // Handle query-type specific content
                match query_type {
                    QueryType::EmailSearch(_) => {
                        // Email search - highlight email addresses
                        let email_regex = Regex::new(
                            r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})"
                        ).unwrap();
                        email_regex.replace_all(line, "\x1b[96m$1\x1b[0m").to_string()
                    }
                    QueryType::Trace(_) => {
                        // Traceroute - highlight hops and latency
                        if line.contains("ms") || line.contains("hop") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for timing info
                        } else {
                            let ip_regex = Regex::new(r"(\d+\.\d+\.\d+\.\d+)").unwrap();
                            ip_regex.replace_all(line, "\x1b[92m$1\x1b[0m").to_string()
                        }
                    }
                    QueryType::Crt(_) => {
                        // Certificate Transparency - comprehensive certificate coloring
                        if line.contains("Serial Number:") || line.contains("ID:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for cert identifiers
                        } else if line.contains("Subject:") || line.contains("Issuer:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for cert subjects/issuers
                        } else if
                            line.contains("Not Before:") ||
                            line.contains("Not After:") ||
                            line.contains("Logged at:")
                        {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for timestamps
                        } else if line.contains("Fingerprint") || line.contains("SHA") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for fingerprints
                        } else if line.contains("Common Name:") || line.contains("CN=") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for common names
                        } else if line.contains("Certificate:") || line.contains("Entry") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for entry headers
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Minecraft(_) => {
                        // Minecraft server - clean color coding
                        if line.contains("Status: Online") || line.contains("server is online") {
                            format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for online
                        } else if
                            line.contains("Status: Offline") ||
                            line.contains("offline") ||
                            line.contains("unreachable") ||
                            line.contains("timeout")
                        {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for offline
                        } else if line.contains("Players:") || line.contains("players online") {
                            let player_regex = Regex::new(r"(\d+/\d+|\d+ players?)").unwrap();
                            let colored = player_regex
                                .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("Version:") {
                            let version_regex = Regex::new(r"(\d+\.\d+[\.\d]*)").unwrap();
                            let colored = version_regex
                                .replace_all(line, "\x1b[1;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("MOTD:") || line.contains("Description:") {
                            format!("\x1b[96m{}\x1b[0m", line)
                        } else if line.contains("Latency:") || line.contains("ms") {
                            let latency_regex = Regex::new(r"(\d+)\s*ms").unwrap();
                            let colored = latency_regex
                                .replace_all(line, "\x1b[1;93m$1ms\x1b[0m")
                                .to_string();
                            format!("\x1b[93m{}\x1b[0m", colored)
                        } else if line.contains("Max Players:") || line.contains("Slots:") {
                            let slot_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = slot_regex
                                .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::MinecraftUser(_) => {
                        // Minecraft user - player information coloring (BGPTools style)
                        if line.contains("Minecraft User Information:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("username:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for username
                        } else if line.contains("uuid:") || line.contains("uuid-short:") {
                            let uuid_regex = Regex::new(
                                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}|[0-9a-fA-F]{32})"
                            ).unwrap();
                            let colored = uuid_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("has-skin:") || line.contains("skin-signed:") {
                            if line.contains("yes") {
                                format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for yes
                            } else {
                                format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for no
                            }
                        } else if
                            line.contains("namemc-url:") ||
                            line.contains("skin-url:") ||
                            line.contains("avatar-url:")
                        {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("profile-status:") {
                            if line.contains("failed") || line.contains("error") {
                                format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for errors
                            } else {
                                format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for status
                            }
                        } else if line.starts_with("property-") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for properties
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Steam(_) => {
                        // Steam - comprehensive game and user information coloring
                        if
                            line.contains("Steam Application Information") ||
                            line.contains("Steam User Profile Information")
                        {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if
                            line.contains("Status: Online") ||
                            line.contains("status: Online")
                        {
                            format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for online status
                        } else if
                            line.contains("Status: Offline") ||
                            line.contains("status: Offline")
                        {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for offline status
                        } else if line.contains("app-id:") || line.contains("steamid:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("name:") || line.contains("personaname:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for names
                        } else if line.contains("price:") || line.contains("original-price:") {
                            if line.contains("(%↓)") || line.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let mut colored = price_regex
                                    .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                colored = discount_regex
                                    .replace_all(&colored, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}\x1b[0m", colored)
                            } else {
                                // Red for full-price games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                let colored = price_regex
                                    .replace_all(line, "\x1b[1;91m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}\x1b[0m", colored)
                            }
                        } else if line.contains("discount:") {
                            let discount_regex = Regex::new(r"(\d+%)").unwrap();
                            let colored = discount_regex
                                .replace_all(line, "\x1b[1;91m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[93m{}\x1b[0m", colored)
                        } else if line.contains("metacritic-score:") {
                            let score_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = score_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if
                            line.contains("recommendations:") ||
                            line.contains("achievements:")
                        {
                            let num_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = num_regex
                                .replace_all(line, "\x1b[1;96m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[96m{}\x1b[0m", colored)
                        } else if line.contains("developers:") || line.contains("publishers:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for developers/publishers
                        } else if line.contains("genres:") || line.contains("categories:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for classifications
                        } else if line.contains("platforms:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for platforms
                        } else if line.contains("release-date:") || line.contains("created:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for dates
                        } else if
                            line.contains("steam-url:") ||
                            line.contains("profileurl:") ||
                            line.contains("website:") ||
                            line.contains("metacritic-url:")
                        {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("visibility:") || line.contains("profile-state:") {
                            if line.contains("Public") || line.contains("Configured") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for public/configured
                            } else if line.contains("Private") || line.contains("Not Configured") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for private/not configured
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other states
                            }
                        } else if line.contains("country:") || line.contains("state:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for location
                        } else if
                            line.contains("avatar:") ||
                            line.contains("avatar-medium:") ||
                            line.contains("avatar-full:")
                        {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;96m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[96m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::SteamSearch(_) => {
                        // Steam search results - clean, structured coloring
                        if line.contains("Steam Game Search Results") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("Found") && line.contains("games:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bold magenta for count
                        } else if line.contains(". Game Information") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bold yellow for game entry headers
                        } else if line.contains("app-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("name:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for game names
                        } else if line.contains("type:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for app type
                        } else if line.contains("price:") {
                            if line.contains("(%↓)") || line.contains("Free") {
                                // Green for discounted games and free games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*|Free)").unwrap();
                                let discount_regex = Regex::new(r"(\d+%↓)").unwrap();
                                let mut colored = price_regex
                                    .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                colored = discount_regex
                                    .replace_all(&colored, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}\x1b[0m", colored)
                            } else {
                                // Red for full-price games
                                let price_regex = Regex::new(r"(\$[\d,]+\.?\d*)").unwrap();
                                let colored = price_regex
                                    .replace_all(line, "\x1b[1;91m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[95m{}\x1b[0m", colored)
                            }
                        } else if line.contains("platforms:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for platforms
                        } else if line.contains("status: Coming Soon") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for coming soon
                        } else if line.contains("steam-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else if line.contains("---") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for separators
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Aur(_) => {
                        // AUR packages - clean color coding
                        if line.contains("out-of-date:") && !line.contains("no") {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for out-of-date
                        } else if line.contains("maintainer:") && line.contains("orphaned") {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for orphaned
                        } else if line.contains("votes:") {
                            let vote_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = vote_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("popularity:") {
                            let pop_regex = Regex::new(r"(\d+\.\d+)").unwrap();
                            let colored = pop_regex
                                .replace_all(line, "\x1b[1;95m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("depends:") {
                            format!("\x1b[94m{}\x1b[0m", line)
                        } else if line.contains("makedepends:") {
                            format!("\x1b[96m{}\x1b[0m", line)
                        } else if line.contains("optdepends:") {
                            format!("\x1b[95m{}\x1b[0m", line)
                        } else if line.contains("conflicts:") {
                            format!("\x1b[93m{}\x1b[0m", line)
                        } else if line.contains("aur-url:") || line.contains("upstream-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Debian(_) => {
                        // Debian packages - comprehensive coloring
                        if
                            line.contains("Priority: required") ||
                            line.contains("Priority: important")
                        {
                            format!("\x1b[91m{}\x1b[0m", line) // Red for critical priority
                        } else if line.contains("Status:") {
                            if line.contains("installed") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for installed
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other status
                            }
                        } else if
                            line.contains("Depends:") ||
                            line.contains("Pre-Depends:") ||
                            line.contains("Recommends:") ||
                            line.contains("Suggests:")
                        {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for dependencies
                        } else if line.contains("Conflicts:") || line.contains("Breaks:") {
                            format!("\x1b[91m{}\x1b[0m", line) // Red for conflicts
                        } else if line.contains("Size:") || line.contains("Installed-Size:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for size info
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Manrs(_) => {
                        // MANRS - highlight compliance status
                        if line.contains("compliant") || line.contains("implemented") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for compliant
                        } else if line.contains("non-compliant") || line.contains("missing") {
                            format!("\x1b[91m{}\x1b[0m", line) // Red for non-compliant
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Imdb(_) => {
                        // IMDb - movie and TV show information coloring (BGPTools style)
                        if line.contains("IMDb Information for:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("imdb-id:") {
                            let id_regex = Regex::new(r"(tt\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for titles
                        } else if line.contains("year:") || line.contains("released:") {
                            let year_regex = Regex::new(r"(\d{4})").unwrap();
                            let colored = year_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("imdb-rating:") {
                            let rating_regex = Regex::new(r"(\d+\.\d+/10)").unwrap();
                            if line.contains("8.") || line.contains("9.") {
                                let colored = rating_regex
                                    .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[92m{}\x1b[0m", colored) // Green for high ratings
                            } else if line.contains("7.") {
                                let colored = rating_regex
                                    .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[93m{}\x1b[0m", colored) // Yellow for good ratings
                            } else {
                                let colored = rating_regex
                                    .replace_all(line, "\x1b[1;91m$1\x1b[0m")
                                    .to_string();
                                format!("\x1b[91m{}\x1b[0m", colored) // Red for low ratings
                            }
                        } else if line.contains("box-office:") {
                            let money_regex = Regex::new(r"(\$[\d,]+)").unwrap();
                            let colored = money_regex
                                .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("director:") || line.contains("writer:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for creative roles
                        } else if line.contains("actors:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for actors
                        } else if line.contains("genre:") {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for genres
                        } else if line.contains("awards:") && !line.contains("N/A") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for awards
                        } else if line.contains("rated:") {
                            if line.contains("PG") || line.contains("G") {
                                format!("\x1b[92m{}\x1b[0m", line) // Green for family-friendly
                            } else if line.contains("R") || line.contains("NC-17") {
                                format!("\x1b[91m{}\x1b[0m", line) // Red for mature content
                            } else {
                                format!("\x1b[93m{}\x1b[0m", line) // Yellow for other ratings
                            }
                        } else if line.contains("imdb-url:") || line.contains("website:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("runtime:") {
                            let time_regex = Regex::new(r"(\d+\s*min)").unwrap();
                            let colored = time_regex
                                .replace_all(line, "\x1b[1;96m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[96m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::ImdbSearch(_) => {
                        // IMDb search results - BGPTools style coloring
                        if line.contains("IMDb Search Results for:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for header
                        } else if line.contains("Found") && line.contains("titles:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bold magenta for count
                        } else if line.contains(". Title Information") {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bold yellow for entry headers
                        } else if line.contains("imdb-id:") {
                            let id_regex = Regex::new(r"(tt\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for titles
                        } else if line.contains("year:") {
                            let year_regex = Regex::new(r"(\d{4})").unwrap();
                            let colored = year_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("type:") {
                            if line.contains("movie") {
                                format!("\x1b[94m{}\x1b[0m", line) // Blue for movies
                            } else if line.contains("series") {
                                format!("\x1b[96m{}\x1b[0m", line) // Cyan for TV series
                            } else {
                                format!("\x1b[95m{}\x1b[0m", line) // Magenta for other types
                            }
                        } else if line.contains("imdb-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else if line.contains("---") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for separators
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Wikipedia(_) => {
                        // Wikipedia - article information coloring (BGPTools style)
                        if line.contains("Wikipedia Article Information:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("page-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("title:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for article titles
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (Wikipedia)
                        } else if line.contains("article-length:") {
                            let size_regex = Regex::new(r"(\d+)\s*bytes").unwrap();
                            let colored = size_regex
                                .replace_all(line, "\x1b[1;93m$1 bytes\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("last-modified:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for timestamps
                        } else if line.contains("categories:") {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for categories
                        } else if line.contains("languages:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for language links
                        } else if line.contains("summary:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for article summary
                        } else if line.contains("wikipedia-url:") || line.contains("edit-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Lyric(_) => {
                        // Lyric - Luotianyi random lyrics coloring (BGPTools style)
                        if line.contains("Luotianyi Random Lyric:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("song-name:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for song names
                        } else if line.contains("singer:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for singer (Luotianyi)
                        } else if line.contains("author:") {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for authors
                        } else if line.contains("year:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for year
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (lty.vc)
                        } else if line.contains("lyric-content:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for lyric content header
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            // Lyric content lines - make them bright and colorful in BGPTools style
                            if
                                !line.trim().is_empty() &&
                                !line.contains(":") &&
                                !line.starts_with("=") &&
                                !line.starts_with("%")
                            {
                                format!("\x1b[1;97m{}\x1b[0m", line) // Bright white for actual lyrics
                            } else {
                                line.to_string()
                            }
                        }
                    }
                    QueryType::Desc(_) => {
                        // Description query - BGPTools style with highlighted backgrounds
                        if line.contains("Description Query Results for:") {
                            format!("\x1b[1;46m\x1b[30m{}\x1b[0m", line) // Black text on cyan background for header
                        } else if line.contains("descriptions found") || line.contains("description found") || line.contains("remarks found") {
                            format!("\x1b[1;45m\x1b[37m{}\x1b[0m", line) // White text on magenta background for count info
                        } else if line.contains("descr:") || line.contains("descr[") || line.contains("remarks:") || line.contains("description:") {
                            // Extract and highlight the description value with background
                            if let Some(colon_pos) = line.find(':') {
                                let attr = &line[..=colon_pos];
                                let value = &line[colon_pos + 1..];
                                format!("\x1b[1;44m\x1b[37m{}\x1b[0m\x1b[1;42m\x1b[30m{}\x1b[0m", attr, value) // White on blue for attr, black on green for description
                            } else {
                                format!("\x1b[1;42m\x1b[30m{}\x1b[0m", line) // Black text on green background for description content
                            }
                        } else if line.contains("Total descriptions:") || line.contains("Total fields:") {
                            format!("\x1b[1;43m\x1b[30m{}\x1b[0m", line) // Black text on yellow background for summary
                        } else if line.contains("No description fields found") || line.contains("No description or remarks fields found") {
                            format!("\x1b[1;41m\x1b[37m{}\x1b[0m", line) // White text on red background for no results
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Meal => {
                        // Meal suggestions - food-themed colorization (BGPTools style)
                        if line.contains("Meal Information") {
                            format!("\x1b[1;46m\x1b[30m{}\x1b[0m", line) // Black text on cyan background for header
                        } else if line.contains("meal-name:") {
                            format!("\x1b[1;43m\x1b[30m{}\x1b[0m", line) // Black text on bright yellow background for meal names
                        } else if line.contains("category:") {
                            format!("\x1b[1;42m\x1b[30m{}\x1b[0m", line) // Black text on green background for categories
                        } else if line.contains("cuisine:") {
                            format!("\x1b[1;45m\x1b[37m{}\x1b[0m", line) // White text on magenta background for cuisine type
                        } else if line.contains("ingredient:") {
                            format!("\x1b[1;94m{}\x1b[0m", line) // Bright blue for ingredients
                        } else if line.contains("instruction-") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bright cyan for cooking instructions
                        } else if line.contains("youtube-video:") || line.contains("meal-image:") {
                            format!("\x1b[1;41m\x1b[37m{}\x1b[0m", line) // White text on red background for media links
                        } else if line.contains("tags:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for tags
                        } else if line.starts_with("%") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for comments
                        } else {
                            line.to_string()
                        }
                    }
                    QueryType::Acgc(_) => {
                        // ACGC - Anime/Comic/Game character information coloring (BGPTools style)
                        if line.contains("ACGC Character Information:") {
                            format!("\x1b[1;96m{}\x1b[0m", line) // Bold cyan for headers
                        } else if line.contains("page-id:") {
                            let id_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = id_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else if line.contains("character-name:") {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for character names
                        } else if line.contains("source:") {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for source (Moegirl Wiki)
                        } else if line.contains("description:") {
                            format!("\x1b[1;37m{}\x1b[0m", line) // Bold white for character description
                        } else if
                            line.contains("voice-actor:") ||
                            line.contains("cv:") ||
                            line.contains("voice-actor-jp:") ||
                            line.contains("voice-actor-cn:")
                        {
                            format!("\x1b[94m{}\x1b[0m", line) // Blue for voice actors
                        } else if
                            line.contains("source-work:") ||
                            line.contains("series:") ||
                            line.contains("character-template:")
                        {
                            format!("\x1b[95m{}\x1b[0m", line) // Magenta for series/work origin
                        } else if
                            line.contains("personality:") ||
                            line.contains("moe-points:") ||
                            line.contains("attributes:") ||
                            line.contains("traits:")
                        {
                            format!("\x1b[96m{}\x1b[0m", line) // Cyan for personality traits
                        } else if
                            line.contains("species:") ||
                            line.contains("identity:") ||
                            line.contains("class:") ||
                            line.contains("level:")
                        {
                            format!("\x1b[1;92m{}\x1b[0m", line) // Bright green for species/identity
                        } else if
                            line.contains("ability:") ||
                            line.contains("skill:") ||
                            line.contains("special-skill:") ||
                            line.contains("weapon:") ||
                            line.contains("equipment:")
                        {
                            format!("\x1b[1;91m{}\x1b[0m", line) // Bright red for abilities/weapons
                        } else if
                            line.contains("title:") ||
                            line.contains("alias:") ||
                            line.contains("nickname:")
                        {
                            format!("\x1b[1;93m{}\x1b[0m", line) // Bright yellow for titles/aliases
                        } else if
                            line.contains("family:") ||
                            line.contains("friends:") ||
                            line.contains("lover:") ||
                            line.contains("master:") ||
                            line.contains("subordinate:")
                        {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for relationships
                        } else if line.contains("categories:") {
                            format!("\x1b[90m{}\x1b[0m", line) // Gray for categories
                        } else if line.contains("clothing:") || line.contains("appearance:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for appearance/clothing
                        } else if line.contains("age:") || line.contains("birthday:") {
                            let number_regex = Regex::new(r"(\d+)").unwrap();
                            let colored = number_regex
                                .replace_all(line, "\x1b[1;93m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("height:") || line.contains("weight:") {
                            let measurement_regex = Regex::new(
                                r"(\d+[\.\d]*\s*[cm|kg|m])"
                            ).unwrap();
                            let colored = measurement_regex
                                .replace_all(line, "\x1b[1;92m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[95m{}\x1b[0m", colored)
                        } else if line.contains("hair-color:") || line.contains("eye-color:") {
                            format!("\x1b[93m{}\x1b[0m", line) // Yellow for physical appearance
                        } else if line.contains("gender:") {
                            if line.contains("女") || line.contains("Female") {
                                format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for female
                            } else if line.contains("男") || line.contains("Male") {
                                format!("\x1b[1;94m{}\x1b[0m", line) // Bright blue for male
                            } else {
                                format!("\x1b[1;96m{}\x1b[0m", line) // Bright cyan for other
                            }
                        } else if
                            line.contains("occupation:") ||
                            line.contains("职业:") ||
                            line.contains("position:")
                        {
                            format!("\x1b[92m{}\x1b[0m", line) // Green for occupation
                        } else if
                            line.contains("origin:") ||
                            line.contains("出身:") ||
                            line.contains("hobby:")
                        {
                            format!("\x1b[1;95m{}\x1b[0m", line) // Bright magenta for origin/hobby
                        } else if line.contains("moegirl-url:") {
                            let url_regex = Regex::new(r"(https?://[^\s]+)").unwrap();
                            let colored = url_regex
                                .replace_all(line, "\x1b[4;94m$1\x1b[0m")
                                .to_string();
                            format!("\x1b[94m{}\x1b[0m", colored)
                        } else {
                            line.to_string()
                        }
                    }
                    _ => {
                        // Apply general network pattern highlighting
                        let asn_regex = Regex::new(r"(AS\d+)").unwrap();
                        let ip_regex = Regex::new(
                            r"(\d+\.\d+\.\d+\.\d+(?:/\d+)?|[0-9a-fA-F:]+::[0-9a-fA-F:]*(?:/\d+)?)"
                        ).unwrap();
                        let domain_regex = Regex::new(
                            r"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}"
                        ).unwrap();

                        let mut result = asn_regex
                            .replace_all(line, "\x1b[93m$1\x1b[0m")
                            .to_string();
                        result = ip_regex.replace_all(&result, "\x1b[92m$1\x1b[0m").to_string();
                        result = domain_regex.replace_all(&result, "\x1b[94m$1\x1b[0m").to_string();
                        result
                    }
                }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scheme_parsing() {
        assert_eq!(ColorScheme::from_string("ripe"), Some(ColorScheme::Ripe));
        assert_eq!(ColorScheme::from_string("RIPE"), Some(ColorScheme::Ripe));
        assert_eq!(ColorScheme::from_string("bgptools"), Some(ColorScheme::BgpTools));
        assert_eq!(ColorScheme::from_string("invalid"), None);
    }

    #[test]
    fn test_protocol_header_parsing() {
        let mut protocol = ColorProtocol::new();

        // Test capability probe
        let probe_request = "X-WHOIS-COLOR-PROBE: 1\r\nexample.com\r\n";
        assert!(protocol.parse_headers(probe_request));
        assert!(protocol.client_supports_color);

        // Test color scheme request
        let mut protocol2 = ColorProtocol::new();
        let color_request = "X-WHOIS-COLOR: ripe\r\nexample.com\r\n";
        assert!(!protocol2.parse_headers(color_request));
        assert!(protocol2.client_supports_color);
        assert_eq!(protocol2.scheme, Some(ColorScheme::Ripe));
    }
}
