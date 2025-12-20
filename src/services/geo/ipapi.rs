use anyhow::Result;
use reqwest::Client;
use super::types::IpApiResponse;

use crate::{log_debug, log_warn};
/// Query IP-API for geo-location information (async version)
pub async fn query_ipapi(client: &Client, ip: &str) -> Result<IpApiResponse> {
    log_debug!("Querying IP-API for: {}", ip);

    let url = format!(
        "http://ip-api.com/json/{}?fields=status,message,country,countryCode,region,regionName,city,zip,lat,lon,timezone,isp,org,as,mobile,proxy,hosting,query",
        ip
    );

    let response = client
        .get(&url)
        .header("User-Agent", "whois-server/1.0")
        .send()
        .await?;

    if !response.status().is_success() {
        log_warn!("IP-API returned non-success status: {}", response.status());
        return Err(anyhow::anyhow!(
            "IP-API request failed with status: {}",
            response.status()
        ));
    }

    let body = response.text().await?;
    log_debug!("IP-API response body: {}", body);

    let api_response: IpApiResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse IP-API response: {}", e))?;

    if api_response.status != "success" {
        log_warn!("IP-API returned error status: {}", api_response.status);
        return Err(anyhow::anyhow!(
            "IP-API returned error: {}",
            api_response.status
        ));
    }

    Ok(api_response)
}
