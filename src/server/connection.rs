use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio::net::TcpStream;
use tracing::{ debug, error, warn };

use crate::services::{
    process_bgptool_query,
    process_email_search,
    process_geo_query,
    process_rir_geo_query,
    process_prefixes_query,
    process_irr_query,
    process_looking_glass_query,
    process_manrs_query,
    process_rpki_query,
    process_dns_query,
    process_traceroute_query,
    process_ssl_query,
    process_crt_query,
    process_minecraft_query,
    process_minecraft_user_query,
    process_steam_query,
    process_steam_search_query,
    process_imdb_query,
    process_imdb_search_query,
    process_acgc_query,
    process_alma_query,
    process_aosc_query,
    process_aur_query,
    process_debian_query,
    process_epel_query,
    process_ubuntu_query,
    process_nixos_query,
    process_opensuse_query,
    process_openwrt_query,
    process_npm_query,
    process_pypi_query,
    process_cargo_query,
    query_modrinth,
    query_curseforge,
    process_github_query,
    process_wikipedia_query,
    process_lyric_query,
    process_desc_query,
    query_random_meal,
    query_random_chinese_meal,
    query_whois,
    query_with_iana_referral,
};
use crate::config::{ SERVER_BANNER, RADB_WHOIS_SERVER, RADB_WHOIS_PORT };
use crate::dn42::process_dn42_query_managed;
use crate::core::{
    analyze_query,
    is_private_ipv4,
    is_private_ipv6,
    QueryType,
    dump_to_file,
    StatsState,
    ColorProtocol,
    Colorizer,
    ColorScheme,
};

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    timeout: Duration,
    dump_traffic: bool,
    dump_dir: &str,
    stats: StatsState,
    enable_color: bool
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
    if let Err(_) = tokio::time::timeout(timeout, read_future).await {
        return Err(anyhow::anyhow!("Request read timeout"));
    }

    // Dump query if requested
    if dump_traffic {
        let timestamp = std::time::SystemTime
            ::now()
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
        debug!("Received WHOIS-COLOR capability probe from {}", addr);
        let capability_response = color_protocol.get_capability_response();

        if let Err(e) = stream.write_all(capability_response.as_bytes()).await {
            error!("Failed to send capability response: {}", e);
        } else {
            debug!("Sent WHOIS-COLOR capability response");
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
        debug!("Received empty query from {}", addr);
        return Ok(());
    }

    debug!("Received query from {}: {} (color: {:?})", addr, query, color_protocol.scheme);

    // Analyze query type
    let query_type = analyze_query(&query);

    // Select appropriate WHOIS server and query
    let result = match &query_type {
        QueryType::Domain(domain) => {
            debug!("Processing domain query: {}", domain);
            if domain.to_lowercase().ends_with(".dn42") {
                debug!("Detected .dn42 domain, using DN42 query");
                process_dn42_query_managed(domain).await
            } else {
                query_with_iana_referral(domain).await
            }
        }
        QueryType::IPv4(ip) => {
            debug!("Processing IPv4 query: {}", ip);
            if is_private_ipv4(*ip) {
                debug!("Detected private IPv4 address, using DN42 query");
                process_dn42_query_managed(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::IPv6(ip) => {
            debug!("Processing IPv6 query: {}", ip);
            if is_private_ipv6(*ip) {
                debug!("Detected private IPv6 address, using DN42 query");
                process_dn42_query_managed(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::ASN(asn) => {
            debug!("Processing ASN query: {}", asn);
            if asn.to_uppercase().starts_with("AS42424") {
                debug!("Detected DN42 ASN, using DN42 query");
                process_dn42_query_managed(asn).await
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
        QueryType::Manrs(base_query) => {
            debug!("Processing MANRS query: {}", base_query);
            process_manrs_query(&format!("{}-MANRS", base_query)).await
        }
        QueryType::Dns(base_query) => {
            debug!("Processing DNS query: {}", base_query);
            process_dns_query(base_query).await
        }
        QueryType::Trace(base_query) => {
            debug!("Processing traceroute query: {}", base_query);
            process_traceroute_query(base_query).await
        }
        QueryType::Ssl(base_query) => {
            debug!("Processing SSL certificate query: {}", base_query);
            process_ssl_query(&format!("{}-SSL", base_query)).await
        }
        QueryType::Crt(base_query) => {
            debug!("Processing Certificate Transparency query: {}", base_query);
            process_crt_query(&format!("{}-CRT", base_query)).await
        }
        QueryType::Minecraft(base_query) => {
            debug!("Processing Minecraft server query: {}", base_query);
            process_minecraft_query(&format!("{}-MC", base_query)).await
        }
        QueryType::MinecraftUser(base_query) => {
            debug!("Processing Minecraft user query: {}", base_query);
            process_minecraft_user_query(&format!("{}-MCU", base_query)).await
        }
        QueryType::Steam(base_query) => {
            debug!("Processing Steam game/user query: {}", base_query);
            process_steam_query(&format!("{}-STEAM", base_query)).await
        }
        QueryType::SteamSearch(base_query) => {
            debug!("Processing Steam game search query: {}", base_query);
            process_steam_search_query(&format!("{}-STEAMSEARCH", base_query)).await
        }
        QueryType::Imdb(base_query) => {
            debug!("Processing IMDb movie/TV show query: {}", base_query);
            process_imdb_query(&format!("{}-IMDB", base_query)).await
        }
        QueryType::ImdbSearch(base_query) => {
            debug!("Processing IMDb search query: {}", base_query);
            process_imdb_search_query(&format!("{}-IMDBSEARCH", base_query)).await
        }
        QueryType::Acgc(base_query) => {
            debug!("Processing ACGC character query: {}", base_query);
            process_acgc_query(&format!("{}-ACGC", base_query)).await
        }
        QueryType::Alma(base_query) => {
            debug!("Processing AlmaLinux package query: {}", base_query);
            process_alma_query(base_query).await
        }
        QueryType::Aosc(base_query) => {
            debug!("Processing AOSC package query: {}", base_query);
            process_aosc_query(base_query).await
        }
        QueryType::Aur(base_query) => {
            debug!("Processing AUR package query: {}", base_query);
            process_aur_query(base_query).await
        }
        QueryType::Debian(base_query) => {
            debug!("Processing Debian package query: {}", base_query);
            process_debian_query(base_query).await
        }
        QueryType::Epel(base_query) => {
            debug!("Processing EPEL package query: {}", base_query);
            process_epel_query(base_query).await
        }
        QueryType::Ubuntu(base_query) => {
            debug!("Processing Ubuntu package query: {}", base_query);
            process_ubuntu_query(base_query).await
        }
        QueryType::NixOs(base_query) => {
            debug!("Processing NixOS package query: {}", base_query);
            process_nixos_query(base_query).await
        }
        QueryType::OpenSuse(base_query) => {
            debug!("Processing OpenSUSE package query: {}", base_query);
            process_opensuse_query(base_query).await
        }
        QueryType::OpenWrt(base_query) => {
            debug!("Processing OpenWrt package query: {}", base_query);
            process_openwrt_query(base_query).await
        }
        QueryType::Npm(base_query) => {
            debug!("Processing NPM package query: {}", base_query);
            process_npm_query(base_query).await
        }
        QueryType::Pypi(base_query) => {
            debug!("Processing PyPI package query: {}", base_query);
            process_pypi_query(base_query).await
        }
        QueryType::Cargo(base_query) => {
            debug!("Processing Cargo (Rust) package query: {}", base_query);
            process_cargo_query(base_query).await
        }
        QueryType::Modrinth(base_query) => {
            debug!("Processing Modrinth mod/resource pack query: {}", base_query);
            query_modrinth(base_query).await
        }
        QueryType::CurseForge(base_query) => {
            debug!("Processing CurseForge mod query: {}", base_query);
            query_curseforge(base_query).await
        }
        QueryType::GitHub(base_query) => {
            debug!("Processing GitHub user/repository query: {}", base_query);
            process_github_query(base_query).await
        }
        QueryType::Wikipedia(base_query) => {
            debug!("Processing Wikipedia article query: {}", base_query);
            process_wikipedia_query(&format!("{}-WIKIPEDIA", base_query)).await
        }
        QueryType::Lyric(base_query) => {
            debug!("Processing Luotianyi lyric query: {}", base_query);
            process_lyric_query(&format!("{}-LYRIC", base_query)).await
        }
        QueryType::Desc(base_query) => {
            debug!("Processing description query: {}", base_query);
            process_desc_query(base_query).await
        }
        QueryType::Meal => {
            debug!("Processing meal suggestion query");
            query_random_meal().await
        }
        QueryType::MealCN => {
            debug!("Processing Chinese meal suggestion query");
            query_random_chinese_meal().await
        }
        QueryType::Help => {
            debug!("Processing HELP query");
            Ok(crate::services::help::generate_help_response())
        }
        QueryType::Unknown(q) => {
            debug!("Unknown query type: {}", q);
            if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                debug!("Detected DN42 related query ({}), using DN42 query", q);
                process_dn42_query_managed(q).await
            } else {
                let public_result = query_with_iana_referral(q).await;

                match &public_result {
                    Ok(response) if
                        response.trim().is_empty() ||
                        response.contains("No entries found") ||
                        response.contains("Not found")
                    => {
                        debug!("Public query returned no results, trying DN42 for: {}", q);
                        process_dn42_query_managed(q).await
                    }
                    Err(_) => {
                        debug!("Public query failed, trying DN42 for: {}", q);
                        process_dn42_query_managed(q).await
                    }
                    _ => public_result,
                }
            }
        }
    };

    // Format the response with proper WHOIS format and optional colorization
    let formatted_response = match result {
        Ok(resp) => {
            let mut formatted = format!("{}\r\n", SERVER_BANNER);
            formatted.push_str("% The objects are in RPSL format\r\n");
            formatted.push_str("% Please report any issues to noc@akae.re\r\n");
            formatted.push_str("\r\n");

            // Apply colorization if requested and supported
            let response_content = if color_protocol.should_colorize() {
                if let Some(scheme) = &color_protocol.scheme {
                    let colorizer = Colorizer::new(scheme.clone());
                    colorizer.colorize_response(&resp, &query_type)
                } else {
                    resp
                }
            } else {
                resp
            };

            // Add the response content (colorized or plain)
            formatted.push_str(&response_content);

            // Ensure response ends with a CRLF
            if !formatted.ends_with("\r\n") {
                formatted.push_str("\r\n");
            }

            formatted
        }
        Err(e) => {
            error!("WHOIS query error for {}: {}", query, e);

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
        let timestamp = std::time::SystemTime
            ::now()
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
            crate::core::record_request(&stats, formatted_response.len()).await;
        }
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

/// Process a WHOIS query and return the response (for use by SSH server and other modules)
#[allow(dead_code)]
pub async fn handle_query(
    query: &str,
    query_type: &QueryType,
    color_scheme: Option<ColorScheme>
) -> Result<String> {
    crate::core::process_query(query, query_type, color_scheme).await
}
