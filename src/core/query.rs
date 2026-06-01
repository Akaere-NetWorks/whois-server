use crate::config::{PRIVATE_IPV4_RANGES, PRIVATE_IPV6_RANGES};
use cidr::{Ipv4Cidr, Ipv6Cidr};
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;
#[cfg(feature = "plugins")]
use std::sync::RwLock;

// WHOIS query types
#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum QueryType {
    // Core (always compiled)
    Domain(String),
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    ASN(String),
    BareASN(String),
    Desc(String),
    Rdap(String),
    Help,
    UpdatePatch,
    Unknown(String),

    // feature = "email"
    #[cfg(feature = "email")]
    EmailSearch(String),

    // feature = "bgp"
    #[cfg(feature = "bgp")]
    BGPTool(String),
    #[cfg(feature = "bgp")]
    Irr(String),
    #[cfg(feature = "bgp")]
    LookingGlass(String),
    #[cfg(feature = "bgp")]
    Rpki(String, String),

    // feature = "bgp-prefixes" (depends on bgp)
    #[cfg(feature = "bgp-prefixes")]
    Prefixes(String),

    // feature = "bgp-irr"
    #[cfg(feature = "bgp-irr")]
    Radb(String),
    #[cfg(feature = "bgp-irr")]
    Altdb(String),
    #[cfg(feature = "bgp-irr")]
    Afrinic(String),
    #[cfg(feature = "bgp-irr")]
    Apnic(String),
    #[cfg(feature = "bgp-irr")]
    ArinIrr(String),
    #[cfg(feature = "bgp-irr")]
    Bell(String),
    #[cfg(feature = "bgp-irr")]
    Jpirr(String),
    #[cfg(feature = "bgp-irr")]
    Lacnic(String),
    #[cfg(feature = "bgp-irr")]
    Level3(String),
    #[cfg(feature = "bgp-irr")]
    Nttcom(String),
    #[cfg(feature = "bgp-irr")]
    RipeIrr(String),
    #[cfg(feature = "bgp-irr")]
    Ris(String),
    #[cfg(feature = "bgp-irr")]
    Tc(String),

    // feature = "bgp-manrs"
    #[cfg(feature = "bgp-manrs")]
    Manrs(String),

    // feature = "peeringdb"
    #[cfg(feature = "peeringdb")]
    PeeringDB(String),

    // feature = "geo"
    #[cfg(feature = "geo")]
    Geo(String),
    #[cfg(feature = "geo")]
    RirGeo(String),

    // feature = "dns"
    #[cfg(feature = "dns")]
    Dns(String),

    // feature = "ntp"
    #[cfg(feature = "ntp")]
    Ntp(String),

    // feature = "ping"
    #[cfg(feature = "ping")]
    Ping(String),

    // feature = "traceroute"
    #[cfg(feature = "traceroute")]
    Trace(String),

    // feature = "ssl"
    #[cfg(feature = "ssl")]
    Ssl(String),

    // feature = "crt"
    #[cfg(feature = "crt")]
    Crt(String),

    // feature = "cfstatus"
    #[cfg(feature = "cfstatus")]
    CfStatus(String),

    // feature = "minecraft"
    #[cfg(feature = "minecraft")]
    Minecraft(String),
    #[cfg(feature = "minecraft")]
    MinecraftUser(String),

    // feature = "steam"
    #[cfg(feature = "steam")]
    Steam(String),
    #[cfg(feature = "steam")]
    SteamSearch(String),

    // feature = "imdb"
    #[cfg(feature = "imdb")]
    Imdb(String),
    #[cfg(feature = "imdb")]
    ImdbSearch(String),

    // feature = "acgc"
    #[cfg(feature = "acgc")]
    Acgc(String),

    // feature = "pkg-alma"
    #[cfg(feature = "pkg-alma")]
    Alma(String),
    // feature = "pkg-aosc"
    #[cfg(feature = "pkg-aosc")]
    Aosc(String),
    // feature = "pkg-aur"
    #[cfg(feature = "pkg-aur")]
    Aur(String),
    // feature = "pkg-debian"
    #[cfg(feature = "pkg-debian")]
    Debian(String),
    // feature = "pkg-epel"
    #[cfg(feature = "pkg-epel")]
    Epel(String),
    // feature = "pkg-ubuntu"
    #[cfg(feature = "pkg-ubuntu")]
    Ubuntu(String),
    // feature = "pkg-nixos"
    #[cfg(feature = "pkg-nixos")]
    NixOs(String),
    // feature = "pkg-opensuse"
    #[cfg(feature = "pkg-opensuse")]
    OpenSuse(String),
    // feature = "pkg-openwrt"
    #[cfg(feature = "pkg-openwrt")]
    OpenWrt(String),
    // feature = "pkg-npm"
    #[cfg(feature = "pkg-npm")]
    Npm(String),
    // feature = "pkg-pypi"
    #[cfg(feature = "pkg-pypi")]
    Pypi(String),
    // feature = "pkg-cargo"
    #[cfg(feature = "pkg-cargo")]
    Cargo(String),
    // feature = "pkg-modrinth"
    #[cfg(feature = "pkg-modrinth")]
    Modrinth(String),
    // feature = "pkg-curseforge"
    #[cfg(feature = "pkg-curseforge")]
    CurseForge(String),

    // feature = "github"
    #[cfg(feature = "github")]
    GitHub(String),

    // feature = "wikipedia"
    #[cfg(feature = "wikipedia")]
    Wikipedia(String),

    // feature = "lyric"
    #[cfg(feature = "lyric")]
    Lyric(String),

    // feature = "pen"
    #[cfg(feature = "pen")]
    Pen(String),

    // feature = "pixiv"
    #[cfg(feature = "pixiv")]
    Pixiv(String),

    // feature = "icp"
    #[cfg(feature = "icp")]
    Icp(String),

    // feature = "meal"
    #[cfg(feature = "meal")]
    Meal,
    #[cfg(feature = "meal")]
    MealCN,

    // feature = "plugins"
    #[cfg(feature = "plugins")]
    Plugin(String, String),
}

static DOMAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$")
        .expect("Invalid domain regex pattern")
});

pub fn analyze_query(query: &str) -> QueryType {
    let query_upper = query.to_uppercase();
    // Check if it's a Chinese meal suggestion query
    #[cfg(feature = "meal")]
    if query == "今天吃什么中国" || query_upper.ends_with("-MEAL-CN") {
        return QueryType::MealCN;
    }

    // Check if it's a meal suggestion query (Chinese phrase or -MEAL suffix)
    #[cfg(feature = "meal")]
    if query == "今天吃什么" || query_upper.ends_with("-MEAL") {
        return QueryType::Meal;
    }

    // Check if it's a HELP query (case-insensitive)
    if query_upper == "HELP" {
        return QueryType::Help;
    }

    // Check if it's an UPDATE-PATCH query (case-insensitive)
    if query_upper == "UPDATE-PATCH" || query_upper == "-UPDATE-PATCH" {
        return QueryType::UpdatePatch;
    }

    // Check if it's an RPKI query in format PREFIX-ASN-RPKI
    #[cfg(feature = "bgp")]
    if query_upper.ends_with("-RPKI") {
        let base_query = &query[..query.len() - 5]; // Remove "-RPKI" suffix

        // Try to parse as prefix-asn format
        if let Some(dash_pos) = base_query.rfind('-') {
            let prefix_part = &base_query[..dash_pos];
            let asn_part = &base_query[dash_pos + 1..];

            // Validate that ASN part is numeric
            if asn_part.chars().all(|c| c.is_ascii_digit()) {
                // Validate prefix part (IP/CIDR format)
                if prefix_part.parse::<Ipv4Cidr>().is_ok() {
                    return QueryType::Rpki(prefix_part.to_string(), asn_part.to_string());
                }
                if prefix_part.parse::<Ipv6Cidr>().is_ok() {
                    return QueryType::Rpki(prefix_part.to_string(), asn_part.to_string());
                }
                // Also try single IP address
                if let Ok(ip) = prefix_part.parse::<IpAddr>() {
                    match ip {
                        IpAddr::V4(_) => {
                            return QueryType::Rpki(
                                format!("{}/32", prefix_part),
                                asn_part.to_string(),
                            );
                        }
                        IpAddr::V6(_) => {
                            return QueryType::Rpki(
                                format!("{}/128", prefix_part),
                                asn_part.to_string(),
                            );
                        }
                    }
                }
            }
        }

        // If parsing failed, treat as unknown
        return QueryType::Unknown(query.to_string());
    }

    // Check if it's a Looking Glass query
    #[cfg(feature = "bgp")]
    if query_upper.ends_with("-LG") {
        let base_query = &query[..query.len() - 3]; // Remove "-LG" suffix
        return QueryType::LookingGlass(base_query.to_string());
    }

    // Check if it's an IRR Explorer query
    #[cfg(feature = "bgp")]
    if query_upper.ends_with("-IRR") {
        let base_query = &query[..query.len() - 4]; // Remove "-IRR" suffix
        return QueryType::Irr(base_query.to_string());
    }

    // Check if it's a RADB query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-RADB") {
        let base_query = &query[..query.len() - 5]; // Remove "-RADB" suffix
        return QueryType::Radb(base_query.to_string());
    }

    // Check if it's an ALTDB query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-ALTDB") {
        let base_query = &query[..query.len() - 6]; // Remove "-ALTDB" suffix
        return QueryType::Altdb(base_query.to_string());
    }

    // Check if it's an AFRINIC IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-AFRINIC") {
        let base_query = &query[..query.len() - 8]; // Remove "-AFRINIC" suffix
        return QueryType::Afrinic(base_query.to_string());
    }

    // Check if it's an APNIC IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-APNIC") {
        let base_query = &query[..query.len() - 6]; // Remove "-APNIC" suffix
        return QueryType::Apnic(base_query.to_string());
    }

    // Check if it's an ARIN IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-ARIN") {
        let base_query = &query[..query.len() - 5]; // Remove "-ARIN" suffix
        return QueryType::ArinIrr(base_query.to_string());
    }

    // Check if it's a BELL IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-BELL") {
        let base_query = &query[..query.len() - 5]; // Remove "-BELL" suffix
        return QueryType::Bell(base_query.to_string());
    }

    // Check if it's a JPIRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-JPIRR") {
        let base_query = &query[..query.len() - 6]; // Remove "-JPIRR" suffix
        return QueryType::Jpirr(base_query.to_string());
    }

    // Check if it's a LACNIC IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-LACNIC") {
        let base_query = &query[..query.len() - 7]; // Remove "-LACNIC" suffix
        return QueryType::Lacnic(base_query.to_string());
    }

    // Check if it's a LEVEL3 IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-LEVEL3") {
        let base_query = &query[..query.len() - 7]; // Remove "-LEVEL3" suffix
        return QueryType::Level3(base_query.to_string());
    }

    // Check if it's an NTTCOM IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-NTTCOM") {
        let base_query = &query[..query.len() - 7]; // Remove "-NTTCOM" suffix
        return QueryType::Nttcom(base_query.to_string());
    }

    // Check if it's a RIPE IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-RIPE") {
        let base_query = &query[..query.len() - 5]; // Remove "-RIPE" suffix
        return QueryType::RipeIrr(base_query.to_string());
    }

    // Check if it's a RIS query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-RIS") {
        let base_query = &query[..query.len() - 4]; // Remove "-RIS" suffix
        return QueryType::Ris(base_query.to_string());
    }

    // Check if it's a TC IRR query
    #[cfg(feature = "bgp-irr")]
    if query_upper.ends_with("-TC") {
        let base_query = &query[..query.len() - 3]; // Remove "-TC" suffix
        return QueryType::Tc(base_query.to_string());
    }

    // Check if it's a MANRS query
    #[cfg(feature = "bgp-manrs")]
    if query_upper.ends_with("-MANRS") {
        let base_query = &query[..query.len() - 6]; // Remove "-MANRS" suffix
        return QueryType::Manrs(base_query.to_string());
    }

    // Check if it's a DNS query
    #[cfg(feature = "dns")]
    if query_upper.ends_with("-DNS") {
        let base_query = &query[..query.len() - 4]; // Remove "-DNS" suffix
        return QueryType::Dns(base_query.to_string());
    }

    // Check if it's an NTP query
    #[cfg(feature = "ntp")]
    if query_upper.ends_with("-NTP") {
        let base_query = &query[..query.len() - 4]; // Remove "-NTP" suffix
        return QueryType::Ntp(base_query.to_string());
    }

    // Check if it's a ping query
    #[cfg(feature = "ping")]
    if query_upper.ends_with("-PING") {
        let base_query = &query[..query.len() - 5]; // Remove "-PING" suffix
        return QueryType::Ping(base_query.to_string());
    }

    // Check if it's a traceroute query (long form)
    #[cfg(feature = "traceroute")]
    if query_upper.ends_with("-TRACEROUTE") {
        let base_query = &query[..query.len() - 11]; // Remove "-TRACEROUTE" suffix
        return QueryType::Trace(base_query.to_string());
    }

    // Check if it's a traceroute query (short form)
    #[cfg(feature = "traceroute")]
    if query_upper.ends_with("-TRACE") {
        let base_query = &query[..query.len() - 6]; // Remove "-TRACE" suffix
        return QueryType::Trace(base_query.to_string());
    }

    // Check if it's an SSL certificate query
    #[cfg(feature = "ssl")]
    if query_upper.ends_with("-SSL") {
        let base_query = &query[..query.len() - 4]; // Remove "-SSL" suffix
        return QueryType::Ssl(base_query.to_string());
    }

    // Check if it's a Certificate Transparency query
    #[cfg(feature = "crt")]
    if query_upper.ends_with("-CRT") {
        let base_query = &query[..query.len() - 4]; // Remove "-CRT" suffix
        return QueryType::Crt(base_query.to_string());
    }

    // Check if it's a Cloudflare Status query
    #[cfg(feature = "cfstatus")]
    if query_upper.ends_with("-CFSTATUS") {
        let base_query = &query[..query.len() - 9]; // Remove "-CFSTATUS" suffix
        return QueryType::CfStatus(base_query.to_string());
    }

    // Check if it's a Minecraft server query
    #[cfg(feature = "minecraft")]
    if query_upper.ends_with("-MINECRAFT") {
        let base_query = &query[..query.len() - 10]; // Remove "-MINECRAFT" suffix
        return QueryType::Minecraft(base_query.to_string());
    }

    // Check if it's a Minecraft user query
    #[cfg(feature = "minecraft")]
    if query_upper.ends_with("-MCU") {
        let base_query = &query[..query.len() - 4]; // Remove "-MCU" suffix
        return QueryType::MinecraftUser(base_query.to_string());
    }

    // Check if it's a Minecraft server query (short form)
    #[cfg(feature = "minecraft")]
    if query_upper.ends_with("-MC") {
        let base_query = &query[..query.len() - 3]; // Remove "-MC" suffix
        return QueryType::Minecraft(base_query.to_string());
    }

    // Check if it's a Steam search query (must be checked before regular Steam query)
    #[cfg(feature = "steam")]
    if query_upper.ends_with("-STEAMSEARCH") {
        let base_query = &query[..query.len() - 12]; // Remove "-STEAMSEARCH" suffix
        return QueryType::SteamSearch(base_query.to_string());
    }

    // Check if it's a Steam game/user query
    #[cfg(feature = "steam")]
    if query_upper.ends_with("-STEAM") {
        let base_query = &query[..query.len() - 6]; // Remove "-STEAM" suffix
        return QueryType::Steam(base_query.to_string());
    }

    // Check if it's an IMDb search query (must be checked before regular IMDb query)
    #[cfg(feature = "imdb")]
    if query_upper.ends_with("-IMDBSEARCH") {
        let base_query = &query[..query.len() - 11]; // Remove "-IMDBSEARCH" suffix
        return QueryType::ImdbSearch(base_query.to_string());
    }

    // Check if it's an IMDb movie/TV show query
    #[cfg(feature = "imdb")]
    if query_upper.ends_with("-IMDB") {
        let base_query = &query[..query.len() - 5]; // Remove "-IMDB" suffix
        return QueryType::Imdb(base_query.to_string());
    }

    // Check if it's an ACGC character query
    #[cfg(feature = "acgc")]
    if query_upper.ends_with("-ACGC") {
        let base_query = &query[..query.len() - 5]; // Remove "-ACGC" suffix
        return QueryType::Acgc(base_query.to_string());
    }

    // Check if it's an AlmaLinux package query
    #[cfg(feature = "pkg-alma")]
    if query_upper.ends_with("-ALMA") {
        let base_query = &query[..query.len() - 5]; // Remove "-ALMA" suffix
        return QueryType::Alma(base_query.to_string());
    }

    // Check if it's an AOSC package query
    #[cfg(feature = "pkg-aosc")]
    if query_upper.ends_with("-AOSC") {
        let base_query = &query[..query.len() - 5]; // Remove "-AOSC" suffix
        return QueryType::Aosc(base_query.to_string());
    }

    // Check if it's an AUR package query
    #[cfg(feature = "pkg-aur")]
    if query_upper.ends_with("-AUR") {
        let base_query = &query[..query.len() - 4]; // Remove "-AUR" suffix
        return QueryType::Aur(base_query.to_string());
    }

    // Check if it's a Debian package query
    #[cfg(feature = "pkg-debian")]
    if query_upper.ends_with("-DEBIAN") {
        let base_query = &query[..query.len() - 7]; // Remove "-DEBIAN" suffix
        return QueryType::Debian(base_query.to_string());
    }

    // Check if it's an EPEL package query
    #[cfg(feature = "pkg-epel")]
    if query_upper.ends_with("-EPEL") {
        let base_query = &query[..query.len() - 5]; // Remove "-EPEL" suffix
        return QueryType::Epel(base_query.to_string());
    }

    // Check if it's an Ubuntu package query
    #[cfg(feature = "pkg-ubuntu")]
    if query_upper.ends_with("-UBUNTU") {
        let base_query = &query[..query.len() - 7]; // Remove "-UBUNTU" suffix
        return QueryType::Ubuntu(base_query.to_string());
    }

    // Check if it's a NixOS package query
    #[cfg(feature = "pkg-nixos")]
    if query_upper.ends_with("-NIXOS") {
        let base_query = &query[..query.len() - 6]; // Remove "-NIXOS" suffix
        return QueryType::NixOs(base_query.to_string());
    }

    // Check if it's an OpenSUSE package query
    #[cfg(feature = "pkg-opensuse")]
    if query_upper.ends_with("-OPENSUSE") {
        let base_query = &query[..query.len() - 9]; // Remove "-OPENSUSE" suffix
        return QueryType::OpenSuse(base_query.to_string());
    }

    // Check if it's an OpenWrt package query
    #[cfg(feature = "pkg-openwrt")]
    if query_upper.ends_with("-OPENWRT") {
        let base_query = &query[..query.len() - 8]; // Remove "-OPENWRT" suffix
        return QueryType::OpenWrt(base_query.to_string());
    }

    // Check if it's an NPM package query
    #[cfg(feature = "pkg-npm")]
    if query_upper.ends_with("-NPM") {
        let base_query = &query[..query.len() - 4]; // Remove "-NPM" suffix
        return QueryType::Npm(base_query.to_string());
    }

    // Check if it's a PyPI package query
    #[cfg(feature = "pkg-pypi")]
    if query_upper.ends_with("-PYPI") {
        let base_query = &query[..query.len() - 5]; // Remove "-PYPI" suffix
        return QueryType::Pypi(base_query.to_string());
    }

    // Check if it's a Cargo (Rust crates) package query
    #[cfg(feature = "pkg-cargo")]
    if query_upper.ends_with("-CARGO") {
        let base_query = &query[..query.len() - 6]; // Remove "-CARGO" suffix
        return QueryType::Cargo(base_query.to_string());
    }

    // Check if it's a Modrinth mod/resource pack query
    #[cfg(feature = "pkg-modrinth")]
    if query_upper.ends_with("-MODRINTH") {
        let base_query = &query[..query.len() - 9]; // Remove "-MODRINTH" suffix
        return QueryType::Modrinth(base_query.to_string());
    }

    // Check if it's a CurseForge mod query
    #[cfg(feature = "pkg-curseforge")]
    if query_upper.ends_with("-CURSEFORGE") {
        let base_query = &query[..query.len() - 11]; // Remove "-CURSEFORGE" suffix
        return QueryType::CurseForge(base_query.to_string());
    }

    // Check if it's a GitHub user/repository query
    #[cfg(feature = "github")]
    if query_upper.ends_with("-GITHUB") {
        let base_query = &query[..query.len() - 7]; // Remove "-GITHUB" suffix
        return QueryType::GitHub(base_query.to_string());
    }

    // Check if it's a Wikipedia article query
    #[cfg(feature = "wikipedia")]
    if query_upper.ends_with("-WIKIPEDIA") {
        let base_query = &query[..query.len() - 10]; // Remove "-WIKIPEDIA" suffix
        return QueryType::Wikipedia(base_query.to_string());
    }

    // Check if it's a Luotianyi lyric query
    #[cfg(feature = "lyric")]
    if query_upper.ends_with("-LYRIC") {
        let base_query = &query[..query.len() - 6]; // Remove "-LYRIC" suffix
        return QueryType::Lyric(base_query.to_string());
    }

    // Check if it's a description-only query
    if query_upper.ends_with("-DESC") {
        let base_query = &query[..query.len() - 5]; // Remove "-DESC" suffix
        return QueryType::Desc(base_query.to_string());
    }

    // Check if it's a PeeringDB query
    #[cfg(feature = "peeringdb")]
    if query_upper.ends_with("-PEERINGDB") {
        let base_query = &query[..query.len() - 10]; // Remove "-PEERINGDB" suffix
        return QueryType::PeeringDB(base_query.to_string());
    }

    // Check if it's a IANA Private Enterprise Numbers query
    #[cfg(feature = "pen")]
    if query_upper.ends_with("-PEN") {
        let base_query = &query[..query.len() - 4]; // Remove "-PEN" suffix
        return QueryType::Pen(base_query.to_string());
    }

    // Check if it's a RDAP query
    #[cfg(feature = "rdap-explicit")]
    if query_upper.ends_with("-RDAP") {
        let base_query = &query[..query.len() - 5]; // Remove "-RDAP" suffix
        return QueryType::Rdap(base_query.to_string());
    }

    // Check if it's a Pixiv query
    #[cfg(feature = "pixiv")]
    if query_upper.ends_with("-PIXIV") {
        let base_query = &query[..query.len() - 6]; // Remove "-PIXIV" suffix
        return QueryType::Pixiv(base_query.to_string());
    }

    // Check if it's an ICP filing query
    #[cfg(feature = "icp")]
    if query_upper.ends_with("-ICP") {
        let base_query = &query[..query.len() - 4]; // Remove "-ICP" suffix
        return QueryType::Icp(base_query.to_string());
    }

    // Check if it's a BGP Tools query
    #[cfg(feature = "bgp")]
    if query_upper.ends_with("-BGPTOOL") {
        let base_query = &query[..query.len() - 8]; // Remove "-BGPTOOL" suffix
        return QueryType::BGPTool(base_query.to_string());
    }

    // Check if it's a prefixes query
    #[cfg(feature = "bgp-prefixes")]
    if query_upper.ends_with("-PREFIXES") {
        let base_query = &query[..query.len() - 9]; // Remove "-PREFIXES" suffix
        return QueryType::Prefixes(base_query.to_string());
    }

    // Check if it's a RIR geo query
    #[cfg(feature = "geo")]
    if query_upper.ends_with("-RIRGEO") {
        let base_query = &query[..query.len() - 7]; // Remove "-RIRGEO" suffix
        return QueryType::RirGeo(base_query.to_string());
    }

    // Check if it's a geo query
    #[cfg(feature = "geo")]
    if query_upper.ends_with("-GEO") {
        let base_query = &query[..query.len() - 4]; // Remove "-GEO" suffix
        return QueryType::Geo(base_query.to_string());
    }

    // Check if it's an email search query
    #[cfg(feature = "email")]
    if query_upper.ends_with("-EMAIL") {
        let base_query = &query[..query.len() - 6]; // Remove "-EMAIL" suffix
        return QueryType::EmailSearch(base_query.to_string());
    }

    // Check if it's a .dn42 domain
    #[cfg(feature = "dn42")]
    if query_upper.ends_with(".DN42") {
        return QueryType::Domain(query.to_string());
    }

    // Check if it has -DN42 suffix or ends with -MNT
    #[cfg(feature = "dn42")]
    if query_upper.ends_with("-DN42") || query_upper.ends_with("-MNT") {
        return QueryType::Unknown(query.to_string());
    }

    // Try to parse as IP address
    if let Ok(ip) = query.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => {
                return QueryType::IPv4(ipv4);
            }
            IpAddr::V6(ipv6) => {
                return QueryType::IPv6(ipv6);
            }
        }
    }

    // Try to parse as CIDR format
    if let Ok(cidr) = query.parse::<Ipv4Cidr>() {
        return QueryType::IPv4(cidr.first_address());
    }

    if let Ok(cidr) = query.parse::<Ipv6Cidr>() {
        return QueryType::IPv6(cidr.first_address());
    }

    // Identify ASN
    if query_upper.starts_with("AS") && query[2..].chars().all(|c| c.is_ascii_digit()) {
        return QueryType::ASN(query.to_string());
    }

    // Identify bare ASN (pure number, e.g., "213605")
    // DN42 first, then fallback to public RDAP/WHOIS
    if query.chars().all(|c| c.is_ascii_digit()) && !query.is_empty() {
        return QueryType::BareASN(query.to_string());
    }

    // Check if it's a domain format
    if DOMAIN_REGEX.is_match(query) {
        return QueryType::Domain(query.to_string());
    }

    // Check for plugin-registered suffixes (before Unknown)
    #[cfg(feature = "plugins")]
    if let Some(plugin_registry) = get_plugin_registry() {
        for suffix in plugin_registry.get_all_suffixes() {
            if query_upper.ends_with(suffix.as_str()) {
                let base_query = &query[..query.len() - suffix.len()];
                return QueryType::Plugin(suffix, base_query.to_string());
            }
        }
    }

    // Default to unknown type
    QueryType::Unknown(query.to_string())
}

static PRIVATE_IPV4_CIDRS: LazyLock<Vec<Ipv4Cidr>> = LazyLock::new(|| {
    PRIVATE_IPV4_RANGES
        .iter()
        .filter_map(|r| r.parse().ok())
        .collect()
});

static PRIVATE_IPV6_CIDRS: LazyLock<Vec<Ipv6Cidr>> = LazyLock::new(|| {
    PRIVATE_IPV6_RANGES
        .iter()
        .filter_map(|r| r.parse().ok())
        .collect()
});

pub fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    PRIVATE_IPV4_CIDRS.iter().any(|cidr| cidr.contains(&ip))
}

pub fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    PRIVATE_IPV6_CIDRS.iter().any(|cidr| cidr.contains(&ip))
}

// Global plugin registry (shared across all threads)
#[cfg(feature = "plugins")]
use crate::plugins::PluginRegistry;
#[cfg(feature = "plugins")]
use std::sync::Arc;

#[cfg(feature = "plugins")]
static PLUGIN_REGISTRY: RwLock<Option<Arc<PluginRegistry>>> = RwLock::new(None);

/// Set the global plugin registry
#[cfg(feature = "plugins")]
pub fn set_plugin_registry(registry: Arc<PluginRegistry>) {
    let mut guard = PLUGIN_REGISTRY.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(registry);
}

/// Get the global plugin registry
#[cfg(feature = "plugins")]
pub fn get_plugin_registry() -> Option<Arc<PluginRegistry>> {
    let guard = PLUGIN_REGISTRY.read().unwrap_or_else(|e| e.into_inner());
    guard.clone()
}
