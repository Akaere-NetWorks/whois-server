//! Ping query handler using Globalping API
//!
//! This module provides ping functionality using the Globalping API with
//! detailed information including ASN, geolocation, and PTR records.
//!
//! Supports location-based queries: target-location-PING (e.g., 1.1.1.1-tw-PING)

use anyhow::Result;
use crate::services::utils::{
    GlobalpingClient,
    GlobalpingRequest,
    IpInfoClient,
    DohClient,
    PingOptions,
    MeasurementOptions,
    MeasurementLocation,
};
use crate::{ log_debug, log_error, log_warn };

/// Parse a query with optional location code
/// Returns (target, location) where location is None if not specified
///
/// The suffix has already been removed by query.rs, so we just need to parse
/// the remaining string which may be in format "target" or "target-location"
/// Examples:
///   "1.1.1.1" -> ("1.1.1.1", None)
///   "1.1.1.1-tw" -> ("1.1.1.1", Some("tw"))
///   "example.com-us" -> ("example.com", Some("us"))
fn parse_location_query<'a>(query: &'a str) -> Result<(&'a str, Option<String>)> {
    // Check if there's a location code (format: target-location)
    // Location code is typically 2-5 characters (country codes, region codes)
    // We need to be careful: target can be IP (1.1.1.1) or domain (example.com)
    if let Some(last_dash_pos) = query.rfind('-') {
        let potential_location = &query[last_dash_pos + 1..];
        let potential_target = &query[..last_dash_pos];

        // Validate: target must contain a dot (domain or IP) or be parseable as IP
        // Location codes are short strings without dots
        let is_valid_target = potential_target.contains('.') ||
                             potential_target.parse::<std::net::Ipv4Addr>().is_ok() ||
                             potential_target.parse::<std::net::Ipv6Addr>().is_ok();

        if is_valid_target && potential_location.len() <= 5 && !potential_location.contains('.') {
            return Ok((potential_target, Some(potential_location.to_string())));
        }
    }

    // No location code found, return entire query as target
    Ok((query, None))
}

/// Process a ping query with -PING suffix
/// Supports optional location code: target-location-PING (e.g., 1.1.1.1-tw-PING)
pub async fn process_ping_query(query: &str) -> Result<String> {
    log_debug!("Processing ping query: {}", query);

    // Parse target and location
    // The suffix has already been removed by query.rs
    // Format: target-location or target
    let (target, location) = parse_location_query(query)?;

    // Initialize clients
    let globalping = match GlobalpingClient::new() {
        Ok(client) => client,
        Err(e) => {
            log_error!("Failed to initialize Globalping client: {}", e);
            return Ok(format!("Ping service error: {}\n", e));
        }
    };

    let ip_info_client = IpInfoClient::new(); // May fail if token not set
    let doh_client = DohClient::new();

    // Submit ping measurement to Globalping
    let measurement_opts: MeasurementOptions = MeasurementOptions::Ping(PingOptions {
        packets: Some(4), // 4 packets per probe
        protocol: Some("ICMP".to_string()),
        port: None,
    });

    let mut request = GlobalpingRequest {
        measurement_type: "ping".to_string(),
        target: target.to_string(),
        limit: Some(5),
        measurement_options: Some(measurement_opts),
        locations: None,
        in_progress_updates: Some(false),
    };

    // Add location if specified
    if let Some(loc) = location {
        request.locations = Some(vec![MeasurementLocation {
            magic: Some(loc),
            limit: None,
            continent: None,
            region: None,
            country: None,
            state: None,
            city: None,
            asn: None,
            network: None,
            tags: None,
        }]);
    }

    let measurement_id = match globalping.submit_measurement(&request).await {
        Ok(id) => id,
        Err(e) => {
            log_error!("Failed to submit ping measurement: {}", e);
            return Ok(format!("Ping failed: {}\n", e));
        }
    };

    log_debug!("Ping measurement ID: {}", measurement_id);

    // Wait for results (30 second timeout)
    let results = match globalping.wait_for_results(&measurement_id, 30).await {
        Ok(results) => results,
        Err(e) => {
            log_error!("Failed to get ping results: {}", e);
            return Ok(format!("Ping measurement timed out or failed: {}\n", e));
        }
    };

    // Format and return output
    format_ping_output(&results, &ip_info_client, &doh_client, target).await
}

/// Format ping results with detailed information
async fn format_ping_output(
    results: &crate::services::utils::GlobalpingResult,
    ip_info_client: &Result<IpInfoClient>,
    doh_client: &DohClient,
    target: &str
) -> Result<String> {
    let mut output = String::new();

    if results.results.is_empty() {
        output.push_str(&format!("No results received for ping to {}\n", target));
        return Ok(output);
    }

    // Process results from each probe
    for probe_result in &results.results {
        let test_result = &probe_result.result;
        let probe_info = &probe_result.probe;

        // Get resolved address
        let target_ip = test_result.resolved_address.as_deref().unwrap_or(target);

        // Get IP info for target
        let ip_info = if let Ok(client) = ip_info_client {
            client.get_ip_info(target_ip).await.ok()
        } else {
            None
        };

        let ptr_records = doh_client.query_ptr(target_ip).await.ok();

        // Header with target and probe info
        output.push_str(&format!("PING {} ({})", target, probe_info.network));
        if let Some(info) = &ip_info {
            output.push_str(&format!(" from {}", info.country));
        }
        output.push('\n');

        // Probe location info
        output.push_str(
            &format!(
                "Probe: {} - {}, {}, {}\n",
                probe_info.network,
                probe_info.city.as_deref().unwrap_or("Unknown"),
                probe_info.state.as_deref().unwrap_or(""),
                probe_info.country
            )
        );

        // Statistics
        if let Some(stats) = &test_result.stats {
            let loss_rate = if stats.total > 0 {
                (((stats.loss as f64) / (stats.total as f64)) * 100.0) as u32
            } else {
                0
            };

            output.push_str(
                &format!(
                    "{} packets transmitted, {} received, {}% packet loss\n",
                    stats.total,
                    stats.rcv,
                    loss_rate
                )
            );

            // RTT statistics
            output.push_str(
                &format!(
                    "rtt min/avg/max = {:.2}/{:.2}/{:.2} ms\n",
                    stats.min,
                    stats.avg,
                    stats.max
                )
            );
        }

        // IP info details
        if let Some(info) = &ip_info {
            output.push_str(
                &format!(
                    "  ASN: {} | {} | {} | {}\n",
                    info.asn,
                    info.country,
                    info.continent,
                    info.as_domain
                )
            );
        }

        // PTR records
        if let Some(ptrs) = &ptr_records {
            if !ptrs.is_empty() {
                output.push_str(&format!("  PTR: {}\n", ptrs.join(", ")));
            }
        }

        // Individual packet times
        if let Some(timings) = &test_result.timings {
            output.push_str("  Times: ");
            for (i, timing) in timings.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("{:.2} ms", timing.rtt));
            }
            output.push('\n');
        }

        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network and API tokens
    async fn test_ping_query_formatting() {
        // This test requires actual API calls
        let result = process_ping_query("1.1.1.1-PING").await;
        assert!(result.is_ok());
    }
}
