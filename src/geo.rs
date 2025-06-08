use anyhow::{Result, anyhow};
use serde::Deserialize;
use tracing::debug;
use std::time::Duration;

// API endpoints
const RIPE_STAT_API_BASE: &str = "https://stat.ripe.net/data/maxmind-geo-lite/data.json";
const RIPE_RIR_GEO_API_BASE: &str = "https://stat.ripe.net/data/rir-geo/data.json";
const IPINFO_API_BASE: &str = "https://api.ipinfo.io/lite";
const IPINFO_TOKEN: &str = "29a9fd77d1bd76";

#[derive(Debug, Deserialize)]
struct RipeStatResponse {
    data: Option<RipeStatData>,
    status: String,
    #[allow(dead_code)]
    messages: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
struct RipeStatData {
    #[allow(dead_code)]
    prefixes: Option<Vec<GeoPrefix>>,
    located_resources: Option<Vec<LocatedResource>>,
    #[allow(dead_code)]
    unknown_resources: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeoPrefix {
    #[allow(dead_code)]
    prefix: String,
    #[allow(dead_code)]
    country: Option<String>,
    #[allow(dead_code)]
    city: Option<String>,
    #[allow(dead_code)]
    latitude: Option<f64>,
    #[allow(dead_code)]
    longitude: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct LocatedResource {
    resource: String,
    locations: Option<Vec<GeoLocation>>,
    #[allow(dead_code)]
    unknown_percentage: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeoLocation {
    country: Option<String>,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    #[allow(dead_code)]
    resources: Option<Vec<String>>,
    #[allow(dead_code)]
    covered_percentage: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct IpinfoResponse {
    ip: String,
    asn: Option<String>,
    as_name: Option<String>,
    as_domain: Option<String>,
    #[allow(dead_code)]
    country_code: Option<String>,
    country: Option<String>,
    #[allow(dead_code)]
    continent_code: Option<String>,
    continent: Option<String>,
    city: Option<String>,
    region: Option<String>,
    #[allow(dead_code)]
    latitude: Option<String>,
    #[allow(dead_code)]
    longitude: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RirGeoResponse {
    data: Option<RirGeoData>,
    status: String,
    #[allow(dead_code)]
    messages: Option<Vec<Vec<String>>>,
    #[allow(dead_code)]
    see_also: Option<Vec<String>>,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    data_call_name: String,
    #[allow(dead_code)]
    data_call_status: String,
    #[allow(dead_code)]
    cached: bool,
    #[allow(dead_code)]
    query_id: String,
    #[allow(dead_code)]
    process_time: u32,
    #[allow(dead_code)]
    server_id: String,
    #[allow(dead_code)]
    build_version: String,
    #[allow(dead_code)]
    status_code: u16,
    #[allow(dead_code)]
    time: String,
}

#[derive(Debug, Deserialize)]
struct RirGeoData {
    located_resources: Option<Vec<RirGeoResource>>,
    #[allow(dead_code)]
    result_time: String,
    #[allow(dead_code)]
    parameters: RirGeoParameters,
    #[allow(dead_code)]
    earliest_time: String,
    #[allow(dead_code)]
    latest_time: String,
}

#[derive(Debug, Deserialize)]
struct RirGeoResource {
    resource: String,
    location: String,
}

#[derive(Debug, Deserialize)]
struct RirGeoParameters {
    #[allow(dead_code)]
    resource: String,
    #[allow(dead_code)]
    query_time: String,
    #[allow(dead_code)]
    cache: Option<String>,
}

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

/// Query RIPE NCC STAT API
async fn query_ripe_api(client: &reqwest::Client, resource: &str) -> Result<RipeStatResponse> {
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

/// Query IPinfo API
async fn query_ipinfo_api(client: &reqwest::Client, resource: &str) -> Result<IpinfoResponse> {
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

/// Query RIPE NCC STAT API (blocking version)
fn query_ripe_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<RipeStatResponse> {
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

/// Query IPinfo API (blocking version)
fn query_ipinfo_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<IpinfoResponse> {
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

/// Process RIR geo location queries ending with -RIRGEO
pub async fn process_rir_geo_query(resource: &str) -> Result<String> {
    debug!("Processing RIR geo query for: {}", resource);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = query_rir_geo_api(&client, resource).await?;
    format_rir_geo_response(resource, &response)
}

/// Query RIPE NCC STAT RIR Geo API
async fn query_rir_geo_api(client: &reqwest::Client, resource: &str) -> Result<RirGeoResponse> {
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

/// Process RIR geo location queries ending with -RIRGEO (blocking version)
pub fn process_rir_geo_query_blocking(resource: &str, timeout: Duration) -> Result<String> {
    debug!("Processing RIR geo query (blocking) for: {}", resource);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    
    let response = query_rir_geo_api_blocking(&client, resource)?;
    format_rir_geo_response(resource, &response)
}

/// Query RIPE NCC STAT RIR Geo API (blocking version)
fn query_rir_geo_api_blocking(client: &reqwest::blocking::Client, resource: &str) -> Result<RirGeoResponse> {
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

/// Format RIR geo location response
fn format_rir_geo_response(resource: &str, response: &RirGeoResponse) -> Result<String> {
    let mut formatted = String::new();
    
    // Header
    formatted.push_str("% RIPE NCC STAT RIR Geographic Query\n");
    formatted.push_str("% Data from RIR Statistics\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");
    
    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No RIR geographic data available\n");
            return Ok(formatted);
        }
    };
    
    // Display located resources
    if let Some(located) = &data.located_resources {
        if !located.is_empty() {
            formatted.push_str("RIR Geographic Location Results\n");
            formatted.push_str("===============================\n\n");
            formatted.push_str("Resource                    | Country Code\n");
            formatted.push_str("----------------------------|-------------\n");
            
            for item in located {
                formatted.push_str(&format!(
                    "{:<27} | {}\n",
                    truncate_string(&item.resource, 27),
                    item.location
                ));
            }
            formatted.push_str("\n");
            
            // Summary
            formatted.push_str(&format!("% Total located resources: {}\n", located.len()));
        }
    } else {
        formatted.push_str("% No located resources found\n");
    }
    
    // Show messages if any
    if let Some(messages) = &response.messages {
        if !messages.is_empty() {
            formatted.push_str("\n% API Messages:\n");
            for message in messages {
                for msg_part in message {
                    formatted.push_str(&format!("% {}\n", msg_part));
                }
            }
        }
    }
    
    Ok(formatted)
}

/// Format combined geo location response from both APIs
fn format_combined_geo_response(
    resource: &str, 
    ripe_result: Result<RipeStatResponse>, 
    ipinfo_result: Result<IpinfoResponse>
) -> Result<String> {
    let mut formatted = String::new();
    
    // Header
    formatted.push_str("% Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT (MaxMind GeoLite2) and IPinfo\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");
    
    // RIPE NCC STAT section
    formatted.push_str("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
    match ripe_result {
        Ok(ripe_response) => {
            if let Some(data) = &ripe_response.data {
                if let Some(located) = &data.located_resources {
                    if !located.is_empty() {
                        // Collect all data first to calculate column widths
                        let mut rows = Vec::new();
                        let mut max_resource_len = 8; // "Resource"
                        let mut max_country_len = 7;  // "Country"
                        let mut max_city_len = 4;     // "City"
                        
                        for item in located {
                            if let Some(locations) = &item.locations {
                                for location in locations {
                                    let country = location.country.as_deref().unwrap_or("N/A");
                                    let city = location.city.as_deref().unwrap_or("N/A");
                                    let lat = location.latitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                                    let lon = location.longitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                                    
                                    max_resource_len = std::cmp::max(max_resource_len, item.resource.len());
                                    max_country_len = std::cmp::max(max_country_len, country.len());
                                    max_city_len = std::cmp::max(max_city_len, city.len());
                                    
                                    rows.push((item.resource.clone(), country.to_string(), city.to_string(), lat, lon));
                                }
                            } else {
                                max_resource_len = std::cmp::max(max_resource_len, item.resource.len());
                                rows.push((item.resource.clone(), "N/A".to_string(), "N/A".to_string(), "N/A".to_string(), "N/A".to_string()));
                            }
                        }
                        
                        // Dynamic header
                        formatted.push_str(&format!(
                            "{:<width1$} | {:<width2$} | {:<width3$} | Latitude  | Longitude\n",
                            "Resource", "Country", "City",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        ));
                        
                        // Dynamic separator
                        formatted.push_str(&format!(
                            "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-----------|----------\n",
                            "", "", "",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        ));
                        
                        // Data rows
                        for (resource, country, city, lat, lon) in rows {
                            formatted.push_str(&format!(
                                "{:<width1$} | {:<width2$} | {:<width3$} | {:<9} | {}\n",
                                resource, country, city, lat, lon,
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            ));
                        }
                    } else {
                        formatted.push_str("% No location data available\n");
                    }
                } else {
                    formatted.push_str("% No location data available\n");
                }
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }
    
    formatted.push_str("\n");
    
    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");
            
            // Calculate dynamic column widths
            let resource_width = std::cmp::max(8, ipinfo_response.ip.len());
            let country_width = std::cmp::max(7, country.len());
            let city_width = std::cmp::max(4, city.len());
            let asn_width = std::cmp::max(3, asn.len());
            
            // Dynamic header
            formatted.push_str(&format!(
                "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | AS Name\n",
                "Resource", "Country", "City", "ASN",
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            ));
            
            // Dynamic separator
            formatted.push_str(&format!(
                "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-{:-<width4$}-|----------\n",
                "", "", "", "",
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            ));
            
            // Data row with dynamic widths
            formatted.push_str(&format!(
                "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | {}\n",
                ipinfo_response.ip,
                country,
                city,
                asn,
                as_name,
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            ));
            
            // Additional info if available
            if let Some(continent) = &ipinfo_response.continent {
                formatted.push_str(&format!("% Continent: {}\n", continent));
            }
            if let Some(region) = &ipinfo_response.region {
                formatted.push_str(&format!("% Region: {}\n", region));
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                formatted.push_str(&format!("% AS Domain: {}\n", as_domain));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }
    
    formatted.push_str("\n");
    
    Ok(formatted)
}

/// Format geo location response
#[allow(dead_code)]
fn format_geo_response(resource: &str, response: &RipeStatResponse) -> Result<String> {
    let mut formatted = String::new();
    
    // Header
    formatted.push_str("% RIPE NCC STAT Geo Location Query\n");
    formatted.push_str("% Data from MaxMind GeoLite2\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");
    
    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No geo location data available\n");
            return Ok(formatted);
        }
    };
    
    // Check for unknown resources
    if let Some(unknown) = &data.unknown_resources {
        if !unknown.is_empty() {
            formatted.push_str("% Unknown resources:\n");
            for resource in unknown {
                formatted.push_str(&format!("% {}\n", resource));
            }
            formatted.push_str("\n");
        }
    }
    
    // Display located resources
    if let Some(located) = &data.located_resources {
        if !located.is_empty() {
            formatted.push_str("Resource    | Country | City           | Latitude  | Longitude\n");
            formatted.push_str("------------|---------|----------------|-----------|----------\n");
            
            for item in located {
                if let Some(locations) = &item.locations {
                    for location in locations {
                        let country = location.country.as_deref().unwrap_or("N/A");
                        let city = location.city.as_deref().unwrap_or("N/A");
                        let lat = location.latitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                        let lon = location.longitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                        
                        formatted.push_str(&format!(
                            "{:<11} | {:<7} | {:<14} | {:<9} | {}\n",
                            truncate_string(&item.resource, 11),
                            truncate_string(country, 7),
                            truncate_string(city, 14),
                            lat,
                            lon
                        ));
                    }
                } else {
                    // No location data for this resource
                    formatted.push_str(&format!(
                        "{:<11} | {:<7} | {:<14} | {:<9} | {}\n",
                        truncate_string(&item.resource, 11),
                        "N/A",
                        "N/A",
                        "N/A",
                        "N/A"
                    ));
                }
            }
            formatted.push_str("\n");
        }
    }
    
    // Display prefix information
    if let Some(prefixes) = &data.prefixes {
        if !prefixes.is_empty() {
            formatted.push_str("Prefix              | Country | City           | Latitude  | Longitude\n");
            formatted.push_str("--------------------|---------|----------------|-----------|----------\n");
            
            for prefix in prefixes {
                let country = prefix.country.as_deref().unwrap_or("N/A");
                let city = prefix.city.as_deref().unwrap_or("N/A");
                let lat = prefix.latitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                let lon = prefix.longitude.map(|f| format!("{:.4}", f)).unwrap_or_else(|| "N/A".to_string());
                
                formatted.push_str(&format!(
                    "{:<19} | {:<7} | {:<14} | {:<9} | {}\n",
                    truncate_string(&prefix.prefix, 19),
                    truncate_string(country, 7),
                    truncate_string(city, 14),
                    lat,
                    lon
                ));
            }
        }
    }
    
    // Show messages if any
    if let Some(messages) = &response.messages {
        if !messages.is_empty() {
            formatted.push_str("\n% API Messages:\n");
            for message in messages {
                for msg_part in message {
                    formatted.push_str(&format!("% {}\n", msg_part));
                }
            }
        }
    }
    
    Ok(formatted)
}

/// Truncate string to specified length
#[allow(dead_code)]
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("very_long_string", 10), "very_lo...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    #[test]
    fn test_format_geo_response_empty() {
        let response = RipeStatResponse {
            data: None,
            status: "ok".to_string(),
            messages: None,
        };
        
        let formatted = format_geo_response("192.168.1.1", &response).unwrap();
        assert!(formatted.contains("% RIPE NCC STAT Geo Location Query"));
        assert!(formatted.contains("% Query: 192.168.1.1"));
        assert!(formatted.contains("% No geo location data available"));
    }
    
    #[test]
    fn test_format_rir_geo_response_empty() {
        let response = RirGeoResponse {
            data: None,
            status: "ok".to_string(),
            messages: None,
            see_also: None,
            version: "1.0".to_string(),
            data_call_name: "rir-geo".to_string(),
            data_call_status: "supported".to_string(),
            cached: false,
            query_id: "test".to_string(),
            process_time: 41,
            server_id: "test".to_string(),
            build_version: "test".to_string(),
            status_code: 200,
            time: "2025-06-08T18:05:15.809098".to_string(),
        };
        
        let formatted = format_rir_geo_response("2001:67c:2e8::/48", &response).unwrap();
        assert!(formatted.contains("% RIPE NCC STAT RIR Geographic Query"));
        assert!(formatted.contains("% Query: 2001:67c:2e8::/48"));
        assert!(formatted.contains("% No RIR geographic data available"));
    }
    
    #[test]
    fn test_format_rir_geo_response_with_data() {
        let response = RirGeoResponse {
            data: Some(RirGeoData {
                located_resources: Some(vec![
                    RirGeoResource {
                        resource: "2001:67c:2e8::/48".to_string(),
                        location: "NL".to_string(),
                    }
                ]),
                result_time: "2025-06-07T00:00:00".to_string(),
                parameters: RirGeoParameters {
                    resource: "2001:67c:2e8::/48".to_string(),
                    query_time: "2025-06-07T00:00:00".to_string(),
                    cache: None,
                },
                earliest_time: "2005-02-18T00:00:00".to_string(),
                latest_time: "2025-06-07T00:00:00".to_string(),
            }),
            status: "ok".to_string(),
            messages: None,
            see_also: None,
            version: "1.0".to_string(),
            data_call_name: "rir-geo".to_string(),
            data_call_status: "supported".to_string(),
            cached: false,
            query_id: "test".to_string(),
            process_time: 41,
            server_id: "test".to_string(),
            build_version: "test".to_string(),
            status_code: 200,
            time: "2025-06-08T18:05:15.809098".to_string(),
        };
        
        let formatted = format_rir_geo_response("2001:67c:2e8::/48", &response).unwrap();
        assert!(formatted.contains("% RIPE NCC STAT RIR Geographic Query"));
        assert!(formatted.contains("% Query: 2001:67c:2e8::/48"));
        assert!(formatted.contains("RIR Geographic Location Results"));
        assert!(formatted.contains("2001:67c:2e8::/48"));
        assert!(formatted.contains("NL"));
        assert!(formatted.contains("% Total located resources: 1"));
    }
} 