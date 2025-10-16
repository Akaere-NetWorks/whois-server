use anyhow::Result;
use reqwest::Client;
use tracing::{ debug, warn };

use super::types::{ MeituanIpResponse, MeituanCityResponse, MeituanCityData };

/// Combined Meituan response containing both IP location and city details
#[derive(Debug, Clone)]
pub struct MeituanCombinedResponse {
    pub ip: String,
    pub country: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub adcode: String,
    pub lat: f64,
    pub lng: f64,
    pub fromwhere: String,
    pub city_details: Option<MeituanCityData>,
}

/// Query Meituan API for geo-location information (async version)
pub async fn query_meituan(client: &Client, ip: &str) -> Result<MeituanCombinedResponse> {
    debug!("Querying Meituan API for: {}", ip);

    // Step 1: Get IP location data
    let ip_url = format!("https://apimobile.meituan.com/locate/v2/ip/loc?rgeo=true&ip={}", ip);

    let ip_response = client
        .get(&ip_url)
        .header("User-Agent", "whois-server/1.0")
        .header("Referer", "https://www.meituan.com/")
        .send().await?;

    if !ip_response.status().is_success() {
        warn!("Meituan IP API returned non-success status: {}", ip_response.status());
        return Err(
            anyhow::anyhow!("Meituan IP API request failed with status: {}", ip_response.status())
        );
    }

    let ip_body = ip_response.text().await?;
    debug!("Meituan IP API response body: {}", ip_body);

    let ip_api_response: MeituanIpResponse = serde_json
        ::from_str(&ip_body)
        .map_err(|e| anyhow::anyhow!("Failed to parse Meituan IP API response: {}", e))?;

    let ip_data = ip_api_response.data.ok_or_else(||
        anyhow::anyhow!("Meituan IP API returned no data")
    )?;

    // Step 2: Get city details using coordinates
    let city_url = format!(
        "https://apimobile.meituan.com/group/v1/city/latlng/{},{}?tag=0",
        ip_data.lat,
        ip_data.lng
    );

    let city_response = client
        .get(&city_url)
        .header("User-Agent", "whois-server/1.0")
        .header("Referer", "https://www.meituan.com/")
        .send().await?;

    let city_details = if city_response.status().is_success() {
        let city_body = city_response.text().await?;
        debug!("Meituan City API response body: {}", city_body);

        match serde_json::from_str::<MeituanCityResponse>(&city_body) {
            Ok(city_api_response) => city_api_response.data,
            Err(e) => {
                warn!("Failed to parse Meituan City API response: {}", e);
                None
            }
        }
    } else {
        warn!("Meituan City API returned non-success status: {}", city_response.status());
        None
    };

    // Combine the results
    let combined = MeituanCombinedResponse {
        ip: ip_data.ip,
        country: ip_data.rgeo.country,
        province: ip_data.rgeo.province,
        city: ip_data.rgeo.city,
        district: ip_data.rgeo.district,
        adcode: ip_data.rgeo.adcode,
        lat: ip_data.lat,
        lng: ip_data.lng,
        fromwhere: ip_data.fromwhere,
        city_details,
    };

    Ok(combined)
}
