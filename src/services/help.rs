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

pub fn generate_help_response() -> String {
    let mut output = String::new();

    output.push_str("WHOIS Server - Query Help\n");
    output.push_str(&"=".repeat(60));
    output.push('\n');
    output.push('\n');

    output.push_str("This WHOIS server supports multiple query types and services.\n");
    output.push_str("Simply type your query followed by the appropriate suffix.\n");
    output.push('\n');

    output.push_str("BASIC QUERIES:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');
    output.push_str("domain.com          - Domain WHOIS information\n");
    output.push_str("192.168.1.1         - IPv4 address information\n");
    output.push_str("2001:db8::1         - IPv6 address information\n");
    output.push_str("AS15169             - ASN (Autonomous System) information\n");
    output.push_str("192.168.0.0/24      - CIDR block information\n");
    output.push('\n');

    output.push_str("ENHANCED QUERIES:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');

    #[cfg(feature = "email")]
    {
        output.push_str("domain.com-EMAIL    - Search for email addresses in WHOIS data\n");
        output.push_str("example: google.com-EMAIL\n");
        output.push('\n');
    }

    #[cfg(feature = "bgp")]
    {
        output.push_str("AS15169-BGPTOOL     - BGP routing analysis and statistics\n");
        output.push_str("example: AS15169-BGPTOOL\n");
        output.push('\n');
    }

    #[cfg(feature = "bgp-prefixes")]
    {
        output.push_str("AS15169-PREFIXES    - List all prefixes announced by ASN\n");
        output.push_str("example: AS15169-PREFIXES\n");
        output.push('\n');
    }

    #[cfg(feature = "peeringdb")]
    {
        output
            .push_str("AS15169-PEERINGDB   - PeeringDB network information (ASN with AS prefix)\n");
        output.push_str(
            "4718-PEERINGDB      - PeeringDB Internet Exchange info (pure number = IX ID)\n",
        );
        output.push_str("example: AS15169-PEERINGDB, 4718-PEERINGDB\n");
        output.push('\n');
    }

    #[cfg(feature = "geo")]
    {
        output.push_str("GEO-LOCATION SERVICES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("8.8.8.8-GEO         - IP geolocation (commercial database)\n");
        output.push_str("example: 8.8.8.8-GEO\n");
        output.push('\n');
        output.push_str("8.8.8.8-RIRGEO      - RIR geolocation (registry data)\n");
        output.push_str("example: 8.8.8.8-RIRGEO\n");
        output.push('\n');
    }

    #[cfg(any(feature = "bgp", feature = "bgp-irr", feature = "bgp-manrs"))]
    {
        output.push_str("ROUTING & REGISTRY SERVICES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "bgp")]
        {
            output.push_str("AS15169-IRR         - IRR Explorer routing registry analysis\n");
            output.push_str("example: AS15169-IRR\n");
            output.push('\n');

            output.push_str("8.8.8.8-LG          - RIPE RIS Looking Glass query\n");
            output.push_str("example: 8.8.8.8-LG\n");
            output.push('\n');

            output.push_str("8.8.0.0/16-15169-RPKI - RPKI validation (prefix-asn-RPKI)\n");
            output.push_str("example: 8.8.0.0/16-15169-RPKI\n");
            output.push('\n');
        }

        #[cfg(feature = "bgp-irr")]
        {
            output.push_str("AS15169-RADB        - Routing Assets Database query\n");
            output.push_str("AS15169-ALTDB       - ALTDB routing registry query\n");
            output.push_str("AS15169-AFRINIC     - AFRINIC IRR query\n");
            output.push_str("AS15169-APNIC       - APNIC IRR query\n");
            output.push_str("AS15169-ARIN        - ARIN IRR query\n");
            output.push_str("AS15169-BELL        - BELL IRR query\n");
            output.push_str("AS15169-JPIRR       - JPIRR query\n");
            output.push_str("AS15169-LACNIC      - LACNIC IRR query\n");
            output.push_str("AS15169-LEVEL3      - LEVEL3 IRR query\n");
            output.push_str("AS15169-NTTCOM      - NTTCOM IRR query\n");
            output.push_str("AS15169-RIPE        - RIPE IRR query\n");
            output.push_str("AS15169-RIS         - RIPE RIS query\n");
            output.push_str("AS15169-TC          - TC IRR query\n");
            output.push('\n');
        }

        #[cfg(feature = "bgp-manrs")]
        {
            output.push_str("AS15169-MANRS       - MANRS (routing security) compliance\n");
            output.push_str("example: AS15169-MANRS\n");
            output.push('\n');
        }
    }

    #[cfg(any(feature = "dns", feature = "traceroute"))]
    {
        output.push_str("NETWORK DIAGNOSTICS:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "dns")]
        {
            output.push_str("google.com-DNS      - DNS resolution information\n");
            output.push_str("example: google.com-DNS\n");
            output.push('\n');
        }

        #[cfg(feature = "traceroute")]
        {
            output.push_str("google.com-TRACE    - Network traceroute to target\n");
            output.push_str("google.com-TRACEROUTE - Alternative traceroute format\n");
            output.push_str("example: google.com-TRACE\n");
            output.push('\n');
        }
    }

    #[cfg(any(feature = "ssl", feature = "crt"))]
    {
        output.push_str("SECURITY & CERTIFICATES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "ssl")]
        {
            output.push_str("google.com-SSL      - SSL/TLS certificate analysis\n");
            output.push_str("example: google.com-SSL\n");
            output.push('\n');
        }

        #[cfg(feature = "crt")]
        {
            output.push_str("google.com-CRT      - Certificate Transparency logs\n");
            output.push_str("example: google.com-CRT\n");
            output.push('\n');
        }
    }

    #[cfg(feature = "cfstatus")]
    {
        output.push_str("SERVICE STATUS:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("-CFSTATUS           - Cloudflare service status\n");
        output.push_str("components-CFSTATUS - Cloudflare components status\n");
        output.push_str("incidents-CFSTATUS  - Cloudflare unresolved incidents\n");
        output.push_str("example: -CFSTATUS\n");
        output.push('\n');
    }

    #[cfg(any(feature = "steam", feature = "minecraft"))]
    {
        output.push_str("GAMING SERVICES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "minecraft")]
        {
            output.push_str("mc.hypixel.net-MINECRAFT - Minecraft server status\n");
            output.push_str("mc.hypixel.net-MC   - Minecraft server status (short)\n");
            output.push_str("example: mc.hypixel.net-MINECRAFT\n");
            output.push('\n');
        }

        #[cfg(feature = "steam")]
        {
            output.push_str("730-STEAM           - Steam game/user information\n");
            output.push_str("example: 730-STEAM (Counter-Strike 2)\n");
            output.push('\n');
            output.push_str("Counter-Strike-STEAMSEARCH - Steam game search\n");
            output.push_str("example: Counter-Strike-STEAMSEARCH\n");
            output.push('\n');
        }
    }

    #[cfg(any(
        feature = "imdb",
        feature = "lyric",
        feature = "wikipedia",
        feature = "meal",
        feature = "acgc"
    ))]
    {
        output.push_str("MEDIA & ENTERTAINMENT:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "imdb")]
        {
            output.push_str("Inception-IMDB      - IMDb movie/TV show information\n");
            output.push_str("tt1375666-IMDB      - IMDb by ID (tt1375666 = Inception)\n");
            output.push_str("example: Inception-IMDB\n");
            output.push('\n');
            output.push_str("Batman-IMDBSEARCH   - IMDb title search\n");
            output.push_str("example: Batman-IMDBSEARCH\n");
            output.push('\n');
        }

        #[cfg(feature = "acgc")]
        {
            output.push_str("character-ACGC      - Anime/Comic/Game character query\n");
            output.push_str("example: Hatsune-ACGC\n");
            output.push('\n');
        }

        #[cfg(feature = "lyric")]
        {
            output.push_str("洛天依-LYRIC        - Luotianyi random lyrics\n");
            output.push_str("example: 洛天依-LYRIC\n");
            output.push('\n');
        }

        #[cfg(feature = "wikipedia")]
        {
            output.push_str("Hatsune-WIKIPEDIA   - Wikipedia article lookup\n");
            output.push_str("example: Rust_programming_language-WIKIPEDIA\n");
            output.push('\n');
        }

        #[cfg(feature = "meal")]
        {
            output.push_str("今天吃什么          - Random meal suggestion (TheMealDB)\n");
            output.push_str("example: 今天吃什么 or -MEAL\n");
            output.push('\n');
            output.push_str("今天吃什么中国      - Random Chinese recipe (HowToCook)\n");
            output.push_str("example: 今天吃什么中国 or -MEAL-CN\n");
            output.push('\n');
        }
    }

    #[cfg(any(
        feature = "pkg-cargo",
        feature = "pkg-pypi",
        feature = "pkg-npm",
        feature = "pkg-aur",
        feature = "pkg-debian",
        feature = "pkg-ubuntu",
        feature = "pkg-nixos",
        feature = "pkg-opensuse",
        feature = "pkg-aosc",
        feature = "pkg-modrinth",
        feature = "pkg-curseforge",
        feature = "pkg-alma",
        feature = "pkg-epel",
        feature = "pkg-openwrt",
    ))]
    {
        output.push_str("PACKAGE REPOSITORIES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        #[cfg(feature = "pkg-cargo")]
        {
            output.push_str("serde-CARGO         - Rust crates.io package information\n");
            output.push_str("example: serde-CARGO\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-pypi")]
        {
            output.push_str("requests-PYPI       - Python PyPI package information\n");
            output.push_str("example: requests-PYPI\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-npm")]
        {
            output.push_str("react-NPM           - Node.js NPM package information\n");
            output.push_str("example: react-NPM\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-aur")]
        {
            output.push_str("yay-AUR             - Arch User Repository packages\n");
            output.push_str("example: yay-AUR\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-debian")]
        {
            output.push_str("curl-DEBIAN         - Debian package information\n");
            output.push_str("example: curl-DEBIAN\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-ubuntu")]
        {
            output.push_str("firefox-UBUNTU      - Ubuntu package information\n");
            output.push_str("example: firefox-UBUNTU\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-nixos")]
        {
            output.push_str("nixpkgs-NIXOS       - NixOS package information\n");
            output.push_str("example: nixpkgs-NIXOS\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-opensuse")]
        {
            output.push_str("zypper-OPENSUSE     - OpenSUSE package information\n");
            output.push_str("example: zypper-OPENSUSE\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-aosc")]
        {
            output.push_str("htop-AOSC           - AOSC OS package information\n");
            output.push_str("example: htop-AOSC\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-alma")]
        {
            output.push_str("kernel-ALMA         - AlmaLinux package information\n");
            output.push_str("example: kernel-ALMA\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-epel")]
        {
            output.push_str("htop-EPEL           - EPEL package information\n");
            output.push_str("example: htop-EPEL\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-openwrt")]
        {
            output.push_str("luci-OPENWRT        - OpenWrt package information\n");
            output.push_str("example: luci-OPENWRT\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-modrinth")]
        {
            output.push_str("sodium-MODRINTH     - Modrinth mod/resource pack information\n");
            output.push_str("example: sodium-MODRINTH\n");
            output.push('\n');
        }
        #[cfg(feature = "pkg-curseforge")]
        {
            output.push_str("jei-CURSEFORGE      - CurseForge mod information\n");
            output.push_str("example: jei-CURSEFORGE or 238222-CURSEFORGE\n");
            output.push('\n');
        }
    }

    #[cfg(feature = "github")]
    {
        output.push_str("DEVELOPMENT SERVICES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("torvalds-GITHUB     - GitHub user/repository information\n");
        output.push_str("microsoft/vscode-GITHUB - GitHub repository info\n");
        output.push_str("example: torvalds-GITHUB\n");
        output.push('\n');
    }

    #[cfg(feature = "pixiv")]
    {
        output.push_str("PIXIV:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("123456-PIXIV        - Pixiv artwork information by ID\n");
        output.push_str("user:123456-PIXIV   - Pixiv user profile by ID\n");
        output.push_str("search:keyword-PIXIV - Search Pixiv artworks by keyword\n");
        output.push_str("ranking-PIXIV       - Daily Pixiv ranking\n");
        output.push_str("ranking:week-PIXIV  - Weekly Pixiv ranking\n");
        output.push_str("example: 114514-PIXIV, user:114514-PIXIV, search:hatsune miku-PIXIV\n");
        output.push('\n');
    }

    #[cfg(feature = "icp")]
    {
        output.push_str("CHINA SPECIAL:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("baidu.com-ICP       - ICP filing for Chinese domains\n");
        output.push_str("example: baidu.com-ICP\n");
        output.push('\n');
    }

    #[cfg(feature = "dn42")]
    {
        output.push_str("DN42 NETWORK QUERIES:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("example.dn42        - DN42 domain information\n");
        output.push_str("AS4242420000        - DN42 ASN information\n");
        output.push_str("172.20.0.0/16       - DN42 network blocks\n");
        output.push_str("fd42::/16           - DN42 IPv6 networks\n");
        output.push('\n');
    }

    #[cfg(feature = "pen")]
    {
        output.push_str("IANA PEN DATABASE:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("1234-PEN            - IANA Private Enterprise Numbers\n");
        output.push_str("example: 1234-PEN\n");
        output.push('\n');
    }

    #[cfg(feature = "ntp")]
    {
        output.push_str("NTP TIME SYNC:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("pool.ntp.org-NTP    - NTP time synchronization test\n");
        output.push_str("example: pool.ntp.org-NTP\n");
        output.push('\n');
    }

    #[cfg(feature = "ping")]
    {
        output.push_str("PING:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("8.8.8.8-PING        - ICMP ping test\n");
        output.push_str("example: 8.8.8.8-PING\n");
        output.push('\n');
    }

    #[cfg(feature = "rdap-explicit")]
    {
        output.push_str("RDAP PROTOCOL:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');
        output.push_str("example.com-RDAP    - Force RDAP protocol query\n");
        output.push_str("example: example.com-RDAP\n");
        output.push('\n');
    }

    output.push_str("SPECIAL COMMANDS:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');
    output.push_str("HELP                - Show this help message\n");
    output.push('\n');

    output.push_str("DESC FILTER:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');
    output.push_str("<query>-DESC        - Show only description fields\n");
    output.push_str("example: AS15169-DESC\n");
    output.push('\n');

    output.push_str("WHOIS-COLOR PROTOCOL:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');
    output.push_str("This server supports WHOIS-COLOR protocol v1.0 for enhanced output.\n");
    output.push_str("Send 'X-WHOIS-COLOR-PROBE: 1' to detect color support.\n");
    output.push_str("Use 'X-WHOIS-COLOR: ripe' or 'X-WHOIS-COLOR: bgptools' for colored output.\n");
    output.push('\n');

    output.push_str("SYSTEM COMMANDS:\n");
    output.push_str(&"-".repeat(40));
    output.push('\n');
    output.push_str("HELP                - Show this help message\n");
    output.push_str("UPDATE-PATCH        - Update response patches from remote repository\n");
    output.push('\n');

    output.push_str("% This help information is provided by WHOIS server\n");
    output
}
