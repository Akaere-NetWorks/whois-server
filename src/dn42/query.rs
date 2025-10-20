use std::net::{ Ipv4Addr, Ipv6Addr };
// Removed unused imports

/// Parse ASN from query, handling various formats for DN42
pub fn parse_asn(query: &str) -> Option<String> {
    let normalized = query.to_uppercase();

    // Handle short ASN formats (1-4 digits) - convert to full DN42 format
    if let Ok(num) = normalized.parse::<u32>() {
        return match num.to_string().len() {
            1 => Some(format!("AS424242000{}", num)),
            2 => Some(format!("AS42424200{}", num)),
            3 => Some(format!("AS4242420{}", num)),
            4 => Some(format!("AS424242{}", num)),
            _ => Some(format!("AS{}", num)),
        };
    }

    // Handle AS prefix
    if let Some(asn_part) = normalized.strip_prefix("AS")
        && let Ok(num) = asn_part.parse::<u32>() {
            return match asn_part.len() {
                1 => Some(format!("AS424242000{}", num)),
                2 => Some(format!("AS42424200{}", num)),
                3 => Some(format!("AS4242420{}", num)),
                4 => Some(format!("AS424242{}", num)),
                _ => Some(normalized),
            };
        }

    None
}

/// Detect query type and parse parameters
#[derive(Debug, Clone)]
pub enum DN42QueryType {
    IPv4Network {
        ip: Ipv4Addr,
        mask: u8,
    },
    IPv6Network {
        ip: Ipv6Addr,
        mask: u8,
    },
    ASN {
        asn: String,
    },
    Person {
        handle: String,
    },
    Maintainer {
        handle: String,
    },
    Schema {
        handle: String,
    },
    Organisation {
        handle: String,
    },
    TincKeyset {
        handle: String,
    },
    TincKey {
        handle: String,
    },
    RouteSet {
        handle: String,
    },
    AsBlock {
        handle: String,
    },
    AsSet {
        handle: String,
    },
    DNS {
        domain: String,
    },
    #[allow(dead_code)]
    Unknown,
}

impl DN42QueryType {
    /// Parse query string and determine type
    pub fn parse(query: &str) -> Self {
        let normalized_query = query.to_uppercase();

        // Try to parse as IP address with CIDR
        if let Some((ip_str, mask_str)) = query.split_once('/') {
            if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>())
                && mask <= 32 {
                    return DN42QueryType::IPv4Network { ip: ipv4, mask };
                }

            if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>())
                && mask <= 128 {
                    return DN42QueryType::IPv6Network { ip: ipv6, mask };
                }
        }

        // Try to parse as single IP address (assume /32 for IPv4, /128 for IPv6)
        if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
            return DN42QueryType::IPv4Network { ip: ipv4, mask: 32 };
        }

        if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
            return DN42QueryType::IPv6Network { ip: ipv6, mask: 128 };
        }

        // Handle ASN queries
        if let Some(asn) = parse_asn(&normalized_query) {
            return DN42QueryType::ASN { asn };
        }

        // Handle person objects (-DN42 suffix)
        if normalized_query.ends_with("-DN42") {
            return DN42QueryType::Person { handle: normalized_query };
        }

        // Handle maintainer objects (-MNT suffix)
        if normalized_query.ends_with("-MNT") {
            return DN42QueryType::Maintainer { handle: normalized_query };
        }

        // Handle schema objects (-SCHEMA suffix)
        if normalized_query.ends_with("-SCHEMA") {
            return DN42QueryType::Schema { handle: normalized_query };
        }

        // Handle organisation objects (ORG- prefix)
        if normalized_query.starts_with("ORG-") {
            return DN42QueryType::Organisation { handle: normalized_query };
        }

        // Handle tinc-keyset objects (SET-*-TINC pattern)
        if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
            return DN42QueryType::TincKeyset { handle: normalized_query };
        }

        // Handle tinc-key objects (-TINC suffix)
        if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
            return DN42QueryType::TincKey { handle: normalized_query };
        }

        // Handle route-set objects (RS- prefix)
        if normalized_query.starts_with("RS-") {
            return DN42QueryType::RouteSet { handle: normalized_query };
        }

        // Handle as-block objects (AS*-AS* pattern)
        if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
            return DN42QueryType::AsBlock { handle: normalized_query };
        }

        // Handle as-set objects (AS prefix, not an ASN)
        if
            normalized_query.starts_with("AS") &&
            !normalized_query
                .chars()
                .skip(2)
                .all(|c| c.is_ascii_digit())
        {
            return DN42QueryType::AsSet { handle: normalized_query };
        }

        // Handle DNS objects (default fallback)
        DN42QueryType::DNS { domain: query.to_lowercase() }
    }

    /// Get the object type for file path construction
    pub fn get_object_type(&self) -> &'static str {
        match self {
            DN42QueryType::IPv4Network { .. } => "inetnum",
            DN42QueryType::IPv6Network { .. } => "inet6num",
            DN42QueryType::ASN { .. } => "aut-num",
            DN42QueryType::Person { .. } => "person",
            DN42QueryType::Maintainer { .. } => "mntner",
            DN42QueryType::Schema { .. } => "schema",
            DN42QueryType::Organisation { .. } => "organisation",
            DN42QueryType::TincKeyset { .. } => "tinc-keyset",
            DN42QueryType::TincKey { .. } => "tinc-key",
            DN42QueryType::RouteSet { .. } => "route-set",
            DN42QueryType::AsBlock { .. } => "as-block",
            DN42QueryType::AsSet { .. } => "as-set",
            DN42QueryType::DNS { .. } => "dns",
            DN42QueryType::Unknown => "unknown",
        }
    }

    /// Get the file name for this query
    pub fn get_file_name(&self) -> String {
        match self {
            DN42QueryType::IPv4Network { ip, mask } => format!("{},{}", ip, mask),
            DN42QueryType::IPv6Network { ip, mask } => format!("{},{}", ip, mask),
            DN42QueryType::ASN { asn } => asn.clone(),
            DN42QueryType::Person { handle } => handle.clone(),
            DN42QueryType::Maintainer { handle } => handle.clone(),
            DN42QueryType::Schema { handle } => handle.clone(),
            DN42QueryType::Organisation { handle } => handle.clone(),
            DN42QueryType::TincKeyset { handle } => handle.clone(),
            DN42QueryType::TincKey { handle } => handle.clone(),
            DN42QueryType::RouteSet { handle } => handle.clone(),
            DN42QueryType::AsBlock { handle } => handle.clone(),
            DN42QueryType::AsSet { handle } => handle.clone(),
            DN42QueryType::DNS { domain } => domain.clone(),
            DN42QueryType::Unknown => "unknown".to_string(),
        }
    }
}

/// Format DN42 query response
pub fn format_query_response(query: &str, content: Option<String>) -> String {
    let mut response = String::new();
    response.push_str(&format!("% Query: {}\n", query));

    if let Some(data) = content {
        response.push_str(&data);
    } else {
        response.push_str("% 404 Not Found\n");
    }

    response
}

/// Format DN42 IPv4 network response with both inetnum and route data
pub fn format_ipv4_network_response(
    query: &str,
    inetnum_content: Option<String>,
    route_content: Option<String>
) -> String {
    let mut response = String::new();
    response.push_str(&format!("% Query: {}\n", query));

    // Add inetnum data
    if let Some(data) = inetnum_content {
        response.push_str(&data);
    } else {
        response.push_str("% 404 - inetnum not found\n");
    }

    response.push_str("% Relevant route object:\n");

    // Add route data
    if let Some(data) = route_content {
        response.push_str(&data);
    } else {
        response.push_str("% 404 - route not found\n");
    }

    response
}

/// Format DN42 IPv6 network response with both inet6num and route6 data
pub fn format_ipv6_network_response(
    query: &str,
    inet6num_content: Option<String>,
    route6_content: Option<String>
) -> String {
    let mut response = String::new();
    response.push_str(&format!("% Query: {}\n", query));

    // Add inet6num data
    if let Some(data) = inet6num_content {
        response.push_str(&data);
    } else {
        response.push_str("% 404 - inet6num not found\n");
    }

    response.push_str("% Relevant route object:\n");

    // Add route6 data
    if let Some(data) = route6_content {
        response.push_str(&data);
    } else {
        response.push_str("% 404 - route6 not found\n");
    }

    response
}
