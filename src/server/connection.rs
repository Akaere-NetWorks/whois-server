use std::net::SocketAddr;
use std::time::Duration;

use crate::config::SERVER_BANNER;
use crate::core::{ColorProtocol, ColorScheme, QueryType, StatsState, analyze_query, dump_to_file};
use crate::{log_debug, log_error, log_warn};
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    timeout: Duration,
    dump_traffic: bool,
    dump_dir: &str,
    stats: StatsState,
    enable_color: bool,
) -> Result<()> {
    // Set nodelay to ensure responses are sent immediately
    if let Err(e) = stream.set_nodelay(true) {
        log_warn!("Failed to set TCP_NODELAY: {}", e);
    }

    // Read request
    let mut buffer = [0u8; 8192];
    let mut request = String::with_capacity(512);

    let read_future = async {
        let mut total_read = 0;
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    request.push_str(&String::from_utf8_lossy(&buffer[0..n]));
                    total_read += n;

                    // Check for CRLF terminator
                    if request.contains("\r\n") || total_read > 900 {
                        break;
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to read request: {}", e));
                }
            }
        }
        Ok(())
    };

    // Read with timeout
    if tokio::time::timeout(timeout, read_future).await.is_err() {
        return Err(anyhow::anyhow!("Request read timeout"));
    }

    // Dump query if requested
    if dump_traffic {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        dump_to_file(&format!("{}/query_{}.txt", dump_dir, timestamp), &request);
    }

    // Parse color protocol headers
    let mut color_protocol = ColorProtocol::new();
    color_protocol.enabled = enable_color;
    let is_capability_probe = color_protocol.parse_headers(&request);

    // Handle capability probe
    if is_capability_probe {
        log_debug!("Received WHOIS-COLOR capability probe from {}", addr);
        let capability_response = color_protocol.get_capability_response();

        if let Err(e) = stream.write_all(capability_response.as_bytes()).await {
            log_error!("Failed to send capability response: {}", e);
        } else {
            log_debug!("Sent WHOIS-COLOR capability response");
        }

        return Ok(());
    }

    // Clean request - trim whitespace and get first line (skip headers)
    let query_line = request
        .trim()
        .lines()
        .find(|line| !line.trim().to_uppercase().starts_with("X-WHOIS-COLOR"))
        .unwrap_or("");

    let query = query_line.trim().to_string();

    // Skip empty queries
    if query.is_empty() {
        log_debug!("Received empty query from {}", addr);
        return Ok(());
    }

    log_debug!(
        "Received query from {}: {} (color: {:?})",
        addr,
        query,
        color_protocol.scheme
    );

    // Analyze query type
    let query_type = analyze_query(&query);

    // Process query using the shared query processor
    // This handles colorization, patches, and telemetry internally
    // with proper #[cfg(feature)] gating for all match arms
    let color_scheme = if color_protocol.should_colorize() {
        color_protocol.scheme.clone()
    } else {
        None
    };
    let result = crate::core::process_query(
        &query,
        &query_type,
        color_scheme,
        Some(addr.ip().to_string()),
    )
    .await;

    // Format the response with proper WHOIS format
    let formatted_response = match result {
        Ok(resp) => {
            let mut formatted = format!("{}\r\n", SERVER_BANNER);
            formatted.push_str("% The objects are in RPSL format\r\n");
            formatted.push_str("% Please report any issues to noc@akae.re\r\n");
            formatted.push_str("\r\n");
            formatted.push_str(&resp);

            // Ensure response ends with a CRLF
            if !formatted.ends_with("\r\n") {
                formatted.push_str("\r\n");
            }

            formatted
        }
        Err(e) => {
            log_error!("WHOIS query error for {}: {}", query, e);

            let mut formatted = format!("{}\r\n", SERVER_BANNER);
            formatted.push_str("% Please report any issues to noc@akae.re\r\n");
            formatted.push_str("\r\n");

            let error_msg = format!("% Error: {}\r\n", e);

            // Apply colorization to error message if requested
            let colored_error = if color_protocol.should_colorize() {
                format!("\x1b[91m{}\x1b[0m", error_msg) // Bright red for errors
            } else {
                error_msg
            };

            formatted.push_str(&colored_error);
            formatted.push_str("\r\n");
            formatted
        }
    };

    // Dump response if requested
    if dump_traffic {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        dump_to_file(
            &format!("{}/response_{}.txt", dump_dir, timestamp),
            &formatted_response,
        );
    }

    // Log the response size (helpful for debugging)
    log_debug!(
        "Sending response ({} bytes) for query: {}",
        formatted_response.len(),
        query
    );

    // Send response - use write_all to ensure entire response is sent
    match stream.write_all(formatted_response.as_bytes()).await {
        Ok(_) => {
            // Flush to ensure data is sent
            if let Err(e) = stream.flush().await {
                log_error!("Failed to flush response: {}", e);
            }
            log_debug!("Query response sent: {}", query);

            // Record statistics
            crate::core::record_request(&stats, formatted_response.len()).await;
        }
        Err(e) => {
            log_error!("Failed to send response for {}: {}", query, e);
            return Err(anyhow::anyhow!("Failed to send response: {}", e));
        }
    }

    // According to RFC 3912, the server MUST close the connection, not wait for client
    log_debug!("Closing connection from server side (RFC 3912 requirement)");

    // First shutdown write side to ensure all data is transmitted
    if let Err(e) = stream.shutdown().await {
        log_warn!("Error shutting down connection: {}", e);
    }

    // Drop the stream to forcibly close the connection
    drop(stream);

    Ok(())
}

/// Process a WHOIS query and return the response (for use by SSH server and other modules)
#[allow(dead_code)]
pub async fn handle_query(
    query: &str,
    query_type: &QueryType,
    color_scheme: Option<ColorScheme>,
    client_ip: Option<String>,
) -> Result<String> {
    crate::core::process_query(query, query_type, color_scheme, client_ip).await
}
