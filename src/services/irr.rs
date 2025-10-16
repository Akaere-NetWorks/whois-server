use std::time::Duration;
use anyhow::Result;
use serde::Deserialize;
use tracing::debug;

/// IRR Explorer API response structures
#[derive(Debug, Deserialize)]
pub struct IrrResponse {
    #[serde(rename = "rpkiRoutes")]
    pub rpki_routes: Option<Vec<RpkiRoute>>,
    #[serde(rename = "categoryOverall")]
    pub category_overall: String,
    #[serde(rename = "irrRoutes")]
    pub irr_routes: Option<IrrRoutes>,
    pub messages: Option<Vec<Message>>,
    #[serde(rename = "bgpOrigins")]
    pub bgp_origins: Option<Vec<u32>>,
    pub prefix: String,
    pub rir: Option<String>,
    #[serde(rename = "goodnessOverall")]
    pub goodness_overall: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct RpkiRoute {
    #[serde(rename = "rpkiStatus")]
    pub rpki_status: String,
    #[serde(rename = "rpslPk")]
    pub rpsl_pk: String,
    #[serde(rename = "rpslText")]
    pub rpsl_text: String,
    #[allow(dead_code)]
    pub asn: u32,
    #[serde(rename = "rpkiMaxLength")]
    pub rpki_max_length: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct IrrRoutes {
    #[serde(rename = "RIPE")]
    pub ripe: Option<Vec<IrrRoute>>,
    #[serde(rename = "RADB")]
    pub radb: Option<Vec<IrrRoute>>,
    #[serde(rename = "ARIN")]
    pub arin: Option<Vec<IrrRoute>>,
    #[serde(rename = "APNIC")]
    pub apnic: Option<Vec<IrrRoute>>,
    #[serde(rename = "AFRINIC")]
    pub afrinic: Option<Vec<IrrRoute>>,
    #[serde(rename = "LACNIC")]
    pub lacnic: Option<Vec<IrrRoute>>,
    #[serde(rename = "LEVEL3")]
    pub level3: Option<Vec<IrrRoute>>,
    #[serde(rename = "ALTDB")]
    pub altdb: Option<Vec<IrrRoute>>,
    #[serde(rename = "BELL")]
    pub bell: Option<Vec<IrrRoute>>,
    #[serde(rename = "JPIRR")]
    pub jpirr: Option<Vec<IrrRoute>>,
    #[serde(rename = "NTTCOM")]
    pub nttcom: Option<Vec<IrrRoute>>,
    #[serde(rename = "RPKI")]
    pub rpki: Option<Vec<IrrRoute>>,
}

#[derive(Debug, Deserialize)]
pub struct IrrRoute {
    #[serde(rename = "rpkiStatus")]
    pub rpki_status: String,
    #[serde(rename = "rpslPk")]
    pub rpsl_pk: String,
    #[serde(rename = "rpslText")]
    pub rpsl_text: String,
    #[allow(dead_code)]
    pub asn: u32,
    #[serde(rename = "rpkiMaxLength")]
    pub rpki_max_length: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub category: String,
    pub text: String,
}

/// Process IRR Explorer queries ending with -IRR
pub async fn process_irr_query(resource: &str) -> Result<String> {
    debug!("Processing IRR Explorer query for: {}", resource);

    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

    let response = query_irr_explorer_api(&client, resource).await?;
    format_irr_response(resource, &response)
}

/// Query IRR Explorer API
async fn query_irr_explorer_api(
    client: &reqwest::Client,
    resource: &str
) -> Result<Vec<IrrResponse>> {
    let url = format!(
        "https://irrexplorer.nlnog.net/api/prefixes/prefix/{}",
        urlencoding::encode(resource)
    );
    debug!("IRR Explorer API URL: {}", url);

    let response = client.get(&url).header("User-Agent", "akaere-whois-server/1.0").send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("IRR Explorer API HTTP error: {}", response.status()));
    }

    let json_response: Vec<IrrResponse> = response.json().await?;
    Ok(json_response)
}


/// Format IRR Explorer response into RIPE-style whois format
fn format_irr_response(resource: &str, responses: &[IrrResponse]) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% IRR Explorer Query\r\n");
    formatted.push_str("% Data from https://irrexplorer.nlnog.net/\r\n");
    formatted.push_str(&format!("% Query: {}\r\n", resource));
    formatted.push_str("\r\n");

    if responses.is_empty() {
        formatted.push_str("% No IRR data available\r\n");
        return Ok(formatted);
    }

    for (i, response) in responses.iter().enumerate() {
        if i > 0 {
            formatted.push_str("\r\n% --- Next Entry ---\r\n\r\n");
        }

        // Add prefix information
        formatted.push_str(&format!("% Prefix: {}\r\n", response.prefix));
        if let Some(rir) = &response.rir {
            formatted.push_str(&format!("% RIR: {}\r\n", rir));
        }
        formatted.push_str(&format!("% Overall Category: {}\r\n", response.category_overall));
        if let Some(goodness) = response.goodness_overall {
            formatted.push_str(&format!("% Goodness Score: {}\r\n", goodness));
        }

        // Add BGP origins
        if let Some(origins) = &response.bgp_origins {
            if !origins.is_empty() {
                formatted.push_str(
                    &format!(
                        "% BGP Origins: {}\r\n",
                        origins
                            .iter()
                            .map(|o| format!("AS{}", o))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                );
            }
        }

        formatted.push_str("\r\n");

        // Add messages
        if let Some(messages) = &response.messages {
            if !messages.is_empty() {
                formatted.push_str("% Messages:\r\n");
                for msg in messages {
                    formatted.push_str(
                        &format!("% [{:>7}] {}\r\n", msg.category.to_uppercase(), msg.text)
                    );
                }
                formatted.push_str("\r\n");
            }
        }

        // Add RPKI routes
        if let Some(rpki_routes) = &response.rpki_routes {
            if !rpki_routes.is_empty() {
                formatted.push_str("% RPKI Routes:\r\n");
                for route in rpki_routes {
                    formatted.push_str(&format!("% RPKI Status: {}\r\n", route.rpki_status));
                    formatted.push_str(&format!("% RPSL Primary Key: {}\r\n", route.rpsl_pk));
                    if let Some(max_len) = route.rpki_max_length {
                        formatted.push_str(&format!("% Max Length: {}\r\n", max_len));
                    }
                    formatted.push_str("\r\n");
                    formatted.push_str(&route.rpsl_text.replace('\n', "\r\n"));
                    if !route.rpsl_text.ends_with('\n') {
                        formatted.push_str("\r\n");
                    }
                    formatted.push_str("\r\n");
                }
            }
        }

        // Add IRR routes
        if let Some(irr_routes) = &response.irr_routes {
            let databases = [
                ("RIPE", &irr_routes.ripe),
                ("RADB", &irr_routes.radb),
                ("ARIN", &irr_routes.arin),
                ("APNIC", &irr_routes.apnic),
                ("AFRINIC", &irr_routes.afrinic),
                ("LACNIC", &irr_routes.lacnic),
                ("LEVEL3", &irr_routes.level3),
                ("ALTDB", &irr_routes.altdb),
                ("BELL", &irr_routes.bell),
                ("JPIRR", &irr_routes.jpirr),
                ("NTTCOM", &irr_routes.nttcom),
                ("RPKI", &irr_routes.rpki),
            ];

            for (db_name, routes_opt) in databases {
                if let Some(routes) = routes_opt {
                    if !routes.is_empty() {
                        formatted.push_str(&format!("% IRR Database: {}\r\n", db_name));
                        for route in routes {
                            formatted.push_str(
                                &format!("% RPKI Status: {}\r\n", route.rpki_status)
                            );
                            formatted.push_str(
                                &format!("% RPSL Primary Key: {}\r\n", route.rpsl_pk)
                            );
                            if let Some(max_len) = route.rpki_max_length {
                                formatted.push_str(&format!("% Max Length: {}\r\n", max_len));
                            }
                            formatted.push_str("\r\n");
                            formatted.push_str(&route.rpsl_text.replace('\n', "\r\n"));
                            if !route.rpsl_text.ends_with('\n') {
                                formatted.push_str("\r\n");
                            }
                            formatted.push_str("\r\n");
                        }
                    }
                }
            }
        }
    }

    Ok(formatted)
}
