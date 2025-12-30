//! IPInfo API client for ASN and geolocation data
//!
//! This module provides an async client for the IPInfo API (https://ipinfo.io)
//! which returns ASN, organization, and geolocation information for IP addresses.

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use crate::{log_debug, log_error};

const IPINFO_API_BASE: &str = "https://api.ipinfo.io/lite";

/// IP information from ipinfo.io
#[derive(Debug, Deserialize, Clone)]
pub struct IpInfo {
    pub ip: String,
    pub asn: String,
    #[serde(rename = "as_name")]
    pub as_name: String,
    #[serde(rename = "as_domain")]
    pub as_domain: String,
    #[serde(rename = "country_code")]
    pub country_code: String,
    pub country: String,
    #[serde(rename = "continent_code")]
    pub continent_code: String,
    pub continent: String,
}

impl IpInfo {
    /// Check if this is private/RFC1918 address space
    pub fn is_private(&self) -> bool {
        self.asn == "*" || self.asn.is_empty()
    }

    /// Get a formatted description string
    pub fn description(&self) -> String {
        if self.is_private() {
            "RFC1918".to_string()
        } else {
            format!("{} [{}]", self.as_name, self.asn)
        }
    }
}

/// Client for IPInfo API
pub struct IpInfoClient {
    client: Client,
    api_token: String,
}

impl IpInfoClient {
    /// Create a new IPInfo client
    ///
    /// Loads API token from IPINFO_API_TOKEN environment variable
    pub fn new() -> Result<Self> {
        let api_token = std::env::var("IPINFO_API_TOKEN")
            .map_err(|_| anyhow::anyhow!("IPINFO_API_TOKEN environment variable not set"))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("whois-server/1.0")
            .build()?;

        log_debug!("IPInfo client initialized");

        Ok(Self { client, api_token })
    }

    /// Get IP information for a given IP address
    pub async fn get_ip_info(&self, ip: &str) -> Result<IpInfo> {
        log_debug!("Fetching IP info for: {}", ip);

        let url = format!("{}/{}?token={}", IPINFO_API_BASE, ip, self.api_token);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch IP info: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            log_error!("IPInfo API error: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("IPInfo API returned error: {} - {}", status, error_text));
        }

        let info: IpInfo = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse IPInfo response: {}", e))?;

        log_debug!("Got IP info: {} -> {}", ip, info.as_name);
        Ok(info)
    }

    /// Get IP information with caching (returns None on error instead of Err)
    pub async fn get_ip_info_cached(&self, ip: &str) -> Option<IpInfo> {
        self.get_ip_info(ip).await.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_info_description() {
        let info = IpInfo {
            ip: "1.1.1.1".to_string(),
            asn: "AS13335".to_string(),
            as_name: "Cloudflare, Inc.".to_string(),
            as_domain: "cloudflare.com".to_string(),
            country_code: "US".to_string(),
            country: "United States".to_string(),
            continent_code: "NA".to_string(),
            continent: "North America".to_string(),
        };

        assert_eq!(info.description(), "Cloudflare, Inc. [AS13335]");
        assert!(!info.is_private());
    }

    #[test]
    fn test_private_ip_detection() {
        let private = IpInfo {
            ip: "192.168.1.1".to_string(),
            asn: "*".to_string(),
            as_name: String::new(),
            as_domain: String::new(),
            country_code: String::new(),
            country: String::new(),
            continent_code: String::new(),
            continent: String::new(),
        };

        assert!(private.is_private());
        assert_eq!(private.description(), "RFC1918");
    }
}
