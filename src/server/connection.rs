use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, warn};

use crate::bgptool::process_bgptool_query;
use crate::config::{SERVER_BANNER, RADB_WHOIS_SERVER, RADB_WHOIS_PORT};
use crate::dn42::process_dn42_query;
use crate::email::process_email_search;
use crate::geo::{process_geo_query, process_rir_geo_query, process_prefixes_query};
use crate::irr::process_irr_query;
use crate::looking_glass::process_looking_glass_query;
use crate::query::{analyze_query, is_private_ipv4, is_private_ipv6, QueryType};
use crate::rpki::process_rpki_query;
use crate::utils::dump_to_file;
use crate::whois::{query_whois, query_with_iana_referral};
use crate::stats::StatsState;

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    timeout: Duration,
    dump_traffic: bool,
    dump_dir: &str,
    stats: StatsState,
) -> Result<()> {
    // Set nodelay to ensure responses are sent immediately
    if let Err(e) = stream.set_nodelay(true) {
        warn!("Failed to set TCP_NODELAY: {}", e);
    }
    
    // Read request
    let mut buffer = [0u8; 1024];
    let mut request = String::new();
    
    let read_future = async {
        let mut total_read = 0;
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    request.push_str(&String::from_utf8_lossy(&buffer[0..n]));
                    total_read += n;
                    
                    // Check for CRLF terminator
                    if request.contains("\r\n") || total_read > 900 {
                        break;
                    }
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to read request: {}", e)),
            }
        }
        Ok(())
    };
    
    // Read with timeout
    if let Err(_) = tokio::time::timeout(timeout, read_future).await {
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
    
    // Clean request - trim whitespace and get first line
    let query = match request.trim().lines().next() {
        Some(q) => q.trim().to_string(),
        None => return Err(anyhow::anyhow!("Empty query")),
    };
    
    // Skip empty queries
    if query.is_empty() {
        debug!("Received empty query from {}", addr);
        return Ok(());
    }
    
    debug!("Received query from {}: {}", addr, query);
    
    // Analyze query type
    let query_type = analyze_query(&query);
    
    // Select appropriate WHOIS server and query
    let result = match &query_type {
        QueryType::Domain(domain) => {
            debug!("Processing domain query: {}", domain);
            if domain.to_lowercase().ends_with(".dn42") {
                debug!("Detected .dn42 domain, using DN42 query");
                process_dn42_query(domain).await
            } else {
                query_with_iana_referral(domain).await
            }
        }
        QueryType::IPv4(ip) => {
            debug!("Processing IPv4 query: {}", ip);
            if is_private_ipv4(*ip) {
                debug!("Detected private IPv4 address, using DN42 query");
                process_dn42_query(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::IPv6(ip) => {
            debug!("Processing IPv6 query: {}", ip);
            if is_private_ipv6(*ip) {
                debug!("Detected private IPv6 address, using DN42 query");
                process_dn42_query(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::ASN(asn) => {
            debug!("Processing ASN query: {}", asn);
            if asn.to_uppercase().starts_with("AS42424") {
                debug!("Detected DN42 ASN, using DN42 query");
                process_dn42_query(asn).await
            } else {
                query_with_iana_referral(asn).await
            }
        }
        QueryType::EmailSearch(base_query) => {
            debug!("Processing email search query: {}", base_query);
            process_email_search(base_query).await
        }
        QueryType::BGPTool(base_query) => {
            debug!("Processing BGP Tools query: {}", base_query);
            process_bgptool_query(base_query).await
        }
        QueryType::Geo(resource) => {
            debug!("Processing geo location query: {}", resource);
            process_geo_query(resource).await
        }
        QueryType::RirGeo(resource) => {
            debug!("Processing RIR geo location query: {}", resource);
            process_rir_geo_query(resource).await
        }
        QueryType::Prefixes(asn) => {
            debug!("Processing ASN prefixes query: {}", asn);
            process_prefixes_query(asn).await
        }
        QueryType::Radb(resource) => {
            debug!("Processing RADB query: {}", resource);
            query_whois(resource, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await
        }
        QueryType::Irr(resource) => {
            debug!("Processing IRR Explorer query: {}", resource);
            process_irr_query(resource).await
        }
        QueryType::LookingGlass(resource) => {
            debug!("Processing Looking Glass query: {}", resource);
            process_looking_glass_query(resource).await
        }
        QueryType::Rpki(prefix, asn) => {
            debug!("Processing RPKI query: prefix={}, asn={}", prefix, asn);
            process_rpki_query(prefix, asn).await
        }
        QueryType::Unknown(q) => {
            debug!("Unknown query type: {}", q);
            if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                debug!("Detected DN42 related query ({}), using DN42 query", q);
                process_dn42_query(q).await
            } else {
                let public_result = query_with_iana_referral(q).await;
                
                match &public_result {
                    Ok(response) if response.trim().is_empty() 
                        || response.contains("No entries found") 
                        || response.contains("Not found") => {
                        debug!("Public query returned no results, trying DN42 for: {}", q);
                        process_dn42_query(q).await
                    },
                    Err(_) => {
                        debug!("Public query failed, trying DN42 for: {}", q);
                        process_dn42_query(q).await
                    },
                    _ => public_result,
                }
            }
        }
    };
    
    // Format the response with proper WHOIS format
    let formatted_response = match result {
        Ok(resp) => {
            let mut formatted = format!("{}\r\n", SERVER_BANNER);
            formatted.push_str("% The objects are in RPSL format\r\n");
            formatted.push_str("% Please report any issues to noc@akae.re\r\n");
            formatted.push_str("\r\n");
            
            // Add the actual response content
            formatted.push_str(&resp);
            
            // Ensure response ends with a CRLF
            if !formatted.ends_with("\r\n") {
                formatted.push_str("\r\n");
            }
            
            formatted
        },
        Err(e) => {
            error!("WHOIS query error for {}: {}", query, e);
            
            let mut formatted = format!("{}\r\n", SERVER_BANNER);
            formatted.push_str("% Please report any issues to noc@akae.re\r\n");
            formatted.push_str("\r\n");
            formatted.push_str(&format!("% Error: {}\r\n", e));
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
        dump_to_file(&format!("{}/response_{}.txt", dump_dir, timestamp), &formatted_response);
    }
    
    // Log the response size (helpful for debugging)
    debug!("Sending response ({} bytes) for query: {}", formatted_response.len(), query);
    
    // Send response - use write_all to ensure entire response is sent
    match stream.write_all(formatted_response.as_bytes()).await {
        Ok(_) => {
            // Flush to ensure data is sent
            if let Err(e) = stream.flush().await {
                error!("Failed to flush response: {}", e);
            }
            debug!("Query response sent: {}", query);
            
            // Record statistics
            crate::stats::record_request(&stats, formatted_response.len()).await;
        },
        Err(e) => {
            error!("Failed to send response for {}: {}", query, e);
            return Err(anyhow::anyhow!("Failed to send response: {}", e));
        }
    }
    
    // According to RFC 3912, the server MUST close the connection, not wait for client
    debug!("Closing connection from server side (RFC 3912 requirement)");
    
    // First shutdown write side to ensure all data is transmitted
    if let Err(e) = stream.shutdown().await {
        warn!("Error shutting down connection: {}", e);
    }
    
    // Drop the stream to forcibly close the connection
    drop(stream);
    
    Ok(())
} 