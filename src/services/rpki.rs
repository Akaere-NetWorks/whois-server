use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

// RPKI API
const RPKI_API_BASE: &str = "https://rpki.akae.re/api/v1/validity";

#[derive(Debug, Deserialize, Serialize)]
pub struct RpkiResponse {
    pub validated_route: ValidatedRoute,
    #[serde(rename = "generatedTime")]
    pub generated_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValidatedRoute {
    pub route: Route,
    pub validity: Validity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Route {
    pub origin_asn: String,
    pub prefix: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Validity {
    pub state: String,
    pub description: String,
    #[serde(rename = "VRPs")]
    pub vrps: Vrps,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vrps {
    pub matched: Vec<Vrp>,
    pub unmatched_as: Vec<Vrp>,
    pub unmatched_length: Vec<Vrp>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vrp {
    pub asn: String,
    pub prefix: String,
    pub max_length: String,
}

/// Process RPKI queries in format prefix-asn-RPKI (async version)
pub async fn process_rpki_query(prefix: &str, asn: &str) -> Result<String> {
    debug!("Processing RPKI query for prefix: {}, ASN: {}", prefix, asn);
    
    let url = format!("{}/{}/{}", RPKI_API_BASE, asn, prefix);
    debug!("Requesting RPKI API URL: {}", url);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RPKI API request failed with status: {}", response.status()));
    }
    
    let rpki_response: RpkiResponse = response.json().await?;
    format_rpki_response(prefix, asn, &rpki_response)
}

/// Process RPKI queries in format prefix-asn-RPKI (blocking version)
pub fn process_rpki_query_blocking(prefix: &str, asn: &str, timeout: Duration) -> Result<String> {
    debug!("Processing RPKI query (blocking) for prefix: {}, ASN: {}", prefix, asn);
    
    let url = format!("{}/{}/{}", RPKI_API_BASE, asn, prefix);
    debug!("Requesting RPKI API URL (blocking): {}", url);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    
    let response = client
        .get(&url)
        .header("User-Agent", "akaere-whois-server/1.0")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("RPKI API request failed with status: {}", response.status()));
    }
    
    let rpki_response: RpkiResponse = response.json()?;
    format_rpki_response(prefix, asn, &rpki_response)
}

/// Format RPKI response in RIPE-style format
fn format_rpki_response(prefix: &str, asn: &str, response: &RpkiResponse) -> Result<String> {
    let mut formatted = String::new();
    
    // Header
    formatted.push_str("% RPKI Validation Query\n");
    formatted.push_str("% Data from rpki.akae.re\n");
    formatted.push_str(&format!("% Query: {}-{}-RPKI\n", prefix, asn));
    formatted.push_str(&format!("% Generated Time: {}\n", response.generated_time));
    formatted.push_str("\n");
    
    // Route information
    formatted.push_str("route:\n");
    formatted.push_str(&format!("  origin-asn:     {}\n", response.validated_route.route.origin_asn));
    formatted.push_str(&format!("  prefix:         {}\n", response.validated_route.route.prefix));
    formatted.push_str("\n");
    
    // Validity information
    formatted.push_str("validity:\n");
    formatted.push_str(&format!("  state:          {}\n", response.validated_route.validity.state));
    formatted.push_str(&format!("  description:    {}\n", response.validated_route.validity.description));
    
    if let Some(reason) = &response.validated_route.validity.reason {
        formatted.push_str(&format!("  reason:         {}\n", reason));
    }
    
    formatted.push_str("\n");
    
    // VRPs (Validated ROA Payloads)
    formatted.push_str("vrps:\n");
    
    // Matched VRPs
    if !response.validated_route.validity.vrps.matched.is_empty() {
        formatted.push_str("  matched:\n");
        for vrp in &response.validated_route.validity.vrps.matched {
            formatted.push_str(&format!("    asn:          {}\n", vrp.asn));
            formatted.push_str(&format!("    prefix:       {}\n", vrp.prefix));
            formatted.push_str(&format!("    max-length:   {}\n", vrp.max_length));
            formatted.push_str("\n");
        }
    } else {
        formatted.push_str("  matched:        none\n");
    }
    
    // Unmatched AS VRPs
    if !response.validated_route.validity.vrps.unmatched_as.is_empty() {
        formatted.push_str("  unmatched-as:\n");
        for vrp in &response.validated_route.validity.vrps.unmatched_as {
            formatted.push_str(&format!("    asn:          {}\n", vrp.asn));
            formatted.push_str(&format!("    prefix:       {}\n", vrp.prefix));
            formatted.push_str(&format!("    max-length:   {}\n", vrp.max_length));
            formatted.push_str("\n");
        }
    } else {
        formatted.push_str("  unmatched-as:   none\n");
    }
    
    // Unmatched length VRPs
    if !response.validated_route.validity.vrps.unmatched_length.is_empty() {
        formatted.push_str("  unmatched-length:\n");
        for vrp in &response.validated_route.validity.vrps.unmatched_length {
            formatted.push_str(&format!("    asn:          {}\n", vrp.asn));
            formatted.push_str(&format!("    prefix:       {}\n", vrp.prefix));
            formatted.push_str(&format!("    max-length:   {}\n", vrp.max_length));
            formatted.push_str("\n");
        }
    } else {
        formatted.push_str("  unmatched-length: none\n");
    }
    
    // Summary
    formatted.push_str("\n% End of RPKI validation result\n");
    
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rpki_response() {
        let response = RpkiResponse {
            validated_route: ValidatedRoute {
                route: Route {
                    origin_asn: "AS13335".to_string(),
                    prefix: "1.1.1.0/24".to_string(),
                },
                validity: Validity {
                    state: "valid".to_string(),
                    description: "At least one VRP Matches the Route Prefix".to_string(),
                    vrps: Vrps {
                        matched: vec![Vrp {
                            asn: "AS13335".to_string(),
                            prefix: "1.1.1.0/24".to_string(),
                            max_length: "24".to_string(),
                        }],
                        unmatched_as: vec![],
                        unmatched_length: vec![],
                    },
                    reason: None,
                },
            },
            generated_time: "2025-06-17T15:27:27Z".to_string(),
        };
        
        let formatted = format_rpki_response("1.1.1.0/24", "13335", &response).unwrap();
        
        assert!(formatted.contains("% RPKI Validation Query"));
        assert!(formatted.contains("% Query: 1.1.1.0/24-13335-RPKI"));
        assert!(formatted.contains("state:          valid"));
        assert!(formatted.contains("origin-asn:     AS13335"));
        assert!(formatted.contains("prefix:         1.1.1.0/24"));
    }
} 