//! Traceroute query handler using Globalping API
//!
//! This module provides traceroute functionality using the Globalping API with
//! detailed information including ASN, geolocation, PTR records, and hop-by-hop analysis.
//!
//! Supports location-based queries: target-location-TRACE (e.g., 1.1.1.1-us-TRACE)

use anyhow::Result;
use crate::services::utils::{GlobalpingClient, GlobalpingRequest, IpInfoClient, DohClient, TracerouteOptions, MeasurementOptions, MeasurementLocation};
use crate::{log_debug, log_error, log_warn};

/// Parse a query with optional location code
/// Returns (target, location) where location is None if not specified
///
/// The suffix has already been removed by query.rs, so we just need to parse
/// the remaining string which may be in format "target" or "target-location"
/// Examples:
///   "1.1.1.1" -> ("1.1.1.1", None)
///   "1.1.1.1-TW" -> ("1.1.1.1", Some("TW"))
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

/// Process a traceroute query with -TRACE suffix
/// Supports optional location code: target-location-TRACE (e.g., 1.1.1.1-us-TRACE)
pub async fn process_traceroute_query(query: &str) -> Result<String> {
    log_debug!("Processing traceroute query: {}", query);

    // Parse target and location
    // The suffix has already been removed by query.rs
    // Format: target-location or target
    let (target, location) = parse_location_query(query)?;

    log_debug!("Starting traceroute to {} (location: {:?})", target, location);

    // Initialize clients
    let globalping = match GlobalpingClient::new() {
        Ok(client) => client,
        Err(e) => {
            log_error!("Failed to initialize Globalping client: {}", e);
            return Ok(format!("Traceroute service error: {}\n", e));
        }
    };

    let ip_info_client = IpInfoClient::new(); // May fail if token not set
    let doh_client = DohClient::new();

    // Submit traceroute measurement to Globalping
    let measurement_opts: MeasurementOptions = MeasurementOptions::Traceroute(TracerouteOptions {
        protocol: Some("ICMP".to_string()),
        port: None,
    });

    log_debug!("Parsed target: '{}', location: {:?}", target, location);

    let mut request: GlobalpingRequest = GlobalpingRequest {
        measurement_type: "traceroute".to_string(),
        target: target.to_string(),
        limit: Some(1), // Use 1 probe
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
            log_error!("Failed to submit traceroute measurement: {}", e);
            return Ok(format!("Traceroute failed: {}\n", e));
        }
    };

    log_debug!("Traceroute measurement ID: {}", measurement_id);

    // Wait for results (60 second timeout for traceroute)
    let results = match globalping.wait_for_results(&measurement_id, 60).await {
        Ok(results) => results,
        Err(e) => {
            log_error!("Failed to get traceroute results: {}", e);
            return Ok(format!("Traceroute measurement timed out or failed: {}\n", e));
        }
    };

    // Format and return output
    format_traceroute_output(&results, &ip_info_client, &doh_client, target).await
}

/// Format traceroute results with detailed hop information
async fn format_traceroute_output(
    results: &crate::services::utils::GlobalpingResult,
    ip_info_client: &Result<IpInfoClient>,
    doh_client: &DohClient,
    target: &str,
) -> Result<String> {
    let mut output = String::new();

    if results.results.is_empty() {
        output.push_str(&format!("No results received for traceroute to {}\n", target));
        return Ok(output);
    }

    // Process results from each probe
    for probe_result in &results.results {
        let test_result = &probe_result.result;
        let probe_info = &probe_result.probe;

        // Get resolved address
        let target_ip = test_result.resolved_address.as_deref().unwrap_or(target);

        // Header line
        output.push_str(&format!(
            "traceroute to {}, 30 hops max, 52 bytes payload, ICMP mode\n",
            target_ip
        ));

        output.push_str(&format!("Probe: {} - {}, {}\n\n",
            probe_info.network,
            probe_info.city.as_deref().unwrap_or("Unknown"),
            probe_info.country
        ));

        // Process hops
        // Globalping API returns hops with resolvedAddress, resolvedHostname, and timings
        if let Some(hops) = &test_result.hops {
            for (hop_num, hop) in hops.iter().enumerate() {
                // Check if hop has resolved address
                if let Some(resolved_address) = &hop.resolved_address {
                    // Get IP info and PTR records
                    let ip_info = if let Ok(client) = ip_info_client {
                        client.get_ip_info(resolved_address).await.ok()
                    } else {
                        None
                    };

                    let ptr_records = doh_client.query_ptr(resolved_address).await.ok();

                    // Format hop information - first line with IP
                    output.push_str(&format!("{:3}   {:15}", hop_num + 1, resolved_address));

                    // ASN and location info on same line
                    if let Some(info) = &ip_info {
                        output.push_str(&format!(
                            "   {:15}  {:20}  {:6}  {:10}  {}\n",
                            info.asn, info.as_name, info.country_code,
                            info.continent_code, info.as_domain
                        ));
                    } else {
                        // No IP info available
                        output.push_str("   *             *                      *           *\n");
                    }

                    // PTR records on next line (indented)
                    if let Some(ptrs) = &ptr_records {
                        if !ptrs.is_empty() {
                            // Take first PTR record
                            output.push_str(&format!("      {:15}\n", ptrs[0]));
                        }
                    }

                    // RTT times on next line (indented)
                    if let Some(timings) = &hop.timings {
                        let times: Vec<String> = timings.iter()
                            .map(|t| format!("{:.2} ms", t.rtt))
                            .collect();

                        if !times.is_empty() {
                            output.push_str(&format!(
                                "                                                {}\n",
                                times.join(" / ")
                            ));
                        } else {
                            output.push_str("                                                *\n");
                        }
                    } else {
                        output.push_str("                                                *\n");
                    }
                } else {
                    // Hop timed out - no IP response
                    output.push_str(&format!("{:3}   *\n", hop_num + 1));
                }
            }
        } else {
            output.push_str("No hops data available in traceroute results\n");
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
    async fn test_traceroute_query_formatting() {
        // This test requires actual API calls
        let result = process_traceroute_query("1.1.1.1-TRACE").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires network and API tokens
    async fn test_traceroute_long_form() {
        // Test long form -TRACEROUTE
        let result = process_traceroute_query("1.1.1.1-TRACEROUTE").await;
        assert!(result.is_ok());
    }
}
