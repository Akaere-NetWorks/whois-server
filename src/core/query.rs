use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use cidr::{Ipv4Cidr, Ipv6Cidr};
use regex::Regex;
use crate::config::{PRIVATE_IPV4_RANGES, PRIVATE_IPV6_RANGES};

// WHOIS query types
#[derive(Debug, Clone)]
pub enum QueryType {
    Domain(String),
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    ASN(String),
    EmailSearch(String), // For queries ending with -EMAIL
    BGPTool(String),     // For queries ending with -BGPTOOL
    Geo(String),         // For queries ending with -GEO
    RirGeo(String),      // For queries ending with -RIRGEO
    Prefixes(String),    // For queries ending with -PREFIXES
    Radb(String),        // For queries ending with -RADB
    Irr(String),         // For queries ending with -IRR
    LookingGlass(String), // For queries ending with -LG
    Rpki(String, String), // For queries in format prefix-asn-RPKI (prefix, asn)
    Manrs(String),       // For queries ending with -MANRS
    Dns(String),         // For queries ending with -DNS
    Trace(String),       // For queries ending with -TRACE
    Ssl(String),         // For queries ending with -SSL
    Crt(String),         // For queries ending with -CRT (Certificate Transparency)
    Minecraft(String),   // For queries ending with -MINECRAFT or -MC
    MinecraftUser(String), // For queries ending with -MCU (Minecraft user info)
    Steam(String),       // For queries ending with -STEAM (Steam games/users)
    SteamSearch(String), // For queries ending with -STEAMSEARCH (Steam game search)
    Imdb(String),        // For queries ending with -IMDB (IMDb movies/TV shows)
    ImdbSearch(String),  // For queries ending with -IMDBSEARCH (IMDb title search)
    Acgc(String),        // For queries ending with -ACGC (Anime/Comic/Game Characters)
    Aur(String),         // For queries ending with -AUR (Arch User Repository)
    Debian(String),      // For queries ending with -DEBIAN (Debian packages)
    Wikipedia(String),   // For queries ending with -WIKIPEDIA (Wikipedia articles)
    Unknown(String),
}

pub fn analyze_query(query: &str) -> QueryType {
    // Check if it's an RPKI query in format PREFIX-ASN-RPKI
    if query.to_uppercase().ends_with("-RPKI") {
        let base_query = &query[..query.len() - 5]; // Remove "-RPKI" suffix
        
        // Try to parse as prefix-asn format
        if let Some(dash_pos) = base_query.rfind('-') {
            let prefix_part = &base_query[..dash_pos];
            let asn_part = &base_query[dash_pos + 1..];
            
            // Validate that ASN part is numeric
            if asn_part.chars().all(|c| c.is_digit(10)) {
                // Validate prefix part (IP/CIDR format)
                if let Ok(_) = prefix_part.parse::<Ipv4Cidr>() {
                    return QueryType::Rpki(prefix_part.to_string(), asn_part.to_string());
                }
                if let Ok(_) = prefix_part.parse::<Ipv6Cidr>() {
                    return QueryType::Rpki(prefix_part.to_string(), asn_part.to_string());
                }
                // Also try single IP address
                if let Ok(ip) = prefix_part.parse::<IpAddr>() {
                    match ip {
                        IpAddr::V4(_) => return QueryType::Rpki(format!("{}/32", prefix_part), asn_part.to_string()),
                        IpAddr::V6(_) => return QueryType::Rpki(format!("{}/128", prefix_part), asn_part.to_string()),
                    }
                }
            }
        }
        
        // If parsing failed, treat as unknown
        return QueryType::Unknown(query.to_string());
    }
    
    // Check if it's a Looking Glass query
    if query.to_uppercase().ends_with("-LG") {
        let base_query = &query[..query.len() - 3]; // Remove "-LG" suffix
        return QueryType::LookingGlass(base_query.to_string());
    }
    
    // Check if it's an IRR Explorer query
    if query.to_uppercase().ends_with("-IRR") {
        let base_query = &query[..query.len() - 4]; // Remove "-IRR" suffix
        return QueryType::Irr(base_query.to_string());
    }
    
    // Check if it's a RADB query
    if query.to_uppercase().ends_with("-RADB") {
        let base_query = &query[..query.len() - 5]; // Remove "-RADB" suffix
        return QueryType::Radb(base_query.to_string());
    }
    
    // Check if it's a MANRS query
    if query.to_uppercase().ends_with("-MANRS") {
        let base_query = &query[..query.len() - 6]; // Remove "-MANRS" suffix
        return QueryType::Manrs(base_query.to_string());
    }
    
    // Check if it's a DNS query
    if query.to_uppercase().ends_with("-DNS") {
        let base_query = &query[..query.len() - 4]; // Remove "-DNS" suffix
        return QueryType::Dns(base_query.to_string());
    }
    
    // Check if it's a traceroute query (long form)
    if query.to_uppercase().ends_with("-TRACEROUTE") {
        let base_query = &query[..query.len() - 11]; // Remove "-TRACEROUTE" suffix
        return QueryType::Trace(base_query.to_string());
    }
    
    // Check if it's a traceroute query (short form)
    if query.to_uppercase().ends_with("-TRACE") {
        let base_query = &query[..query.len() - 6]; // Remove "-TRACE" suffix
        return QueryType::Trace(base_query.to_string());
    }
    
    // Check if it's an SSL certificate query
    if query.to_uppercase().ends_with("-SSL") {
        let base_query = &query[..query.len() - 4]; // Remove "-SSL" suffix
        return QueryType::Ssl(base_query.to_string());
    }
    
    // Check if it's a Certificate Transparency query
    if query.to_uppercase().ends_with("-CRT") {
        let base_query = &query[..query.len() - 4]; // Remove "-CRT" suffix
        return QueryType::Crt(base_query.to_string());
    }
    
    // Check if it's a Minecraft server query
    if query.to_uppercase().ends_with("-MINECRAFT") {
        let base_query = &query[..query.len() - 10]; // Remove "-MINECRAFT" suffix
        return QueryType::Minecraft(base_query.to_string());
    }
    
    // Check if it's a Minecraft user query
    if query.to_uppercase().ends_with("-MCU") {
        let base_query = &query[..query.len() - 4]; // Remove "-MCU" suffix
        return QueryType::MinecraftUser(base_query.to_string());
    }
    
    // Check if it's a Minecraft server query (short form)
    if query.to_uppercase().ends_with("-MC") {
        let base_query = &query[..query.len() - 3]; // Remove "-MC" suffix
        return QueryType::Minecraft(base_query.to_string());
    }
    
    // Check if it's a Steam search query (must be checked before regular Steam query)
    if query.to_uppercase().ends_with("-STEAMSEARCH") {
        let base_query = &query[..query.len() - 12]; // Remove "-STEAMSEARCH" suffix
        return QueryType::SteamSearch(base_query.to_string());
    }
    
    // Check if it's a Steam game/user query
    if query.to_uppercase().ends_with("-STEAM") {
        let base_query = &query[..query.len() - 6]; // Remove "-STEAM" suffix
        return QueryType::Steam(base_query.to_string());
    }
    
    // Check if it's an IMDb search query (must be checked before regular IMDb query)
    if query.to_uppercase().ends_with("-IMDBSEARCH") {
        let base_query = &query[..query.len() - 11]; // Remove "-IMDBSEARCH" suffix
        return QueryType::ImdbSearch(base_query.to_string());
    }
    
    // Check if it's an IMDb movie/TV show query
    if query.to_uppercase().ends_with("-IMDB") {
        let base_query = &query[..query.len() - 5]; // Remove "-IMDB" suffix
        return QueryType::Imdb(base_query.to_string());
    }
    
    // Check if it's an ACGC character query
    if query.to_uppercase().ends_with("-ACGC") {
        let base_query = &query[..query.len() - 5]; // Remove "-ACGC" suffix
        return QueryType::Acgc(base_query.to_string());
    }
    
    // Check if it's an AUR package query
    if query.to_uppercase().ends_with("-AUR") {
        let base_query = &query[..query.len() - 4]; // Remove "-AUR" suffix
        return QueryType::Aur(base_query.to_string());
    }
    
    // Check if it's a Debian package query
    if query.to_uppercase().ends_with("-DEBIAN") {
        let base_query = &query[..query.len() - 7]; // Remove "-DEBIAN" suffix
        return QueryType::Debian(base_query.to_string());
    }
    
    // Check if it's a Wikipedia article query
    if query.to_uppercase().ends_with("-WIKIPEDIA") {
        let base_query = &query[..query.len() - 10]; // Remove "-WIKIPEDIA" suffix
        return QueryType::Wikipedia(base_query.to_string());
    }
    
    // Check if it's a BGP Tools query
    if query.to_uppercase().ends_with("-BGPTOOL") {
        let base_query = &query[..query.len() - 8]; // Remove "-BGPTOOL" suffix
        return QueryType::BGPTool(base_query.to_string());
    }
    
    // Check if it's a prefixes query
    if query.to_uppercase().ends_with("-PREFIXES") {
        let base_query = &query[..query.len() - 9]; // Remove "-PREFIXES" suffix
        return QueryType::Prefixes(base_query.to_string());
    }
    
    // Check if it's a RIR geo query
    if query.to_uppercase().ends_with("-RIRGEO") {
        let base_query = &query[..query.len() - 7]; // Remove "-RIRGEO" suffix
        return QueryType::RirGeo(base_query.to_string());
    }
    
    // Check if it's a geo query
    if query.to_uppercase().ends_with("-GEO") {
        let base_query = &query[..query.len() - 4]; // Remove "-GEO" suffix
        return QueryType::Geo(base_query.to_string());
    }
    
    // Check if it's an email search query
    if query.to_uppercase().ends_with("-EMAIL") {
        let base_query = &query[..query.len() - 6]; // Remove "-EMAIL" suffix
        return QueryType::EmailSearch(base_query.to_string());
    }
    
    // Check if it's a .dn42 domain
    if query.to_lowercase().ends_with(".dn42") {
        return QueryType::Domain(query.to_string());
    }
    
    // Check if it has -DN42 suffix or ends with -MNT
    if query.to_uppercase().ends_with("-DN42") || query.to_uppercase().ends_with("-MNT") {
        return QueryType::Unknown(query.to_string());
    }
    
    // Try to parse as IP address
    if let Ok(ip) = query.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => return QueryType::IPv4(ipv4),
            IpAddr::V6(ipv6) => return QueryType::IPv6(ipv6),
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
    if query.to_uppercase().starts_with("AS") && query[2..].chars().all(|c| c.is_digit(10)) {
        return QueryType::ASN(query.to_string());
    }
    
    // Check if it's a domain format
    let domain_regex = Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$").unwrap();
    if domain_regex.is_match(query) {
        return QueryType::Domain(query.to_string());
    }
    
    // Default to unknown type
    QueryType::Unknown(query.to_string())
}

pub fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    for range_str in PRIVATE_IPV4_RANGES {
        if let Ok(range) = range_str.parse::<Ipv4Cidr>() {
            if range.contains(&ip) {
                return true;
            }
        }
    }
    false
}

pub fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    for range_str in PRIVATE_IPV6_RANGES {
        if let Ok(range) = range_str.parse::<Ipv6Cidr>() {
            if range.contains(&ip) {
                return true;
            }
        }
    }
    false
} 