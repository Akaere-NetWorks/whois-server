use anyhow::{Result, anyhow};
use super::constants::{IPINFO_API_BASE, IPINFO_TOKEN};
use super::types::IpinfoResponse;

use crate::{log_debug};
/// Query IPinfo API
pub async fn query_ipinfo_api(client: &reqwest::Client, resource: &str) -> Result<IpinfoResponse> {
    let url = format!("{}/{}?token={}", IPINFO_API_BASE, resource, IPINFO_TOKEN);
    log_debug!("IPinfo API URL: {}", url);

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
