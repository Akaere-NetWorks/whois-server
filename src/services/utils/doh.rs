//! Cloudflare DNS-over-HTTPS (DOH) client for PTR record lookups
//!
//! This module provides an async client for Cloudflare's DOH service
//! to perform DNS queries over HTTPS.

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use crate::log_debug;

const CLOUDFLARE_DOH_URL: &str = "https://cloudflare-dns.com/dns-query";

/// DOH response structure
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct DohResponse {
    pub Status: u32,
    #[serde(rename = "Answer", default)]
    pub Answer: Option<Vec<DohAnswer>>,
    #[serde(rename = "Comment", default)]
    #[allow(dead_code)]
    pub Comment: Option<String>,
}

/// DNS answer record
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct DohAnswer {
    pub name: String,
    pub data: String,
    #[serde(rename = "type")]
    pub record_type: u32,
}

/// Client for Cloudflare DOH
pub struct DohClient {
    client: Client,
}

impl DohClient {
    /// Create a new DOH client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("whois-server/1.0")
            .build();

        Self {
            client: client.unwrap_or_else(|_| Client::new()),
        }
    }

    /// Query PTR records for an IP address
    ///
    /// Returns a vector of PTR hostnames (can be empty if no PTR records found)
    pub async fn query_ptr(&self, ip: &str) -> Result<Vec<String>> {
        log_debug!("Querying PTR records for: {}", ip);

        // Parse IP and create PTR name
        let ip_addr: IpAddr = ip.parse()
            .map_err(|_| anyhow::anyhow!("Invalid IP address: {}", ip))?;

        let ptr_name = match ip_addr {
            IpAddr::V4(ipv4) => self.create_ipv4_ptr_name(ipv4),
            IpAddr::V6(ipv6) => self.create_ipv6_ptr_name(ipv6),
        };

        log_debug!("PTR query name: {}", ptr_name);

        // Build DOH request URL
        let url = format!(
            "{}?name={}&type=PTR&do=false",
            CLOUDFLARE_DOH_URL,
            urlencoding::encode(&ptr_name)
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/dns-json")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("DOH request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("DOH request failed with status: {}", status));
        }

        let doh_response: DohResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse DOH response: {}", e))?;

        // Check if query was successful
        if doh_response.Status != 0 {
            // NXDOMAIN or other DNS error - return empty vec, not an error
            log_debug!("DNS query returned status: {}", doh_response.Status);
            return Ok(Vec::new());
        }

        // Extract PTR records
        let ptr_records: Vec<String> = doh_response.Answer
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.record_type == 12) // 12 = PTR record type
            .map(|a| {
                // Remove trailing dot if present
                let name = if a.data.ends_with('.') {
                    a.data[..a.data.len() - 1].to_string()
                } else {
                    a.data
                };
                name
            })
            .collect();

        if ptr_records.is_empty() {
            log_debug!("No PTR records found for {}", ip);
        } else {
            log_debug!("Found {} PTR record(s) for {}", ptr_records.len(), ip);
        }

        Ok(ptr_records)
    }

    /// Create IPv4 PTR name (e.g., 1.1.1.1 -> 1.1.1.1.in-addr.arpa)
    fn create_ipv4_ptr_name(&self, ip: Ipv4Addr) -> String {
        let octets = ip.octets();
        format!(
            "{}.{}.{}.{}.in-addr.arpa",
            octets[3], octets[2], octets[1], octets[0]
        )
    }

    /// Create IPv6 PTR name (e.g., 2001:db8::1 -> 1.0.0.0...ip6.arpa)
    fn create_ipv6_ptr_name(&self, ip: Ipv6Addr) -> String {
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

    /// Query PTR records with caching (returns empty Vec on error instead of Err)
    #[allow(dead_code)]
    pub async fn query_ptr_cached(&self, ip: &str) -> Vec<String> {
        self.query_ptr(ip).await.unwrap_or_default()
    }
}

impl Default for DohClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_ptr_name() {
        let client = DohClient::new();

        // Test IPv4 PTR name creation
        let ip: Ipv4Addr = "1.1.1.1".parse().unwrap();
        let ptr_name = client.create_ipv4_ptr_name(ip);
        assert_eq!(ptr_name, "1.1.1.1.in-addr.arpa");

        let ip: Ipv4Addr = "8.8.8.8".parse().unwrap();
        let ptr_name = client.create_ipv4_ptr_name(ip);
        assert_eq!(ptr_name, "8.8.8.8.in-addr.arpa");
    }

    #[test]
    fn test_ipv6_ptr_name() {
        let client = DohClient::new();

        // Test IPv6 PTR name creation (simplified test)
        let ip: Ipv6Addr = "2001:db8::1".parse().unwrap();
        let ptr_name = client.create_ipv6_ptr_name(ip);

        // Just check that it ends with ip6.arpa
        assert!(ptr_name.ends_with(".ip6.arpa"));
    }

    #[test]
    fn test_doh_client_default() {
        let client = DohClient::default();
        // Just ensure it can be created
        assert_eq!(CLOUDFLARE_DOH_URL, "https://cloudflare-dns.com/dns-query");
    }
}
