use std::time::Duration;
use anyhow::Result;
use tracing::{debug, warn, error};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};

/// Certificate entry from crt.sh API
#[derive(Debug, Deserialize, Serialize)]
struct CrtEntry {
    issuer_ca_id: u64,
    issuer_name: String,
    common_name: Option<String>,
    name_value: String,
    id: u64,
    entry_timestamp: String,
    not_before: String,
    not_after: String,
    serial_number: String,
}

/// Processed certificate information for display
#[derive(Debug, Clone)]
struct CertificateEntry {
    id: u64,
    common_name: String,
    subject_alt_names: Vec<String>,
    issuer: String,
    serial_number: String,
    not_before: String,
    not_after: String,
    entry_timestamp: String,
    #[allow(dead_code)]
    is_valid: bool,
}

/// Certificate Transparency service for querying crt.sh
pub struct CrtService {
    client: reqwest::Client,
    timeout: Duration,
}

impl Default for CrtService {
    fn default() -> Self {
        Self::new()
    }
}

impl CrtService {
    /// Create a new CRT service with default 20-second timeout
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 (WHOIS Server; Certificate Transparency Lookup)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            timeout: Duration::from_secs(20),
        }
    }

    /// Create CRT service with custom timeout (max 20 seconds for stability)
    #[allow(dead_code)]
    pub fn with_timeout(timeout: Duration) -> Self {
        let timeout = std::cmp::min(timeout, Duration::from_secs(20));
        
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("Mozilla/5.0 (WHOIS Server; Certificate Transparency Lookup)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, timeout }
    }

    /// Query crt.sh for certificate transparency logs
    pub async fn query_crt(&self, domain: &str) -> Result<String> {
        debug!("Querying Certificate Transparency logs for domain: {}", domain);

        match self.fetch_certificates(domain).await {
            Ok(certificates) => {
                let valid_certs = self.filter_valid_certificates(certificates);
                let output = self.format_certificates(&valid_certs, domain);
                debug!("CRT query completed for {}, found {} valid certificates", domain, valid_certs.len());
                Ok(output)
            }
            Err(e) => {
                error!("Failed to fetch certificates for {}: {}", domain, e);
                Ok(format!(
                    "Certificate Transparency Query Failed for {}\nError: {}\n\nNote: crt.sh API is known to be unstable and may timeout frequently.\nPlease try again or use alternative certificate lookup methods.\n",
                    domain, e
                ))
            }
        }
    }

    /// Fetch certificates from crt.sh API
    async fn fetch_certificates(&self, domain: &str) -> Result<Vec<CrtEntry>> {
        let url = format!("https://crt.sh/json?q={}", urlencoding::encode(domain));
        debug!("Fetching certificates from URL: {}", url);

        // Set a strict timeout to prevent hanging
        let response = tokio::time::timeout(
            self.timeout,
            self.client.get(&url).send()
        ).await
        .map_err(|_| anyhow::anyhow!("Request timeout after {} seconds - crt.sh API is unresponsive", self.timeout.as_secs()))?
        .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {} - {}", 
                response.status(), 
                response.status().canonical_reason().unwrap_or("Unknown error")
            ));
        }

        // Parse JSON response with timeout
        let json_text = tokio::time::timeout(
            Duration::from_secs(10),
            response.text()
        ).await
        .map_err(|_| anyhow::anyhow!("Response parsing timeout - crt.sh returned too much data"))?
        .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        if json_text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty response from crt.sh - domain may not exist or have no certificates"));
        }

        let certificates: Vec<CrtEntry> = serde_json::from_str(&json_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON response: {}", e))?;

        debug!("Successfully fetched {} certificate entries", certificates.len());
        Ok(certificates)
    }

    /// Filter certificates to only include currently valid ones
    fn filter_valid_certificates(&self, certificates: Vec<CrtEntry>) -> Vec<CertificateEntry> {
        let now = Utc::now();
        let mut valid_certs = Vec::new();

        for cert in certificates {
            // Parse the not_before and not_after dates
            let not_before = match self.parse_crt_date(&cert.not_before) {
                Ok(date) => date,
                Err(e) => {
                    warn!("Failed to parse not_before date '{}': {}", cert.not_before, e);
                    continue;
                }
            };

            let not_after = match self.parse_crt_date(&cert.not_after) {
                Ok(date) => date,
                Err(e) => {
                    warn!("Failed to parse not_after date '{}': {}", cert.not_after, e);
                    continue;
                }
            };

            // Check if certificate is currently valid
            let is_valid = now >= not_before && now <= not_after;

            // Skip expired certificates
            if !is_valid {
                continue;
            }

            // Parse Subject Alternative Names from name_value field
            let mut subject_alt_names: Vec<String> = cert.name_value
                .split('\n')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            // Remove duplicates
            subject_alt_names.sort();
            subject_alt_names.dedup();

            let processed_cert = CertificateEntry {
                id: cert.id,
                common_name: cert.common_name.unwrap_or_else(|| {
                    subject_alt_names.first().unwrap_or(&"Unknown".to_string()).clone()
                }),
                subject_alt_names,
                issuer: cert.issuer_name,
                serial_number: cert.serial_number,
                not_before: self.format_date_display(&not_before),
                not_after: self.format_date_display(&not_after),
                entry_timestamp: cert.entry_timestamp,
                is_valid,
            };

            valid_certs.push(processed_cert);
        }

        // Sort by not_after date (most recent expiration first)
        valid_certs.sort_by(|a, b| b.not_after.cmp(&a.not_after));

        // Remove duplicates based on serial number and issuer
        let mut unique_certs = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for cert in valid_certs {
            let key = format!("{}:{}", cert.serial_number, cert.issuer);
            if seen.insert(key) {
                unique_certs.push(cert);
            }
        }

        unique_certs
    }

    /// Parse crt.sh date format (ISO 8601)
    fn parse_crt_date(&self, date_str: &str) -> Result<DateTime<Utc>> {
        // Try parsing with different formats that crt.sh might use
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.3fZ",  // With milliseconds
            "%Y-%m-%dT%H:%M:%SZ",      // Without milliseconds
            "%Y-%m-%dT%H:%M:%S%.f",    // With fractional seconds, no Z
            "%Y-%m-%d %H:%M:%S",       // Space separator
        ];

        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        // If none of the formats work, try parsing as ISO 8601
        date_str.parse::<DateTime<Utc>>()
            .map_err(|e| anyhow::anyhow!("Unable to parse date '{}': {}", date_str, e))
    }

    /// Format date for display
    fn format_date_display(&self, date: &DateTime<Utc>) -> String {
        format!("{} ({})", 
            date.format("%Y-%m-%d %H:%M:%S UTC"), 
            date.timestamp()
        )
    }

    /// Format certificates for display
    fn format_certificates(&self, certificates: &[CertificateEntry], domain: &str) -> String {
        if certificates.is_empty() {
            return format!(
                "Certificate Transparency Query Results for: {}\n\nNo valid (non-expired) certificates found in Certificate Transparency logs.\nThis could mean:\n- Domain has no certificates\n- All certificates are expired\n- Domain is not publicly accessible\n- crt.sh may not have indexed this domain yet\n",
                domain
            );
        }

        let mut output = String::new();
        output.push_str(&format!("Certificate Transparency Query Results for: {}\n", domain));
        output.push_str(&format!("Found {} valid (non-expired) certificates from CT logs\n", certificates.len()));
        output.push_str("=" .repeat(80).as_str());
        output.push('\n');

        for (index, cert) in certificates.iter().enumerate() {
            output.push_str(&format!("\n[{}] Certificate #{}\n", index + 1, cert.id));
            output.push_str(&format!("Common Name: {}\n", cert.common_name));
            
            if cert.subject_alt_names.len() > 1 || 
               (cert.subject_alt_names.len() == 1 && cert.subject_alt_names[0] != cert.common_name) {
                output.push_str("Subject Alternative Names:\n");
                for san in &cert.subject_alt_names {
                    output.push_str(&format!("  - {}\n", san));
                }
            }

            output.push_str(&format!("Issuer: {}\n", cert.issuer));
            output.push_str(&format!("Serial Number: {}\n", cert.serial_number));
            output.push_str(&format!("Valid From: {}\n", cert.not_before));
            output.push_str(&format!("Valid Until: {}\n", cert.not_after));
            output.push_str(&format!("CT Log Entry: {}\n", cert.entry_timestamp));
            
            if index < certificates.len() - 1 {
                output.push_str("-" .repeat(40).as_str());
                output.push('\n');
            }
        }

        output.push('\n');
        output.push_str("Note: Data sourced from Certificate Transparency logs via crt.sh\n");
        output.push_str("Only currently valid (non-expired) certificates are shown\n");

        output
    }

    /// Check if a query string is a CRT query
    pub fn is_crt_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-CRT")
    }

    /// Parse CRT query to extract domain
    pub fn parse_crt_query(query: &str) -> Option<String> {
        if !Self::is_crt_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 4]; // Remove "-CRT"
        Some(clean_query.to_string())
    }
}

/// Process Certificate Transparency query with -CRT suffix
pub async fn process_crt_query(query: &str) -> Result<String> {
    let crt_service = CrtService::new();
    
    if let Some(domain) = CrtService::parse_crt_query(query) {
        debug!("Processing CRT query for domain: {}", domain);
        return crt_service.query_crt(&domain).await;
    }
    
    error!("Invalid CRT query format: {}", query);
    Ok(format!(
        "Invalid Certificate Transparency query format. Use: domain-CRT\nQuery: {}\nExample: example.com-CRT\n",
        query
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crt_query_detection() {
        assert!(CrtService::is_crt_query("example.com-CRT"));
        assert!(CrtService::is_crt_query("example.com-crt"));
        assert!(CrtService::is_crt_query("sub.example.com-CRT"));
        
        assert!(!CrtService::is_crt_query("example.com"));
        assert!(!CrtService::is_crt_query("example.com-SSL"));
        assert!(!CrtService::is_crt_query("CRT-example.com"));
    }

    #[test]
    fn test_crt_query_parsing() {
        assert_eq!(
            CrtService::parse_crt_query("example.com-CRT"),
            Some("example.com".to_string())
        );
        
        assert_eq!(
            CrtService::parse_crt_query("sub.domain.com-CRT"),
            Some("sub.domain.com".to_string())
        );
        
        assert_eq!(CrtService::parse_crt_query("example.com"), None);
    }

    #[tokio::test]
    async fn test_crt_service_creation() {
        let service = CrtService::new();
        assert_eq!(service.timeout, Duration::from_secs(20));
        
        let custom_service = CrtService::with_timeout(Duration::from_secs(15));
        assert_eq!(custom_service.timeout, Duration::from_secs(15));
        
        // Test that timeout is capped at 20 seconds
        let capped_service = CrtService::with_timeout(Duration::from_secs(30));
        assert_eq!(capped_service.timeout, Duration::from_secs(20));
    }
}