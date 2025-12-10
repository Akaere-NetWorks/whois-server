// WHOIS Server - Cloudflare Status Service
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cloudflare Status API service
//!
//! This module provides functionality to query Cloudflare's status page API
//! to retrieve system status, component statuses, and incident information.

use anyhow::{ Context, Result, anyhow };
use serde::{ Deserialize, Serialize };
use tracing::debug;

const CLOUDFLARE_STATUS_API: &str = "https://www.cloudflarestatus.com/api/v2";
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// Response structure for the status endpoint
#[derive(Debug, Deserialize, Serialize)]
struct StatusResponse {
    page: PageInfo,
    status: StatusInfo,
}

/// Response structure for the components endpoint
#[derive(Debug, Deserialize, Serialize)]
struct ComponentsResponse {
    page: PageInfo,
    components: Vec<Component>,
}

/// Response structure for the incidents endpoint
#[derive(Debug, Deserialize, Serialize)]
struct IncidentsResponse {
    page: PageInfo,
    incidents: Vec<Incident>,
}

/// Page information
#[derive(Debug, Deserialize, Serialize)]
struct PageInfo {
    id: String,
    name: String,
    url: String,
    updated_at: String,
}

/// Overall status information
#[derive(Debug, Deserialize, Serialize)]
struct StatusInfo {
    description: String,
    indicator: String,
}

/// Component status information
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Component {
    id: String,
    name: String,
    status: String,
    description: Option<String>,
    group: bool,
    group_id: Option<String>,
    position: i32,
    showcase: bool,
    created_at: String,
    updated_at: String,
}

/// Incident information
#[derive(Debug, Deserialize, Serialize)]
struct Incident {
    id: String,
    name: String,
    status: String,
    impact: String,
    created_at: String,
    updated_at: String,
    monitoring_at: Option<String>,
    resolved_at: Option<String>,
    shortlink: String,
    incident_updates: Vec<IncidentUpdate>,
}

/// Incident update information
#[derive(Debug, Deserialize, Serialize)]
struct IncidentUpdate {
    id: String,
    status: String,
    body: String,
    created_at: String,
    display_at: String,
    updated_at: String,
}

/// Process a Cloudflare Status query
pub async fn process_cfstatus_query(query: &str) -> Result<String> {
    debug!("Processing Cloudflare Status query: {}", query);

    // Extract the base query without the -CFSTATUS suffix
    let base_query = query
        .strip_suffix("-CFSTATUS")
        .or_else(|| query.strip_suffix("-cfstatus"))
        .unwrap_or(query)
        .trim();

    // Determine what type of query to perform
    match base_query.to_uppercase().as_str() {
        "" | "STATUS" => query_status().await,
        "COMPONENTS" => query_components().await,
        "INCIDENTS" => query_incidents().await,
        _ => {
            // Default to status query
            query_status().await
        }
    }
}

/// Query the overall Cloudflare status
async fn query_status() -> Result<String> {
    debug!("Querying Cloudflare overall status");

    let url = format!("{}/status.json", CLOUDFLARE_STATUS_API);
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(&url)
        .send().await
        .context("Failed to send request to Cloudflare Status API")?;

    if !response.status().is_success() {
        return Err(anyhow!("Cloudflare Status API returned error: {}", response.status()));
    }

    let status_response: StatusResponse = response
        .json().await
        .context("Failed to parse Cloudflare Status API response")?;

    Ok(format_status_response(&status_response))
}

/// Query Cloudflare component statuses
async fn query_components() -> Result<String> {
    debug!("Querying Cloudflare components");

    let url = format!("{}/components.json", CLOUDFLARE_STATUS_API);
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(&url)
        .send().await
        .context("Failed to send request to Cloudflare Status API")?;

    if !response.status().is_success() {
        return Err(anyhow!("Cloudflare Status API returned error: {}", response.status()));
    }

    let components_response: ComponentsResponse = response
        .json().await
        .context("Failed to parse Cloudflare Status API response")?;

    Ok(format_components_response(&components_response))
}

/// Query Cloudflare unresolved incidents
async fn query_incidents() -> Result<String> {
    debug!("Querying Cloudflare unresolved incidents");

    let url = format!("{}/incidents/unresolved.json", CLOUDFLARE_STATUS_API);
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(&url)
        .send().await
        .context("Failed to send request to Cloudflare Status API")?;

    if !response.status().is_success() {
        return Err(anyhow!("Cloudflare Status API returned error: {}", response.status()));
    }

    let incidents_response: IncidentsResponse = response
        .json().await
        .context("Failed to parse Cloudflare Status API response")?;

    Ok(format_incidents_response(&incidents_response))
}

/// Format the status response for display
fn format_status_response(response: &StatusResponse) -> String {
    let indicator_symbol = match response.status.indicator.as_str() {
        "none" => "✓",
        "minor" => "⚠",
        "major" => "⚠",
        "critical" => "✗",
        _ => "?",
    };

    let mut output = String::new();
    output.push_str(&format!("% Cloudflare Status - {}\n", response.page.name));
    output.push_str(&format!("% Last Updated: {}\n", response.page.updated_at));
    output.push_str(&format!("% URL: {}\n", response.page.url));
    output.push_str("%\n");
    output.push_str(&format!("% Status: {} {}\n", indicator_symbol, response.status.description));
    output.push_str(&format!("% Indicator: {}\n", response.status.indicator));
    output.push_str("%\n");
    output.push_str("% Query 'components-cfstatus' for component details\n");
    output.push_str("% Query 'incidents-cfstatus' for unresolved incidents\n");

    output
}

/// Format the components response for display
fn format_components_response(response: &ComponentsResponse) -> String {
    let mut output = String::new();
    output.push_str(&format!("% Cloudflare Components - {}\n", response.page.name));
    output.push_str(&format!("% Last Updated: {}\n", response.page.updated_at));
    output.push_str("%\n");

    if response.components.is_empty() {
        output.push_str("% No components found\n");
        return output;
    }

    // Sort components by position
    let mut components = response.components.clone();
    components.sort_by_key(|c| c.position);

    output.push_str(&format!("% Total Components: {}\n", components.len()));
    output.push_str("%\n");

    for component in &components {
        let status_symbol = match component.status.as_str() {
            "operational" => "✓",
            "degraded_performance" => "⚠",
            "partial_outage" => "⚠",
            "major_outage" => "✗",
            _ => "?",
        };

        output.push_str(
            &format!("% {} {} ({})\n", status_symbol, component.name, component.status)
        );

        if let Some(desc) = &component.description {
            if !desc.is_empty() {
                output.push_str(&format!("%   Description: {}\n", desc));
            }
        }

        if component.group {
            output.push_str("%   Type: Component Group\n");
        }

        output.push_str(&format!("%   ID: {}\n", component.id));
        output.push_str(&format!("%   Updated: {}\n", component.updated_at));
        output.push_str("%\n");
    }

    output
}

/// Format the incidents response for display
fn format_incidents_response(response: &IncidentsResponse) -> String {
    let mut output = String::new();
    output.push_str(&format!("% Cloudflare Incidents - {}\n", response.page.name));
    output.push_str(&format!("% Last Updated: {}\n", response.page.updated_at));
    output.push_str("%\n");

    if response.incidents.is_empty() {
        output.push_str("% No unresolved incidents\n");
        output.push_str("% All systems operational\n");
        return output;
    }

    output.push_str(&format!("% Unresolved Incidents: {}\n", response.incidents.len()));
    output.push_str("%\n");

    for incident in &response.incidents {
        let impact_symbol = match incident.impact.as_str() {
            "none" => "○",
            "minor" => "●",
            "major" => "●",
            "critical" => "●",
            _ => "?",
        };

        output.push_str(
            &format!("% {} {} [{}]\n", impact_symbol, incident.name, incident.impact.to_uppercase())
        );
        output.push_str(&format!("%   Status: {}\n", incident.status));
        output.push_str(&format!("%   Created: {}\n", incident.created_at));
        output.push_str(&format!("%   Updated: {}\n", incident.updated_at));
        output.push_str(&format!("%   Short Link: {}\n", incident.shortlink));

        if !incident.incident_updates.is_empty() {
            output.push_str("%\n");
            output.push_str("%   Latest Updates:\n");

            // Show the 3 most recent updates
            let updates_to_show = incident.incident_updates.iter().take(3);

            for update in updates_to_show {
                output.push_str(&format!("%     [{} at {}]\n", update.status, update.created_at));

                // Wrap the body text
                let wrapped_body = wrap_text(&update.body, 70);
                for line in wrapped_body.lines() {
                    output.push_str(&format!("%     {}\n", line));
                }
                output.push_str("%\n");
            }
        }

        output.push_str("%\n");
    }

    output
}

/// Wrap text to a maximum line width
fn wrap_text(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                result.push_str(&current_line);
                result.push('\n');
                current_line.clear();
            }
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        result.push_str(&current_line);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_status() {
        let result = query_status().await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Cloudflare Status"));
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a very long line that should be wrapped at the maximum width specified";
        let wrapped = wrap_text(text, 20);
        for line in wrapped.lines() {
            assert!(line.len() <= 20);
        }
    }
}
