// WHOIS Server - RDAP Backend (Experimental)
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! RDAP (Registration Data Access Protocol) backend for WHOIS queries.
//!
//! This module provides an alternative query backend that tries RDAP first,
//! falling back to legacy WHOIS on failure. This is **experimental** and must
//! be enabled via the `rdap-backend` feature flag.
//!
//! When `rdap-backend` is NOT enabled, all queries use the default WHOIS backend.

use crate::{log_debug, log_warn};
use anyhow::Result;
use rdap::whois::format_whois;
use rdap::{QueryType as RdapQueryType, RdapClient, RdapRequest};

/// Query using RDAP first, falling back to WHOIS if RDAP fails.
///
/// This implements IANA's recommendation to prefer RDAP over legacy WHOIS.
/// RDAP (RFC 7480-7484) provides structured, internationalized responses
/// with better access control and extensibility.
///
/// The fallback to WHOIS ensures backward compatibility for registries
/// that have not yet deployed RDAP services.
pub async fn query_rdap_with_whois_fallback(
    query: &str,
    rdap_query_type: RdapQueryType,
    whois_fallback: impl std::future::Future<Output = Result<String>>,
) -> Result<String> {
    log_debug!(
        "RDAP-first query for: {} (type: {:?})",
        query,
        rdap_query_type
    );

    // Try RDAP first
    match query_rdap_internal(query, rdap_query_type).await {
        Ok(response) => {
            log_debug!("RDAP query succeeded for: {}", query);
            Ok(response)
        }
        Err(e) => {
            log_warn!(
                "RDAP query failed for {}, falling back to WHOIS: {}",
                query,
                e
            );
            // Fall back to traditional WHOIS
            whois_fallback.await
        }
    }
}

/// Internal RDAP query that returns Err on failure (for fallback logic)
async fn query_rdap_internal(query: &str, query_type: RdapQueryType) -> Result<String> {
    let client =
        RdapClient::new().map_err(|e| anyhow::anyhow!("Failed to create RDAP client: {}", e))?;

    let request = RdapRequest::new(query_type, query);

    let result = client
        .query(&request)
        .await
        .map_err(|e| anyhow::anyhow!("RDAP query error: {}", e))?;

    // Use the rdap crate's built-in WHOIS-style formatter
    let whois_text = format_whois(&result);

    // Check if the response is meaningful (not just an error or empty)
    if whois_text.trim().is_empty() {
        return Err(anyhow::anyhow!("RDAP returned empty response"));
    }

    // Add RDAP source header
    let output = format!(
        "% RDAP (Registration Data Access Protocol) Response\n\
         % Query: {}\n\
         %\n\
         {}\n\
         % End of RDAP response\n",
        query, whois_text
    );

    Ok(output)
}

/// Detect the RDAP query type from a raw query string.
/// Returns None if the query type cannot be determined for RDAP.
pub fn detect_rdap_query_type(query: &str) -> Option<RdapQueryType> {
    RdapRequest::detect_type(query).ok()
}

/// Process explicit RDAP query (for -RDAP suffix queries)
pub async fn process_rdap_query(query: &str) -> Result<String> {
    log_debug!("Processing explicit RDAP query: {}", query);

    // Auto-detect query type
    let query_type = match RdapRequest::detect_type(query) {
        Ok(qt) => qt,
        Err(e) => {
            return Ok(format!(
                "% RDAP Query Error\n\
                 % Unable to detect query type for: {}\n\
                 % Error: {}\n\
                 % \n\
                 % Supported query types:\n\
                 %   - Domain names (e.g., example.com)\n\
                 %   - IP addresses (e.g., 8.8.8.8, 2001:4860:4860::8888)\n\
                 %   - AS numbers (e.g., AS15169 or 15169)\n",
                query, e
            ));
        }
    };

    match query_rdap_internal(query, query_type).await {
        Ok(response) => Ok(response),
        Err(e) => {
            log_warn!("RDAP query failed for {}: {}", query, e);
            Ok(format!(
                "% RDAP Query Failed\n\
                 % Query: {}\n\
                 % Error: {}\n\
                 % \n\
                 % This may be due to:\n\
                 %   - Network connectivity issues\n\
                 %   - RDAP service unavailable\n\
                 %   - Invalid or non-existent resource\n",
                query, e
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rdap_query_format() {
        // Basic test to ensure the function signature works
        let result = process_rdap_query("example.com").await;
        assert!(result.is_ok());
    }
}
