use std::time::Duration;
use anyhow::Result;
use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio::net::TcpStream as AsyncTcpStream;
use tracing::{ debug, warn };

use crate::config::{
    DEFAULT_WHOIS_SERVER,
    DEFAULT_WHOIS_PORT,
    TIMEOUT_SECONDS,
    RADB_WHOIS_SERVER,
    RADB_WHOIS_PORT,
};
use crate::services::iana_cache::IanaCache;

pub async fn query_with_iana_referral(query: &str) -> Result<String> {
    debug!("Querying with IANA referral: {}", query);

    // Try to get WHOIS server from cache
    let iana_cache = IanaCache::new()?;
    let whois_server = match iana_cache.get_whois_server(query).await {
        Some(server) => server,
        None => {
            debug!("No IANA referral found for {}, using default server", query);
            DEFAULT_WHOIS_SERVER.to_string()
        }
    };

    debug!("Using WHOIS server: {}", whois_server);

    // Query the WHOIS server
    match query_whois(query, &whois_server, DEFAULT_WHOIS_PORT).await {
        Ok(response) => {
            // Check if response indicates transferred/no data and try RADB fallback
            if should_try_radb_fallback(&response, query) {
                debug!("Primary response suggests transferred resource, trying RADB fallback for: {}", query);
                match query_whois(query, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await {
                    Ok(radb_response) => {
                        if is_meaningful_response(&radb_response, query) {
                            debug!("RADB provided meaningful data for: {}", query);
                            Ok(radb_response)
                        } else {
                            debug!(
                                "RADB response not meaningful, combining with original response"
                            );
                            // Combine both responses with a note about the fallback attempt
                            let combined = format!(
                                "{}\n\n% Additional query attempted via RADB:\n{}\n% End of RADB response\n",
                                response.trim(),
                                radb_response.trim()
                            );
                            Ok(combined)
                        }
                    }
                    Err(e) => {
                        debug!("RADB query failed for {}: {}", query, e);
                        // Add a note about the failed fallback attempt
                        let enhanced = format!(
                            "{}\n\n% Note: RADB fallback attempted but failed: {}\n",
                            response.trim(),
                            e
                        );
                        Ok(enhanced)
                    }
                }
            } else {
                Ok(response)
            }
        }
        Err(e) => {
            warn!("Query failed on {}, attempting to refresh IANA cache: {}", whois_server, e);

            // Query failed, try to refresh IANA cache
            if let Some(refreshed_server) = iana_cache.refresh_cache_on_failure(query).await {
                debug!("Retrying with refreshed server: {}", refreshed_server);
                match query_whois(query, &refreshed_server, DEFAULT_WHOIS_PORT).await {
                    Ok(response) => Ok(response),
                    Err(_) => {
                        // If refreshed server also fails, try RADB as final fallback
                        debug!("Refreshed server failed, trying RADB as final fallback for: {}", query);
                        match query_whois(query, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await {
                            Ok(radb_resp) => Ok(radb_resp),
                            Err(_) =>
                                query_whois(query, DEFAULT_WHOIS_SERVER, DEFAULT_WHOIS_PORT).await,
                        }
                    }
                }
            } else {
                // If refresh also fails, try RADB then default server as last resort
                debug!("IANA refresh failed, trying RADB fallback for: {}", query);
                match query_whois(query, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await {
                    Ok(radb_resp) => Ok(radb_resp),
                    Err(_) => {
                        debug!("RADB failed, trying default server as final fallback");
                        query_whois(query, DEFAULT_WHOIS_SERVER, DEFAULT_WHOIS_PORT).await
                    }
                }
            }
        }
    }
}

pub async fn query_whois(query: &str, server: &str, port: u16) -> Result<String> {
    let address = format!("{}:{}", server, port);
    debug!("Querying WHOIS server: {}", address);

    let timeout = Duration::from_secs(TIMEOUT_SECONDS);

    // Connect to the WHOIS server with timeout
    let connect_future = AsyncTcpStream::connect(&address);
    let mut stream = match tokio::time::timeout(timeout, connect_future).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!("Cannot connect to WHOIS server {}: {}", address, e));
        }
        Err(_) => {
            return Err(anyhow::anyhow!("Connection to WHOIS server timed out: {}", address));
        }
    };

    // Try to disable Nagle's algorithm
    if let Err(e) = stream.set_nodelay(true) {
        warn!("Failed to set TCP_NODELAY: {}", e);
    }

    // Prepare and send the query - WHOIS protocol expects CRLF-terminated query
    let query_str = format!("{}\r\n", query);
    match tokio::time::timeout(timeout, stream.write_all(query_str.as_bytes())).await {
        Ok(Ok(_)) => {
            // Flush to ensure the query is sent immediately
            if let Err(e) = stream.flush().await {
                return Err(anyhow::anyhow!("Failed to flush query to WHOIS server: {}", e));
            }
        }
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!("Failed to write query to WHOIS server: {}", e));
        }
        Err(_) => {
            return Err(anyhow::anyhow!("Query write timed out"));
        }
    }

    // Read the response with timeout
    let mut response = String::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer size

    let read_start = std::time::Instant::now();
    let mut total_bytes = 0;

    // Keep reading until end of stream or timeout
    loop {
        match tokio::time::timeout(timeout, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => {
                break;
            } // End of stream
            Ok(Ok(n)) => {
                response.push_str(&String::from_utf8_lossy(&buffer[0..n]));
                total_bytes += n;

                // Prevent excessively large responses
                if total_bytes > 1_000_000 {
                    // 1MB limit
                    debug!("Response exceeded size limit (1MB), truncating");
                    break;
                }

                // Check if we've been reading for too long
                if read_start.elapsed() > timeout {
                    debug!("Read timeout reached after {} bytes", total_bytes);
                    break;
                }
            }
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Failed to read WHOIS server response: {}", e));
            }
            Err(_) => {
                debug!("Timeout reading WHOIS response after {} bytes", total_bytes);
                break; // Just break on timeout, return what we have so far
            }
        }
    }

    // Log response info for debugging
    debug!("Received {} bytes from {}", total_bytes, address);

    if response.is_empty() {
        return Err(anyhow::anyhow!("Empty response from WHOIS server"));
    }

    Ok(response)
}

fn should_try_radb_fallback(response: &str, query: &str) -> bool {
    let response_lower = response.to_lowercase();

    // Check for indicators that suggest transferred resources or empty responses
    let transfer_indicators = [
        "not managed by the ripe ncc",
        "not managed by ripe ncc",
        "managed by arin",
        "managed by apnic",
        "managed by lacnic",
        "managed by afrinic",
        "transferred",
        "no entries found",
        "not found",
        "no match found",
        "no data found",
        "% no entries found",
        "% not found",
        "asn block not managed",
        "ip block not managed",
        "for registration information",
        "you can find the whois server to query",
    ];

    // Check if the response is very short (likely just headers)
    let meaningful_lines: Vec<&str> = response
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('%'))
        .collect();

    if meaningful_lines.len() < 3 {
        debug!(
            "Response has very few meaningful lines ({}), suggesting RADB fallback",
            meaningful_lines.len()
        );
        return true;
    }

    // Check if response lacks routing-specific information (suggests network registry vs routing registry)
    let has_routing_info =
        response_lower.contains("route:") ||
        response_lower.contains("descr:") ||
        response_lower.contains("origin:") ||
        response_lower.contains("as-path:") ||
        response_lower.contains("source:") ||
        response_lower.contains("remarks:");

    // If query looks like a CIDR block but response has no routing info, try RADB
    if (query.contains('/') || query.contains('-')) && !has_routing_info {
        debug!(
            "Query appears to be for routing info but response lacks routing fields, suggesting RADB fallback"
        );
        return true;
    }

    // Check for transfer indicators
    for indicator in &transfer_indicators {
        if response_lower.contains(indicator) {
            debug!("Found transfer indicator '{}', suggesting RADB fallback", indicator);
            return true;
        }
    }

    false
}

fn is_meaningful_response(response: &str, query: &str) -> bool {
    let meaningful_lines: Vec<&str> = response
        .lines()
        .filter(|line| {
            let line = line.trim();
            // Skip comments, empty lines, and generic headers
            !line.is_empty() &&
                !line.starts_with('%') &&
                !line.starts_with('#') &&
                !line.contains("Please report any issues") &&
                !line.contains("The objects are in RPSL format")
        })
        .collect();

    // Consider response meaningful if it has substantive content
    meaningful_lines.len() >= 5 &&
        response.len() > 200 && // At least 200 characters of content
        !should_try_radb_fallback(response, query) // And doesn't look like a transfer notice
}
