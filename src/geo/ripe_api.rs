use anyhow::{Result, anyhow};
use tracing::debug;

use crate::geo::constants::{RIPE_STAT_API_BASE, RIPE_RIR_GEO_API_BASE, RIPE_PREFIXES_API_BASE};
use crate::geo::types::{RipeStatResponse, RirGeoResponse, PrefixesResponse};

/// Query RIPE NCC STAT API
pub async fn query_ripe_api(client: &reqwest::Client, resource: &str) -> Result<RipeStatResponse> {
    let url = format!("{}?resource={}", RIPE_STAT_API_BASE, urlencoding::encode(resource));
    debug!("RIPE STAT API URL: {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE API HTTP error: {}", response.status()));
    }
    
    let json_response: RipeStatResponse = response.json().await?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE API error: status={}", json_response.status));
    }
    
    Ok(json_response)
}

/// Query RIPE NCC STAT API (blocking version)
pub fn query_ripe_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<RipeStatResponse> {
    let url = format!("{}?resource={}", RIPE_STAT_API_BASE, urlencoding::encode(resource));
    debug!("RIPE STAT API URL (blocking): {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE API HTTP error: {}", response.status()));
    }
    
    let json_response: RipeStatResponse = response.json()?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE API error: status={}", json_response.status));
    }
    
    Ok(json_response)
}

/// Query RIPE NCC STAT RIR Geo API
pub async fn query_rir_geo_api(client: &reqwest::Client, resource: &str) -> Result<RirGeoResponse> {
    let url = format!("{}?resource={}", RIPE_RIR_GEO_API_BASE, urlencoding::encode(resource));
    debug!("RIPE RIR Geo API URL: {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE RIR Geo API HTTP error: {}", response.status()));
    }
    
    let json_response: RirGeoResponse = response.json().await?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE RIR Geo API error: status={}", json_response.status));
    }
    
    Ok(json_response)
}

/// Query RIPE NCC STAT RIR Geo API (blocking version)
pub fn query_rir_geo_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<RirGeoResponse> {
    let url = format!("{}?resource={}", RIPE_RIR_GEO_API_BASE, urlencoding::encode(resource));
    debug!("RIPE RIR Geo API URL (blocking): {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE RIR Geo API HTTP error: {}", response.status()));
    }
    
    let json_response: RirGeoResponse = response.json()?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE RIR Geo API error: status={}", json_response.status));
    }
    
    Ok(json_response)
}

/// Query RIPE NCC STAT announced-prefixes API
pub async fn query_prefixes_api(client: &reqwest::Client, asn: &str) -> Result<PrefixesResponse> {
    let url = format!("{}?resource={}", RIPE_PREFIXES_API_BASE, urlencoding::encode(asn));
    debug!("RIPE Prefixes API URL: {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE Prefixes API HTTP error: {}", response.status()));
    }
    
    let json_response: PrefixesResponse = response.json().await?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE Prefixes API error: status={}", json_response.status));
    }
    
    Ok(json_response)
}

/// Query RIPE NCC STAT announced-prefixes API (blocking version)
pub fn query_prefixes_api_blocking(client: &reqwest::blocking::Client, asn: &str) -> Result<PrefixesResponse> {
    let url = format!("{}?resource={}", RIPE_PREFIXES_API_BASE, urlencoding::encode(asn));
    debug!("RIPE Prefixes API URL (blocking): {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RIPE Prefixes API HTTP error: {}", response.status()));
    }
    
    let json_response: PrefixesResponse = response.json()?;
    
    if json_response.status != "ok" {
        return Err(anyhow!("RIPE Prefixes API error: status={}", json_response.status));
    }
    
    Ok(json_response)
} 