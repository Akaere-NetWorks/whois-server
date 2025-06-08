use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener as AsyncTcpListener, TcpStream as AsyncTcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::bgptool::{process_bgptool_query, process_bgptool_query_blocking};
use crate::config::{SERVER_BANNER, DN42_WHOIS_SERVER, DN42_WHOIS_PORT};
use crate::email::{process_email_search, process_email_search_blocking};
use crate::geo::{process_geo_query, process_geo_query_blocking};
use crate::query::{analyze_query, is_private_ipv4, is_private_ipv6, QueryType};
use crate::utils::dump_to_file;
use crate::whois::{
    blocking_query_whois, blocking_query_with_iana_referral,
    query_whois, query_with_iana_referral,
};

pub async fn run_async_server(
    addr: &str,
    max_connections: usize,
    timeout: u64,
    dump_traffic: bool,
    dump_dir: &str,
) -> Result<()> {
    // Start server
    let listener = AsyncTcpListener::bind(&addr).await
        .context(format!("Failed to bind to {}", addr))?;
    
    let (tx, mut rx) = mpsc::channel::<()>(max_connections);

    // Handle connections
    loop {
        tokio::select! {
            _ = rx.recv() => {
                // A connection completed, continue accepting new connections
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        info!("Accepted connection from {}", addr);
                        let tx_clone = tx.clone();
                        
                        // Set timeout
                        let timeout = Duration::from_secs(timeout);
                        let dump_traffic = dump_traffic;
                        let dump_dir = dump_dir.to_string();
                        
                        // Handle connection
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, addr, timeout, dump_traffic, &dump_dir).await {
                                error!("Connection handling error: {}", e);
                            }
                            
                            // Notify completion
                            let _ = tx_clone.send(()).await;
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        }
    }
}

// Blocking TCP server implementation for testing
pub fn run_blocking_server(addr: &str, timeout_secs: u64, dump_traffic: bool, dump_dir: &str) -> Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(false)?;
    
    info!("Blocking server listening on {}", addr);
    
    let timeout = Duration::from_secs(timeout_secs);
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Set timeout
                stream.set_read_timeout(Some(timeout))?;
                stream.set_write_timeout(Some(timeout))?;
                
                info!("Blocking mode: accepted connection from {}", stream.peer_addr()?);
                
                // Read query
                let mut buffer = [0u8; 1024];
                let mut request = String::new();
                
                // Read until CRLF or buffer is full
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            request.push_str(&String::from_utf8_lossy(&buffer[0..n]));
                            if request.contains("\r\n") || request.len() > 900 {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to read request: {}", e);
                            break;
                        }
                    }
                }
                
                // Clean request - trim whitespace and get first line
                let query = match request.trim().lines().next() {
                    Some(q) => q.trim().to_string(),
                    None => {
                        error!("Empty query received");
                        continue;
                    }
                };
                
                // Skip empty queries
                if query.is_empty() {
                    debug!("Received empty query");
                    continue;
                }
                
                info!("Blocking mode: received query: {}", query);
                
                // Dump query if requested
                if dump_traffic {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    dump_to_file(&format!("{}/query_{}.txt", dump_dir, timestamp), &request);
                }
                
                // Analyze query type and select appropriate WHOIS server
                let query_type = analyze_query(&query);
                let response = match &query_type {
                    QueryType::Domain(domain) => {
                        info!("Processing domain query: {}", domain);
                        if domain.to_lowercase().ends_with(".dn42") {
                            info!("Detected .dn42 domain, using DN42 query");
                            blocking_query_whois(domain, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                        } else {
                            blocking_query_with_iana_referral(domain, timeout)
                        }
                    }
                    QueryType::IPv4(ip) => {
                        info!("Processing IPv4 query: {}", ip);
                        if is_private_ipv4(*ip) {
                            info!("Detected private IPv4 address, using DN42 query");
                            blocking_query_whois(&query, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                        } else {
                            blocking_query_with_iana_referral(&query, timeout)
                        }
                    }
                    QueryType::IPv6(ip) => {
                        info!("Processing IPv6 query: {}", ip);
                        if is_private_ipv6(*ip) {
                            info!("Detected private IPv6 address, using DN42 query");
                            blocking_query_whois(&query, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                        } else {
                            blocking_query_with_iana_referral(&query, timeout)
                        }
                    }
                    QueryType::ASN(asn) => {
                        info!("Processing ASN query: {}", asn);
                        if asn.to_uppercase().starts_with("AS42424") {
                            info!("Detected DN42 ASN, using DN42 query");
                            blocking_query_whois(asn, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                        } else {
                            blocking_query_with_iana_referral(asn, timeout)
                        }
                    }
                    QueryType::EmailSearch(base_query) => {
                        info!("Processing email search query: {}", base_query);
                        process_email_search_blocking(base_query, timeout)
                    }
                    QueryType::BGPTool(base_query) => {
                        info!("Processing BGP Tools query: {}", base_query);
                        process_bgptool_query_blocking(base_query, timeout)
                    }
                    QueryType::Geo(resource) => {
                        info!("Processing geo location query: {}", resource);
                        process_geo_query_blocking(resource, timeout)
                    }
                    QueryType::Unknown(q) => {
                        info!("Unknown query type: {}", q);
                        if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                            info!("Detected DN42 related query ({}), using DN42 query", q);
                            blocking_query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                        } else {
                            let public_result = blocking_query_with_iana_referral(q, timeout);
                            
                            match &public_result {
                                Ok(response) if response.trim().is_empty() 
                                    || response.contains("No entries found") 
                                    || response.contains("Not found") => {
                                    info!("Public query returned no results, trying DN42 for: {}", q);
                                    blocking_query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                                },
                                Err(_) => {
                                    info!("Public query failed, trying DN42 for: {}", q);
                                    blocking_query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT, timeout)
                                },
                                _ => public_result,
                            }
                        }
                    }
                };
                
                // Format and send response
                match response {
                    Ok(resp) => {
                        let formatted = format!("{}\r\n% The objects are in RPSL format\r\n% Please report any issues to noc@akae.re\r\n\r\n{}\r\n", 
                                               SERVER_BANNER, resp);
                        
                        // Dump response if requested
                        if dump_traffic {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis();
                            dump_to_file(&format!("{}/response_{}.txt", dump_dir, timestamp), &formatted);
                        }
                        
                        info!("Sending response ({} bytes)", formatted.len());
                        
                        // Set TCP_NODELAY to disable Nagle's algorithm
                        if let Err(e) = stream.set_nodelay(true) {
                            warn!("Failed to set TCP_NODELAY: {}", e);
                        }
                        
                        match stream.write_all(formatted.as_bytes()) {
                            Ok(_) => {
                                if let Err(e) = stream.flush() {
                                    error!("Failed to flush response: {}", e);
                                }
                                info!("Response sent successfully");
                            }
                            Err(e) => error!("Failed to send response: {}", e),
                        }
                    }
                    Err(e) => {
                        error!("WHOIS query error: {}", e);
                        let error_msg = format!("{}\r\n% Please report any issues to noc@akae.re\r\n\r\n% Error: {}\r\n\r\n", 
                                               SERVER_BANNER, e);
                        
                        if let Err(write_err) = stream.write_all(error_msg.as_bytes()) {
                            error!("Failed to send error message: {}", write_err);
                        }
                    }
                }
                
                // According to RFC 3912, the server MUST close the connection
                info!("Closing connection from server side (RFC 3912 requirement)");
                if let Err(e) = stream.shutdown(std::net::Shutdown::Both) {
                    warn!("Error shutting down connection: {}", e);
                }
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn handle_connection(
    mut stream: AsyncTcpStream,
    addr: SocketAddr,
    timeout: Duration,
    dump_traffic: bool,
    dump_dir: &str,
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
                query_whois(domain, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
            } else {
                query_with_iana_referral(domain).await
            }
        }
        QueryType::IPv4(ip) => {
            debug!("Processing IPv4 query: {}", ip);
            if is_private_ipv4(*ip) {
                debug!("Detected private IPv4 address, using DN42 query");
                query_whois(&query, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::IPv6(ip) => {
            debug!("Processing IPv6 query: {}", ip);
            if is_private_ipv6(*ip) {
                debug!("Detected private IPv6 address, using DN42 query");
                query_whois(&query, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::ASN(asn) => {
            debug!("Processing ASN query: {}", asn);
            if asn.to_uppercase().starts_with("AS42424") {
                debug!("Detected DN42 ASN, using DN42 query");
                query_whois(asn, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
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
        QueryType::Unknown(q) => {
            debug!("Unknown query type: {}", q);
            if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                debug!("Detected DN42 related query ({}), using DN42 query", q);
                query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
            } else {
                let public_result = query_with_iana_referral(q).await;
                
                match &public_result {
                    Ok(response) if response.trim().is_empty() 
                        || response.contains("No entries found") 
                        || response.contains("Not found") => {
                        debug!("Public query returned no results, trying DN42 for: {}", q);
                        query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
                    },
                    Err(_) => {
                        debug!("Public query failed, trying DN42 for: {}", q);
                        query_whois(q, DN42_WHOIS_SERVER, DN42_WHOIS_PORT).await
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

pub fn create_dump_dir_if_needed(dump_traffic: bool, dump_dir: &str) -> Result<()> {
    if dump_traffic {
        let path = Path::new(dump_dir);
        if !path.exists() {
            info!("Creating dumps directory: {}", dump_dir);
            std::fs::create_dir_all(path).context("Failed to create dumps directory")?;
        }
    }
    Ok(())
} 