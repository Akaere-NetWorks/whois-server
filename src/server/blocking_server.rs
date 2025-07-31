use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::services::{
    process_bgptool_query_blocking, process_email_search_blocking, 
    process_geo_query_blocking, process_rir_geo_query_blocking, process_prefixes_query_blocking,
    process_irr_query_blocking, process_looking_glass_query_blocking, process_manrs_query_blocking,
    process_rpki_query_blocking, blocking_query_whois, blocking_query_with_iana_referral
};
use crate::config::{SERVER_BANNER, RADB_WHOIS_SERVER, RADB_WHOIS_PORT};
use crate::dn42::process_dn42_query_managed_blocking;
use crate::core::{analyze_query, is_private_ipv4, is_private_ipv6, QueryType, dump_to_file};

// Blocking TCP server implementation for testing
pub fn run_blocking_server(addr: &str, timeout_secs: u64, dump_traffic: bool, dump_dir: &str, _enable_color: bool) -> Result<()> {
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
                            process_dn42_query_managed_blocking(domain)
                        } else {
                            blocking_query_with_iana_referral(domain, timeout)
                        }
                    }
                    QueryType::IPv4(ip) => {
                        info!("Processing IPv4 query: {}", ip);
                        if is_private_ipv4(*ip) {
                            info!("Detected private IPv4 address, using DN42 query");
                            process_dn42_query_managed_blocking(&query)
                        } else {
                            blocking_query_with_iana_referral(&query, timeout)
                        }
                    }
                    QueryType::IPv6(ip) => {
                        info!("Processing IPv6 query: {}", ip);
                        if is_private_ipv6(*ip) {
                            info!("Detected private IPv6 address, using DN42 query");
                            process_dn42_query_managed_blocking(&query)
                        } else {
                            blocking_query_with_iana_referral(&query, timeout)
                        }
                    }
                    QueryType::ASN(asn) => {
                        info!("Processing ASN query: {}", asn);
                        if asn.to_uppercase().starts_with("AS42424") {
                            info!("Detected DN42 ASN, using DN42 query");
                            process_dn42_query_managed_blocking(asn)
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
                    QueryType::RirGeo(resource) => {
                        info!("Processing RIR geo location query: {}", resource);
                        process_rir_geo_query_blocking(resource, timeout)
                    }
                    QueryType::Prefixes(asn) => {
                        info!("Processing ASN prefixes query: {}", asn);
                        process_prefixes_query_blocking(asn, timeout)
                    }
                    QueryType::Radb(resource) => {
                        info!("Processing RADB query: {}", resource);
                        blocking_query_whois(resource, RADB_WHOIS_SERVER, RADB_WHOIS_PORT, timeout)
                    }
                    QueryType::Irr(resource) => {
                        info!("Processing IRR Explorer query: {}", resource);
                        process_irr_query_blocking(resource, timeout)
                    }
                    QueryType::LookingGlass(resource) => {
                        info!("Processing Looking Glass query: {}", resource);
                        process_looking_glass_query_blocking(resource, timeout)
                    }
                    QueryType::Rpki(prefix, asn) => {
                        info!("Processing RPKI query: prefix={}, asn={}", prefix, asn);
                        process_rpki_query_blocking(prefix, asn, timeout)
                    }
                    QueryType::Manrs(base_query) => {
                        info!("Processing MANRS query: {}", base_query);
                        process_manrs_query_blocking(&format!("{}-MANRS", base_query))
                    }
                    QueryType::Dns(base_query) => {
                        info!("Processing DNS query: {}", base_query);
                        // For blocking server, we need to use a blocking implementation
                        // For now, return a notice that DNS queries require async server
                        Ok(format!("DNS queries are only supported on the async server.\nPlease use the main server (port 43) for DNS lookups.\nQuery: {}\n", base_query))
                    }
                    QueryType::Trace(base_query) => {
                        info!("Processing traceroute query: {}", base_query);
                        // Traceroute requires async socket operations for proper timeout handling
                        // Return a notice that traceroute queries require async server
                        Ok(format!("Traceroute queries are only supported on the async server.\nPlease use the main server (port 43) for traceroute.\nQuery: {}\n", base_query))
                    }
                    QueryType::Ssl(base_query) => {
                        info!("Processing SSL certificate query: {}", base_query);
                        // SSL queries require async socket operations
                        // Return a notice that SSL queries require async server
                        Ok(format!("SSL certificate queries are only supported on the async server.\nPlease use the main server (port 43) for SSL queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Crt(base_query) => {
                        info!("Processing Certificate Transparency query: {}", base_query);
                        // CRT queries require async HTTP operations
                        // Return a notice that CRT queries require async server
                        Ok(format!("Certificate Transparency queries are only supported on the async server.\nPlease use the main server (port 43) for CRT queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Minecraft(base_query) => {
                        info!("Processing Minecraft server query: {}", base_query);
                        // Minecraft queries require async TCP operations
                        // Return a notice that Minecraft queries require async server
                        Ok(format!("Minecraft server queries are only supported on the async server.\nPlease use the main server (port 43) for Minecraft queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::MinecraftUser(base_query) => {
                        info!("Processing Minecraft user query: {}", base_query);
                        // Minecraft user queries require async HTTP operations
                        // Return a notice that Minecraft user queries require async server
                        Ok(format!("Minecraft user queries are only supported on the async server.\nPlease use the main server (port 43) for Minecraft user queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Steam(base_query) => {
                        info!("Processing Steam game/user query: {}", base_query);
                        // Steam API queries require async HTTP operations
                        // Return a notice that Steam queries require async server
                        Ok(format!("Steam game/user queries are only supported on the async server.\nPlease use the main server (port 43) for Steam queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::SteamSearch(base_query) => {
                        info!("Processing Steam game search query: {}", base_query);
                        // Steam search queries require async HTTP operations
                        // Return a notice that Steam search queries require async server
                        Ok(format!("Steam game search queries are only supported on the async server.\nPlease use the main server (port 43) for Steam search queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Imdb(base_query) => {
                        info!("Processing IMDb movie/TV show query: {}", base_query);
                        // IMDb queries require async HTTP operations
                        // Return a notice that IMDb queries require async server
                        Ok(format!("IMDb movie/TV show queries are only supported on the async server.\nPlease use the main server (port 43) for IMDb queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::ImdbSearch(base_query) => {
                        info!("Processing IMDb search query: {}", base_query);
                        // IMDb search queries require async HTTP operations
                        // Return a notice that IMDb search queries require async server
                        Ok(format!("IMDb search queries are only supported on the async server.\nPlease use the main server (port 43) for IMDb search queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Acgc(base_query) => {
                        info!("Processing ACGC character query: {}", base_query);
                        // ACGC queries require async HTTP operations
                        // Return a notice that ACGC queries require async server
                        Ok(format!("ACGC character queries are only supported on the async server.\nPlease use the main server (port 43) for ACGC queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Aosc(base_query) => {
                        info!("Processing AOSC package query: {}", base_query);
                        // AOSC queries require async HTTP operations
                        // Return a notice that AOSC queries require async server
                        Ok(format!("AOSC package queries are only supported on the async server.\nPlease use the main server (port 43) for AOSC queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Aur(base_query) => {
                        info!("Processing AUR package query: {}", base_query);
                        // AUR queries require async HTTP operations
                        // Return a notice that AUR queries require async server
                        Ok(format!("AUR package queries are only supported on the async server.\nPlease use the main server (port 43) for AUR queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Debian(base_query) => {
                        info!("Processing Debian package query: {}", base_query);
                        // Debian queries require async HTTP operations
                        // Return a notice that Debian queries require async server
                        Ok(format!("Debian package queries are only supported on the async server.\nPlease use the main server (port 43) for Debian queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Ubuntu(base_query) => {
                        info!("Processing Ubuntu package query: {}", base_query);
                        // Ubuntu queries require async HTTP operations
                        // Return a notice that Ubuntu queries require async server
                        Ok(format!("Ubuntu package queries are only supported on the async server.\nPlease use the main server (port 43) for Ubuntu queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::NixOs(base_query) => {
                        info!("Processing NixOS package query: {}", base_query);
                        // NixOS queries require async HTTP operations
                        // Return a notice that NixOS queries require async server
                        Ok(format!("NixOS package queries are only supported on the async server.\nPlease use the main server (port 43) for NixOS queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::OpenSuse(base_query) => {
                        info!("Processing OpenSUSE package query: {}", base_query);
                        // OpenSUSE queries require async HTTP operations
                        // Return a notice that OpenSUSE queries require async server
                        Ok(format!("OpenSUSE package queries are only supported on the async server.\nPlease use the main server (port 43) for OpenSUSE queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Npm(base_query) => {
                        info!("Processing NPM package query: {}", base_query);
                        // NPM queries require async HTTP operations
                        // Return a notice that NPM queries require async server
                        Ok(format!("NPM package queries are only supported on the async server.\nPlease use the main server (port 43) for NPM queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Pypi(base_query) => {
                        info!("Processing PyPI package query: {}", base_query);
                        // PyPI queries require async HTTP operations
                        // Return a notice that PyPI queries require async server
                        Ok(format!("PyPI package queries are only supported on the async server.\nPlease use the main server (port 43) for PyPI queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::GitHub(base_query) => {
                        info!("Processing GitHub user/repository query: {}", base_query);
                        // GitHub queries require async HTTP operations
                        // Return a notice that GitHub queries require async server
                        Ok(format!("GitHub user/repository queries are only supported on the async server.\nPlease use the main server (port 43) for GitHub queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Wikipedia(base_query) => {
                        info!("Processing Wikipedia article query: {}", base_query);
                        // Wikipedia queries require async HTTP operations
                        // Return a notice that Wikipedia queries require async server
                        Ok(format!("Wikipedia article queries are only supported on the async server.\nPlease use the main server (port 43) for Wikipedia queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Lyric(base_query) => {
                        info!("Processing Luotianyi lyric query: {}", base_query);
                        // Lyric queries require async HTTP operations
                        // Return a notice that lyric queries require async server
                        Ok(format!("Luotianyi lyric queries are only supported on the async server.\nPlease use the main server (port 43) for lyric queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Cargo(base_query) => {
                        info!("Processing Cargo package query: {}", base_query);
                        // Cargo queries require async HTTP operations
                        // Return a notice that Cargo queries require async server
                        Ok(format!("Cargo package queries are only supported on the async server.\nPlease use the main server (port 43) for Cargo queries.\nQuery: {}\n", base_query))
                    }
                    QueryType::Meal => {
                        info!("Processing meal suggestion query");
                        // Meal queries require async HTTP operations
                        // Return a notice that meal queries require async server
                        Ok("Meal suggestion queries (今天吃什么/-MEAL) are only supported on the async server.\nPlease use the main server (port 43) for meal queries.\n".to_string())
                    }
                    QueryType::Help => {
                        info!("Processing HELP query");
                        Ok(crate::services::help::generate_help_response())
                    }
                    QueryType::Unknown(q) => {
                        info!("Unknown query type: {}", q);
                        if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                            info!("Detected DN42 related query ({}), using DN42 query", q);
                            process_dn42_query_managed_blocking(q)
                        } else {
                            let public_result = blocking_query_with_iana_referral(q, timeout);
                            
                            match &public_result {
                                Ok(response) if response.trim().is_empty() 
                                    || response.contains("No entries found") 
                                    || response.contains("Not found") => {
                                    info!("Public query returned no results, trying DN42 for: {}", q);
                                    process_dn42_query_managed_blocking(q)
                                },
                                Err(_) => {
                                    info!("Public query failed, trying DN42 for: {}", q);
                                    process_dn42_query_managed_blocking(q)
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