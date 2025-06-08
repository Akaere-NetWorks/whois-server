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
    Unknown(String),
}

pub fn analyze_query(query: &str) -> QueryType {
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