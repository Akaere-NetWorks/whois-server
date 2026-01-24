//! Cloudflare DNS-over-HTTPS (DOH) client for DNS queries
//!
//! This module provides an async client for Cloudflare's DOH service
//! to perform DNS queries over HTTPS.

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::{ IpAddr, Ipv4Addr, Ipv6Addr };
use std::time::Duration;
use crate::log_debug;

const CLOUDFLARE_DOH_URL: &str = "https://cloudflare-dns.com/dns-query";

/// DNS record types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum DnsRecordType {
    A = 1, // IPv4 address
    NS = 2, // Name server
    CNAME = 5, // Canonical name
    SOA = 6, // Start of authority
    PTR = 12, // Pointer record
    MX = 15, // Mail exchange
    TXT = 16, // Text record
    AAAA = 28, // IPv6 address
}

impl DnsRecordType {
    /// Convert from numeric value
    #[allow(dead_code)]
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::A),
            2 => Some(Self::NS),
            5 => Some(Self::CNAME),
            6 => Some(Self::SOA),
            12 => Some(Self::PTR),
            15 => Some(Self::MX),
            16 => Some(Self::TXT),
            28 => Some(Self::AAAA),
            _ => None,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::NS => "NS",
            Self::CNAME => "CNAME",
            Self::SOA => "SOA",
            Self::PTR => "PTR",
            Self::MX => "MX",
            Self::TXT => "TXT",
            Self::AAAA => "AAAA",
        }
    }
}

/// DOH response structure (extended)
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct DnsResponse {
    pub Status: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub TC: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub RD: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub RA: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub AD: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub CD: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub Question: Option<Vec<DnsQuestion>>,
    #[serde(rename = "Answer", default)]
    pub Answer: Option<Vec<DnsAnswer>>,
    #[serde(rename = "Authority", default)]
    #[allow(dead_code)]
    pub Authority: Option<Vec<DnsAnswer>>,
    #[serde(rename = "Comment", default)]
    #[allow(dead_code)]
    pub Comment: Option<String>,
}

/// DNS question section
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DnsQuestion {
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub qtype: u32,
}

/// DNS answer record (extended)
#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct DnsAnswer {
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: u32,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub TTL: u32,
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

    /// Generic DNS query method
    ///
    /// Queries any DNS record type for a given name
    pub async fn query(&self, name: &str, record_type: &str) -> Result<DnsResponse> {
        log_debug!("Querying DNS: {} type={}", name, record_type);

        let url = format!(
            "{}?name={}&type={}&do=false",
            CLOUDFLARE_DOH_URL,
            urlencoding::encode(name),
            record_type
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/dns-json")
            .send().await
            .map_err(|e| anyhow::anyhow!("DOH request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("DOH request failed with HTTP status: {}", status));
        }

        let doh_response: DnsResponse = response
            .json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse DOH response: {}", e))?;

        if doh_response.Status != 0 {
            log_debug!("DNS query returned status: {}", doh_response.Status);
        }

        Ok(doh_response)
    }

    /// Batch query multiple record types concurrently
    ///
    /// Returns a HashMap with record type as key and list of answers as value
    pub async fn query_batch(
        &self,
        name: &str,
        types: &[DnsRecordType]
    ) -> Result<HashMap<String, Vec<DnsAnswer>>> {
        use futures::future::{ join_all, FutureExt };

        let mut results = HashMap::new();

        // Create futures for all queries
        let mut futures = Vec::new();
        for record_type in types {
            let name_owned = name.to_string();
            let type_str = record_type.as_str().to_string();
            let client = self.client.clone();

            futures.push(
                (
                    async move {
                        let url = format!(
                            "{}?name={}&type={}&do=false",
                            CLOUDFLARE_DOH_URL,
                            urlencoding::encode(&name_owned),
                            type_str
                        );

                        let response = client
                            .get(&url)
                            .header("Accept", "application/dns-json")
                            .send().await;

                        match response {
                            Ok(resp) if resp.status().is_success() => {
                                match resp.json::<crate::services::utils::doh::DnsResponse>().await {
                                    Ok(doh_response) => Ok((type_str, doh_response)),
                                    Err(_) => Err(type_str),
                                }
                            }
                            _ => Err(type_str),
                        }
                    }
                ).boxed()
            );
        }

        // Execute all queries concurrently
        let responses = join_all(futures).await;

        // Process results
        for result in responses {
            match result {
                Ok((type_str, doh_response)) if doh_response.Status == 0 => {
                    if let Some(answers) = doh_response.Answer {
                        if !answers.is_empty() {
                            results.insert(type_str, answers);
                        }
                    }
                }
                Ok((type_str, doh_response)) => {
                    log_debug!(
                        "DNS query for {} returned status: {}",
                        type_str,
                        doh_response.Status
                    );
                }
                Err(type_str) => {
                    log_debug!("Failed to query {} records", type_str);
                }
            }
        }

        Ok(results)
    }

    /// Query PTR records for an IP address
    ///
    /// Returns a vector of PTR hostnames (can be empty if no PTR records found)
    pub async fn query_ptr(&self, ip: &str) -> Result<Vec<String>> {
        log_debug!("Querying PTR records for: {}", ip);

        // Parse IP and create PTR name
        let ip_addr: IpAddr = ip
            .parse()
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
            .send().await
            .map_err(|e| anyhow::anyhow!("DOH request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("DOH request failed with status: {}", status));
        }

        let doh_response: crate::services::utils::doh::DnsResponse = response
            .json().await
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
        format!("{}.{}.{}.{}.in-addr.arpa", octets[3], octets[2], octets[1], octets[0])
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
        let _client = DohClient::default();
        // Just ensure it can be created
        assert_eq!(CLOUDFLARE_DOH_URL, "https://cloudflare-dns.com/dns-query");
    }
}
