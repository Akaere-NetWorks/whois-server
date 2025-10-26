use anyhow::Result;
use reqwest::Client;
use tracing::{debug, warn};

use super::types::BilibiliIpResponse;

/// Query BiliBili API for geo-location information (async version)
pub async fn query_bilibili(client: &Client, ip: &str) -> Result<BilibiliIpResponse> {
    debug!("Querying BiliBili API for: {}", ip);

    let url = format!(
        "https://api.live.bilibili.com/client/v1/Ip/getInfoNew?ip={}",
        ip
    );

    let response = client
        .get(&url)
        .header("User-Agent", "whois-server/1.0")
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await?;

    if !response.status().is_success() {
        warn!(
            "BiliBili API returned non-success status: {}",
            response.status()
        );
        return Err(anyhow::anyhow!(
            "BiliBili API request failed with status: {}",
            response.status()
        ));
    }

    let body = response.text().await?;
    debug!("BiliBili API response body: {}", body);

    let api_response: BilibiliIpResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse BiliBili API response: {}", e))?;

    if api_response.code != 0 {
        warn!("BiliBili API returned error code: {}", api_response.code);
        return Err(anyhow::anyhow!(
            "BiliBili API returned error: {} - {}",
            api_response.code,
            api_response.message
        ));
    }

    Ok(api_response)
}
