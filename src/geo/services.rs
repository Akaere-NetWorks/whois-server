use anyhow::Result;
use tracing::debug;
use std::time::Duration;

use crate::geo::ripe_api::{query_ripe_api, query_ripe_api_blocking, query_rir_geo_api, query_rir_geo_api_blocking, query_prefixes_api, query_prefixes_api_blocking};
use crate::geo::ipinfo_api::{query_ipinfo_api, query_ipinfo_api_blocking};
use crate::geo::formatters::{format_combined_geo_response, format_rir_geo_response, format_prefixes_response, format_prefixes_response_blocking};

/// Process geo location queries ending with -GEO
pub async fn process_geo_query(resource: &str) -> Result<String> {
    debug!("Processing geo query for: {}", resource);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    // Query both APIs in parallel
    let ripe_future = query_ripe_api(&client, resource);
    let ipinfo_future = query_ipinfo_api(&client, resource);
    
    let (ripe_result, ipinfo_result) = tokio::join!(ripe_future, ipinfo_future);
    
    format_combined_geo_response(resource, ripe_result, ipinfo_result)
}

/// Process geo location queries ending with -GEO (blocking version)
pub fn process_geo_query_blocking(resource: &str, timeout: Duration) -> Result<String> {
    debug!("Processing geo query (blocking) for: {}", resource);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    
    // Query both APIs
    let ripe_result = query_ripe_api_blocking(&client, resource);
    let ipinfo_result = query_ipinfo_api_blocking(&client, resource);
    
    format_combined_geo_response(resource, ripe_result, ipinfo_result)
}

/// Process RIR geo location queries ending with -RIRGEO
pub async fn process_rir_geo_query(resource: &str) -> Result<String> {
    debug!("Processing RIR geo query for: {}", resource);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = query_rir_geo_api(&client, resource).await?;
    format_rir_geo_response(resource, &response)
}

/// Process RIR geo location queries ending with -RIRGEO (blocking version)
pub fn process_rir_geo_query_blocking(resource: &str, timeout: Duration) -> Result<String> {
    debug!("Processing RIR geo query (blocking) for: {}", resource);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    
    let response = query_rir_geo_api_blocking(&client, resource)?;
    format_rir_geo_response(resource, &response)
}

/// Process ASN prefixes queries ending with -PREFIXES
pub async fn process_prefixes_query(asn: &str) -> Result<String> {
    debug!("Processing prefixes query for ASN: {}", asn);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    // Query prefixes API
    let prefixes_result = query_prefixes_api(&client, asn).await;
    
    match prefixes_result {
        Ok(prefixes_response) => {
            format_prefixes_response(asn, &prefixes_response, &client).await
        }
        Err(e) => {
            let mut formatted = String::new();
            formatted.push_str("% ASN Announced Prefixes Query\n");
            formatted.push_str("% Data from RIPE NCC STAT\n");
            formatted.push_str(&format!("% Query: {}\n", asn));
            formatted.push_str("\n");
            formatted.push_str(&format!("% Error: {}\n", e));
            Ok(formatted)
        }
    }
}

/// Process ASN prefixes queries ending with -PREFIXES (blocking version)
pub fn process_prefixes_query_blocking(asn: &str, timeout: Duration) -> Result<String> {
    debug!("Processing prefixes query (blocking) for ASN: {}", asn);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    
    // Query prefixes API
    let prefixes_result = query_prefixes_api_blocking(&client, asn);
    
    match prefixes_result {
        Ok(prefixes_response) => {
            format_prefixes_response_blocking(asn, &prefixes_response, &client)
        }
        Err(e) => {
            let mut formatted = String::new();
            formatted.push_str("% ASN Announced Prefixes Query\n");
            formatted.push_str("% Data from RIPE NCC STAT\n");
            formatted.push_str(&format!("% Query: {}\n", asn));
            formatted.push_str("\n");
            formatted.push_str(&format!("% Error: {}\n", e));
            Ok(formatted)
        }
    }
} 