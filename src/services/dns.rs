//! DNS query handler using Cloudflare DOH API
//!
//! This module provides DNS functionality using Cloudflare's DNS-over-HTTPS API
//! with support for multiple record types: A, AAAA, CNAME, MX, TXT, NS, SOA, PTR

use anyhow::Result;
use std::net::IpAddr;
use crate::services::utils::doh::{DohClient, DnsRecordType, DnsAnswer};
use crate::{log_debug, log_error};

/// DNS service using Cloudflare DOH API
pub struct DnsService {
    client: DohClient,
}

impl DnsService {
    /// Create a new DNS service
    pub fn new() -> Self {
        Self {
            client: DohClient::new(),
        }
    }

    /// Query all DNS records for a domain
    pub async fn query_dns(&self, domain: &str) -> Result<String> {
        log_debug!("Querying DNS records for domain: {}", domain);

        let record_types = vec![
            DnsRecordType::A,
            DnsRecordType::AAAA,
            DnsRecordType::CNAME,
            DnsRecordType::MX,
            DnsRecordType::TXT,
            DnsRecordType::NS,
            DnsRecordType::SOA,
        ];

        let results = self.client.query_batch(domain, &record_types).await?;

        if results.is_empty() {
            return Ok(format!("No DNS records found for domain: {}\n", domain));
        }

        let mut output = format!("DNS Records for {}:\n", domain);

        // Output records in order
        for record_type in &record_types {
            let type_str = record_type.as_str();
            if let Some(answers) = results.get(type_str) {
                if !answers.is_empty() {
                    output.push_str(&format!("\n{} Records:\n", type_str));
                    for answer in answers {
                        output.push_str(&self.format_doh_answer(answer, type_str));
                    }
                }
            }
        }

        Ok(output)
    }

    /// Reverse DNS lookup (PTR records)
    pub async fn query_rdns(&self, ip: IpAddr) -> Result<String> {
        log_debug!("Querying reverse DNS for IP: {}", ip);

        // Create PTR query name
        let ptr_name = match ip {
            IpAddr::V4(ipv4) => self.create_ipv4_ptr_name(ipv4),
            IpAddr::V6(ipv6) => self.create_ipv6_ptr_name(ipv6),
        };

        log_debug!("PTR query name: {}", ptr_name);

        // Query PTR records
        match self.client.query(&ptr_name, "PTR").await {
            Ok(response) => {
                if response.Status != 0 {
                    return Ok(format!("No reverse DNS record found for IP: {}\n", ip));
                }

                let mut output = format!("Reverse DNS Results for {}:\n\nPTR Records:\n", ip);

                if let Some(answers) = response.Answer {
                    if answers.is_empty() {
                        return Ok(format!("No reverse DNS record found for IP: {}\n", ip));
                    }

                    for answer in answers {
                        // Remove trailing dot if present
                        let data = if answer.data.ends_with('.') {
                            &answer.data[..answer.data.len() - 1]
                        } else {
                            &answer.data
                        };

                        output.push_str(&format!("  {} (TTL: {})\n", data, answer.TTL));
                    }

                    Ok(output)
                } else {
                    Ok(format!("No reverse DNS record found for IP: {}\n", ip))
                }
            }
            Err(e) => {
                log_error!("Failed to query PTR records for {}: {}", ip, e);
                Ok(format!("Reverse DNS lookup failed for {}: {}\n", ip, e))
            }
        }
    }

    /// Format DOH answer records for display
    fn format_doh_answer(&self, answer: &DnsAnswer, record_type: &str) -> String {
        match record_type {
            "MX" => {
                // MX format: "10 mail.example.com"
                if let Some(space_pos) = answer.data.find(' ') {
                    let preference = &answer.data[..space_pos];
                    let exchange = &answer.data[space_pos + 1..];
                    format!("  {} {} (TTL: {})\n", preference, exchange, answer.TTL)
                } else {
                    format!("  {} (TTL: {})\n", answer.data, answer.TTL)
                }
            }
            "TXT" => {
                // TXT records may be quoted
                let data = if answer.data.starts_with('"') && answer.data.ends_with('"') {
                    &answer.data[1..answer.data.len() - 1]
                } else {
                    &answer.data
                };
                format!("  \"{}\" (TTL: {})\n", data, answer.TTL)
            }
            _ => {
                // A, AAAA, CNAME, NS, SOA, PTR
                format!("  {} (TTL: {})\n", answer.data, answer.TTL)
            }
        }
    }

    /// Create IPv4 PTR name (e.g., 1.1.1.1 -> 1.1.1.1.in-addr.arpa)
    fn create_ipv4_ptr_name(&self, ip: std::net::Ipv4Addr) -> String {
        let octets = ip.octets();
        format!(
            "{}.{}.{}.{}.in-addr.arpa",
            octets[3], octets[2], octets[1], octets[0]
        )
    }

    /// Create IPv6 PTR name
    fn create_ipv6_ptr_name(&self, ip: std::net::Ipv6Addr) -> String {
        let segments = ip.segments();
        let mut nibbles = Vec::new();

        for segment in segments.iter().rev() {
            let bytes = segment.to_be_bytes();
            for byte in bytes.iter().rev() {
                nibbles.push(format!("{:x}", byte & 0x0f));
                nibbles.push(format!("{:x}", (byte & 0xf0) >> 4));
            }
        }

        format!("{}.ip6.arpa", nibbles.join("."))
    }

    /// Check if query is a valid domain name
    pub fn is_domain_name(query: &str) -> bool {
        // Basic domain validation
        if query.is_empty() || query.len() > 253 {
            return false;
        }

        // Must contain at least one dot
        if !query.contains('.') {
            return false;
        }

        // Check if it's an IP address
        if query.parse::<IpAddr>().is_ok() {
            return false;
        }

        // Check for valid domain characters
        let parts: Vec<&str> = query.split('.').collect();
        if parts.len() < 2 {
            return false;
        }

        for part in parts {
            if part.is_empty() || part.len() > 63 {
                return false;
            }

            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return false;
            }

            if part.starts_with('-') || part.ends_with('-') {
                return false;
            }
        }

        true
    }

    /// Parse IP address
    pub fn parse_ip_address(query: &str) -> Option<IpAddr> {
        query.parse::<IpAddr>().ok()
    }
}

/// Process DNS query with -DNS suffix
pub async fn process_dns_query(query: &str) -> Result<String> {
    let dns_service = DnsService::new();

    // Remove -DNS suffix if present
    let clean_query = if query.to_uppercase().ends_with("-DNS") {
        &query[..query.len() - 4]
    } else {
        query
    };

    log_debug!("Processing DNS query for: {}", clean_query);

    // Check if it's an IP address (for rDNS)
    if let Some(ip) = DnsService::parse_ip_address(clean_query) {
        log_debug!("Detected IP address, performing reverse DNS lookup");
        return dns_service.query_rdns(ip).await;
    }

    // Check if it's a domain (for forward DNS)
    if DnsService::is_domain_name(clean_query) {
        log_debug!("Detected domain name, performing DNS lookup");
        return dns_service.query_dns(clean_query).await;
    }

    // Invalid format
    log_error!("Invalid DNS query format: {}", clean_query);
    Ok(format!(
        "Invalid DNS query format. Please provide a valid domain name or IP address.\nQuery: {}\n",
        clean_query
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_validation() {
        assert!(DnsService::is_domain_name("example.com"));
        assert!(DnsService::is_domain_name("sub.example.com"));
        assert!(!DnsService::is_domain_name("1.1.1.1"));
        assert!(!DnsService::is_domain_name("localhost"));
        assert!(!DnsService::is_domain_name(""));
    }

    #[test]
    fn test_ip_parsing() {
        assert!(DnsService::parse_ip_address("1.1.1.1").is_some());
        assert!(DnsService::parse_ip_address("2001:4860:4860::8888").is_some());
        assert!(DnsService::parse_ip_address("example.com").is_none());
    }
}
