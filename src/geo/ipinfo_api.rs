use anyhow::{Result, anyhow};
use tracing::debug;

use crate::geo::constants::{IPINFO_API_BASE, IPINFO_TOKEN};
use crate::geo::types::IpinfoResponse;

/// Query IPinfo API
pub async fn query_ipinfo_api(client: &reqwest::Client, resource: &str) -> Result<IpinfoResponse> {
    let url = format!("{}/{}?token={}", IPINFO_API_BASE, resource, IPINFO_TOKEN);
    debug!("IPinfo API URL: {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("IPinfo API HTTP error: {}", response.status()));
    }
    
    let json_response: IpinfoResponse = response.json().await?;
    Ok(json_response)
}

/// Query IPinfo API (blocking version)
pub fn query_ipinfo_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<IpinfoResponse> {
    let url = format!("{}/{}?token={}", IPINFO_API_BASE, resource, IPINFO_TOKEN);
    debug!("IPinfo API URL (blocking): {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("IPinfo API HTTP error: {}", response.status()));
    }
    
    let json_response: IpinfoResponse = response.json()?;
    Ok(json_response)
} 