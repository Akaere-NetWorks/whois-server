/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2024 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use cidr::{Ipv4Cidr, Ipv6Cidr};
use clap::Parser;
use regex::Regex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;

// WHOIS server constants
const IANA_WHOIS_SERVER: &str = "whois.iana.org";
const DEFAULT_WHOIS_SERVER: &str = "whois.ripe.net";
const DEFAULT_WHOIS_PORT: u16 = 43;
const TIMEOUT_SECONDS: u64 = 10;
const DN42_WHOIS_SERVER: &str = "lantian.pub";
const DN42_WHOIS_PORT: u16 = 43;

// Server identification banner
const SERVER_BANNER: &str = "% Akaere NetWorks Whois Server";

// Private IP range definitions
const PRIVATE_IPV4_RANGES: &[&str] = &[
    "10.0.0.0/8",     // RFC1918
    "172.16.0.0/12",  // RFC1918
    "192.168.0.0/16", // RFC1918
    "169.254.0.0/16", // Link-local addresses
    "192.0.2.0/24",   // Documentation examples (TEST-NET-1)
    "198.51.100.0/24", // Documentation examples (TEST-NET-2)
    "203.0.113.0/24", // Documentation examples (TEST-NET-3)
    "100.64.0.0/10",  // CGNAT (Carrier-grade NAT)
    "127.0.0.0/8",    // Localhost
];

const PRIVATE_IPV6_RANGES: &[&str] = &[
    "fc00::/7",       // Unique Local Addresses
    "fd00::/8",       // Unique Local Addresses (subset)
    "fe80::/10",      // Link-local addresses
    "::1/128",        // Localhost
    "2001:db8::/32",  // Documentation addresses
];

#[derive(Parser)]
#[command(author, version, about = "A simple WHOIS server")]
struct Cli {
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,

    /// Listen port
    #[arg(short, long, default_value_t = 43)]
    port: u16,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
    
    /// Enable trace output (extremely verbose)
    #[arg(short, long)]
    trace: bool,
    
    /// Maximum concurrent connections
    #[arg(long, default_value_t = 100)]
    max_connections: usize,
    
    /// Connection timeout in seconds
    #[arg(long, default_value_t = 10)]
    timeout: u64,
    
    /// Write raw queries and responses to files for debugging
    #[arg(long)]
    dump_traffic: bool,
    
    /// Dump traffic directory (default: ./dumps)
    #[arg(long, default_value = "dumps")]
    dump_dir: String,
    
    /// Use blocking (non-async) network operations
    #[arg(long)]
    use_blocking: bool,
}

// WHOIS query types
enum QueryType {
    Domain(String),
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    ASN(String),
    Unknown(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    
    // Initialize logging
    let log_level = if args.trace {
        Level::TRACE
    } else if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_span_events(FmtSpan::CLOSE)
        .init();
    
    // Create dump directory if needed
    if args.dump_traffic {
        let path = Path::new(&args.dump_dir);
        if !path.exists() {
            info!("Creating dumps directory: {}", args.dump_dir);
            std::fs::create_dir_all(path).context("Failed to create dumps directory")?;
        }
    }
    
    // Create server address
    let addr = format!("{}:{}", args.host, args.port);
    info!("Starting WHOIS server on {}", addr);
    
    if args.use_blocking {
        info!("Using blocking TCP connections (non-async)");
        run_blocking_server(&addr, args.timeout, args.dump_traffic, &args.dump_dir)?;
        return Ok(());
    }
    
    // Start server
    let listener = AsyncTcpListener::bind(&addr).await
        .context(format!("Failed to bind to {}", addr))?;
    
    let (tx, mut rx) = mpsc::channel::<()>(args.max_connections);

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
                        let timeout = Duration::from_secs(args.timeout);
                        let dump_traffic = args.dump_traffic;
                        let dump_dir = args.dump_dir.clone();
                        
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
fn run_blocking_server(addr: &str, timeout_secs: u64, dump_traffic: bool, dump_dir: &str) -> Result<()> {
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

fn blocking_query_with_iana_referral(query: &str, timeout: Duration) -> Result<String> {
    info!("First querying IANA: {}", query);
    
    // First query IANA
    let iana_response = blocking_query_whois(query, IANA_WHOIS_SERVER, DEFAULT_WHOIS_PORT, timeout)?;
    
    // Extract WHOIS server from IANA response
    let whois_server = extract_whois_server(&iana_response)
        .unwrap_or_else(|| DEFAULT_WHOIS_SERVER.to_string());
    
    info!("IANA referred server: {}", whois_server);
    
    // Query the actual WHOIS server
    let response = blocking_query_whois(query, &whois_server, DEFAULT_WHOIS_PORT, timeout)?;
    
    Ok(response)
}

fn blocking_query_whois(query: &str, server: &str, port: u16, timeout: Duration) -> Result<String> {
    let address = format!("{}:{}", server, port);
    info!("Querying WHOIS server: {}", address);
    
    // Connect to the WHOIS server with timeout
    let mut stream = TcpStream::connect_timeout(&address.parse()?, timeout)
        .context(format!("Cannot connect to WHOIS server {}", address))?;
    
    // Set read/write timeouts
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    
    // Try to disable Nagle's algorithm
    if let Err(e) = stream.set_nodelay(true) {
        warn!("Failed to set TCP_NODELAY: {}", e);
    }
    
    // Prepare and send the query - WHOIS protocol expects CRLF-terminated query
    let query_str = format!("{}\r\n", query);
    stream.write_all(query_str.as_bytes())?;
    stream.flush()?;
    
    // Read the response
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    
    info!("Received {} bytes from {}", response.len(), address);
    
    if response.is_empty() {
        return Err(anyhow::anyhow!("Empty response from WHOIS server"));
    }
    
    Ok(response)
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

// Helper function to dump content to a file
fn dump_to_file(filename: &str, content: &str) {
    match File::create(filename) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content.as_bytes()) {
                error!("Failed to write to dump file {}: {}", filename, e);
            } else {
                debug!("Wrote {} bytes to {}", content.len(), filename);
            }
        },
        Err(e) => error!("Failed to create dump file {}: {}", filename, e),
    }
}

fn analyze_query(query: &str) -> QueryType {
    // Check if it's a .dn42 domain
    if query.to_lowercase().ends_with(".dn42") {
        return QueryType::Domain(query.to_string());
    }
    
    // Check if it has -DN42 suffix or ends with -MNT
    if query.to_uppercase().ends_with("-DN42") || query.to_uppercase().ends_with("-MNT") {
        return QueryType::Unknown(query.to_string());
    }
    
    // Try to parse as IP address
    if let Ok(ip) = query.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => return QueryType::IPv4(ipv4),
            IpAddr::V6(ipv6) => return QueryType::IPv6(ipv6),
        }
    }
    
    // Try to parse as CIDR format
    if let Ok(cidr) = query.parse::<Ipv4Cidr>() {
        return QueryType::IPv4(cidr.first_address());
    }
    
    if let Ok(cidr) = query.parse::<Ipv6Cidr>() {
        return QueryType::IPv6(cidr.first_address());
    }
    
    // Identify ASN
    if query.to_uppercase().starts_with("AS") && query[2..].chars().all(|c| c.is_digit(10)) {
        return QueryType::ASN(query.to_string());
    }
    
    // Check if it's a domain format
    let domain_regex = Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$").unwrap();
    if domain_regex.is_match(query) {
        return QueryType::Domain(query.to_string());
    }
    
    // Default to unknown type
    QueryType::Unknown(query.to_string())
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    for range_str in PRIVATE_IPV4_RANGES {
        if let Ok(range) = range_str.parse::<Ipv4Cidr>() {
            if range.contains(&ip) {
                return true;
            }
        }
    }
    false
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    for range_str in PRIVATE_IPV6_RANGES {
        if let Ok(range) = range_str.parse::<Ipv6Cidr>() {
            if range.contains(&ip) {
                return true;
            }
        }
    }
    false
}

async fn query_with_iana_referral(query: &str) -> Result<String> {
    debug!("First querying IANA: {}", query);
    
    // First query IANA
    let iana_response = query_whois(query, IANA_WHOIS_SERVER, DEFAULT_WHOIS_PORT).await?;
    
    // Extract WHOIS server from IANA response
    let whois_server = extract_whois_server(&iana_response)
        .unwrap_or_else(|| DEFAULT_WHOIS_SERVER.to_string());
    
    debug!("IANA referred server: {}", whois_server);
    
    // Query the actual WHOIS server
    let response = query_whois(query, &whois_server, DEFAULT_WHOIS_PORT).await?;
    
    Ok(response)
}

async fn query_whois(query: &str, server: &str, port: u16) -> Result<String> {
    let address = format!("{}:{}", server, port);
    debug!("Querying WHOIS server: {}", address);
    
    let timeout = Duration::from_secs(TIMEOUT_SECONDS);
    
    // Connect to the WHOIS server with timeout
    let connect_future = AsyncTcpStream::connect(&address);
    let mut stream = match tokio::time::timeout(timeout, connect_future).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => return Err(anyhow::anyhow!("Cannot connect to WHOIS server {}: {}", address, e)),
        Err(_) => return Err(anyhow::anyhow!("Connection to WHOIS server timed out: {}", address)),
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
        },
        Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to write query to WHOIS server: {}", e)),
        Err(_) => return Err(anyhow::anyhow!("Query write timed out")),
    }
    
    // Read the response with timeout
    let mut response = String::new();
    let mut buffer = [0u8; 8192];  // 8KB buffer size
    
    let read_start = std::time::Instant::now();
    let mut total_bytes = 0;
    
    // Keep reading until end of stream or timeout
    loop {
        match tokio::time::timeout(timeout, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => break,  // End of stream
            Ok(Ok(n)) => {
                response.push_str(&String::from_utf8_lossy(&buffer[0..n]));
                total_bytes += n;
                
                // Prevent excessively large responses
                if total_bytes > 1_000_000 {  // 1MB limit
                    debug!("Response exceeded size limit (1MB), truncating");
                    break;
                }
                
                // Check if we've been reading for too long
                if read_start.elapsed() > timeout {
                    debug!("Read timeout reached after {} bytes", total_bytes);
                    break;
                }
            },
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to read WHOIS server response: {}", e)),
            Err(_) => {
                debug!("Timeout reading WHOIS response after {} bytes", total_bytes);
                break; // Just break on timeout, return what we have so far
            },
        }
    }
    
    // Log response info for debugging
    debug!("Received {} bytes from {}", total_bytes, address);
    
    if response.is_empty() {
        return Err(anyhow::anyhow!("Empty response from WHOIS server"));
    }
    
    Ok(response)
}

fn extract_whois_server(response: &str) -> Option<String> {
    for line in response.lines() {
        let line = line.trim();
        
        // Look for "whois:" field
        if line.starts_with("whois:") {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().to_string());
            }
        }
        
        // Also look for "refer:" field as a fallback
        if line.starts_with("refer:") {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}
