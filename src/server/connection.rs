use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio::net::TcpStream;
use crate::config::{
    AFRINIC_WHOIS_PORT,
    AFRINIC_WHOIS_SERVER,
    ALTDB_WHOIS_PORT,
    ALTDB_WHOIS_SERVER,
    APNIC_WHOIS_PORT,
    APNIC_WHOIS_SERVER,
    ARIN_WHOIS_PORT,
    ARIN_WHOIS_SERVER,
    BELL_WHOIS_PORT,
    BELL_WHOIS_SERVER,
    JPIRR_WHOIS_PORT,
    JPIRR_WHOIS_SERVER,
    LACNIC_WHOIS_PORT,
    LACNIC_WHOIS_SERVER,
    LEVEL3_WHOIS_PORT,
    LEVEL3_WHOIS_SERVER,
    NTTCOM_WHOIS_PORT,
    NTTCOM_WHOIS_SERVER,
    RADB_WHOIS_PORT,
    RADB_WHOIS_SERVER,
    RIPE_WHOIS_PORT,
    RIPE_WHOIS_SERVER,
    RIS_WHOIS_PORT,
    RIS_WHOIS_SERVER,
    SERVER_BANNER,
    TC_WHOIS_PORT,
    TC_WHOIS_SERVER,
};
use crate::core::{
    ColorProtocol,
    ColorScheme,
    Colorizer,
    QueryType,
    StatsState,
    analyze_query,
    apply_response_patches,
    dump_to_file,
    is_private_ipv4,
    is_private_ipv6,
};
use crate::{log_debug, log_error, log_warn};
use crate::dn42::process_dn42_query_managed;
use crate::services::{
    handle_ntp_query,
    process_acgc_query,
    process_alma_query,
    process_aosc_query,
    process_aur_query,
    process_bgptool_query,
    process_cargo_query,
    process_cfstatus_query,
    process_crt_query,
    process_debian_query,
    process_desc_query,
    process_dns_query,
    process_email_search,
    process_epel_query,
    process_geo_query,
    process_github_query,
    process_imdb_query,
    process_imdb_search_query,
    process_irr_query,
    process_looking_glass_query,
    process_lyric_query,
    process_manrs_query,
    process_minecraft_query,
    process_minecraft_user_query,
    process_nixos_query,
    process_npm_query,
    process_opensuse_query,
    process_openwrt_query,
    process_peeringdb_query,
    process_pen_query,
    process_prefixes_query,
    process_pypi_query,
    process_rdap_query,
    process_rir_geo_query,
    process_rpki_query,
    process_ssl_query,
    process_steam_query,
    process_steam_search_query,
    process_traceroute_query,
    process_ubuntu_query,
    process_wikipedia_query,
    query_curseforge,
    query_modrinth,
    query_random_chinese_meal,
    query_random_meal,
    query_whois,
    query_with_iana_referral,
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
        log_warn!("Failed to set TCP_NODELAY: {}", e);
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

    log_debug!("Received query from {}: {} (color: {:?})", addr, query, color_protocol.scheme);

    // Start timing the query
    let start_time = std::time::Instant::now();

    // Analyze query type
    let query_type = analyze_query(&query);

    // Select appropriate WHOIS server and query
    let result = match &query_type {
        QueryType::Domain(domain) => {
            log_debug!("Processing domain query: {}", domain);
            if domain.to_lowercase().ends_with(".dn42") {
                log_debug!("Detected .dn42 domain, using DN42 query");
                process_dn42_query_managed(domain).await
            } else {
                query_with_iana_referral(domain).await
            }
        }
        QueryType::IPv4(ip) => {
            log_debug!("Processing IPv4 query: {}", ip);
            if is_private_ipv4(*ip) {
                log_debug!("Detected private IPv4 address, using DN42 query");
                process_dn42_query_managed(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::IPv6(ip) => {
            log_debug!("Processing IPv6 query: {}", ip);
            if is_private_ipv6(*ip) {
                log_debug!("Detected private IPv6 address, using DN42 query");
                process_dn42_query_managed(&query).await
            } else {
                query_with_iana_referral(&query).await
            }
        }
        QueryType::ASN(asn) => {
            log_debug!("Processing ASN query: {}", asn);
            if asn.to_uppercase().starts_with("AS42424") {
                log_debug!("Detected DN42 ASN, using DN42 query");
                process_dn42_query_managed(asn).await
            } else {
                query_with_iana_referral(asn).await
            }
        }
        QueryType::EmailSearch(base_query) => {
            log_debug!("Processing email search query: {}", base_query);
            process_email_search(base_query).await
        }
        QueryType::BGPTool(base_query) => {
            log_debug!("Processing BGP Tools query: {}", base_query);
            process_bgptool_query(base_query).await
        }
        QueryType::Geo(resource) => {
            log_debug!("Processing geo location query: {}", resource);
            process_geo_query(resource).await
        }
        QueryType::RirGeo(resource) => {
            log_debug!("Processing RIR geo location query: {}", resource);
            process_rir_geo_query(resource).await
        }
        QueryType::Prefixes(asn) => {
            log_debug!("Processing ASN prefixes query: {}", asn);
            process_prefixes_query(asn).await
        }
        QueryType::Radb(resource) => {
            log_debug!("Processing RADB query: {}", resource);
            query_whois(resource, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await
        }
        QueryType::Altdb(resource) => {
            log_debug!("Processing ALTDB query: {}", resource);
            query_whois(resource, ALTDB_WHOIS_SERVER, ALTDB_WHOIS_PORT).await
        }
        QueryType::Afrinic(resource) => {
            log_debug!("Processing AFRINIC query: {}", resource);
            query_whois(resource, AFRINIC_WHOIS_SERVER, AFRINIC_WHOIS_PORT).await
        }
        QueryType::Apnic(resource) => {
            log_debug!("Processing APNIC query: {}", resource);
            query_whois(resource, APNIC_WHOIS_SERVER, APNIC_WHOIS_PORT).await
        }
        QueryType::ArinIrr(resource) => {
            log_debug!("Processing ARIN IRR query: {}", resource);
            query_whois(resource, ARIN_WHOIS_SERVER, ARIN_WHOIS_PORT).await
        }
        QueryType::Bell(resource) => {
            log_debug!("Processing BELL query: {}", resource);
            query_whois(resource, BELL_WHOIS_SERVER, BELL_WHOIS_PORT).await
        }
        QueryType::Jpirr(resource) => {
            log_debug!("Processing JPIRR query: {}", resource);
            query_whois(resource, JPIRR_WHOIS_SERVER, JPIRR_WHOIS_PORT).await
        }
        QueryType::Lacnic(resource) => {
            log_debug!("Processing LACNIC query: {}", resource);
            query_whois(resource, LACNIC_WHOIS_SERVER, LACNIC_WHOIS_PORT).await
        }
        QueryType::Level3(resource) => {
            log_debug!("Processing LEVEL3 query: {}", resource);
            query_whois(resource, LEVEL3_WHOIS_SERVER, LEVEL3_WHOIS_PORT).await
        }
        QueryType::Nttcom(resource) => {
            log_debug!("Processing NTTCOM query: {}", resource);
            query_whois(resource, NTTCOM_WHOIS_SERVER, NTTCOM_WHOIS_PORT).await
        }
        QueryType::RipeIrr(resource) => {
            log_debug!("Processing RIPE IRR query: {}", resource);
            query_whois(resource, RIPE_WHOIS_SERVER, RIPE_WHOIS_PORT).await
        }
        QueryType::Ris(resource) => {
            log_debug!("Processing RIS query: {}", resource);
            query_whois(resource, RIS_WHOIS_SERVER, RIS_WHOIS_PORT).await
        }
        QueryType::Tc(resource) => {
            log_debug!("Processing TC query: {}", resource);
            query_whois(resource, TC_WHOIS_SERVER, TC_WHOIS_PORT).await
        }
        QueryType::Irr(resource) => {
            log_debug!("Processing IRR Explorer query: {}", resource);
            process_irr_query(resource).await
        }
        QueryType::LookingGlass(resource) => {
            log_debug!("Processing Looking Glass query: {}", resource);
            process_looking_glass_query(resource).await
        }
        QueryType::Rpki(prefix, asn) => {
            log_debug!("Processing RPKI query: prefix={}, asn={}", prefix, asn);
            process_rpki_query(prefix, asn).await
        }
        QueryType::Manrs(base_query) => {
            log_debug!("Processing MANRS query: {}", base_query);
            process_manrs_query(&format!("{}-MANRS", base_query)).await
        }
        QueryType::Dns(base_query) => {
            log_debug!("Processing DNS query: {}", base_query);
            process_dns_query(base_query).await
        }
        QueryType::Ntp(base_query) => {
            log_debug!("Processing NTP query: {}", base_query);
            handle_ntp_query(base_query).await
        }
        QueryType::Trace(base_query) => {
            log_debug!("Processing traceroute query: {}", base_query);
            process_traceroute_query(base_query).await
        }
        QueryType::Ssl(base_query) => {
            log_debug!("Processing SSL certificate query: {}", base_query);
            process_ssl_query(&format!("{}-SSL", base_query)).await
        }
        QueryType::Crt(base_query) => {
            log_debug!("Processing Certificate Transparency query: {}", base_query);
            process_crt_query(&format!("{}-CRT", base_query)).await
        }
        QueryType::CfStatus(base_query) => {
            log_debug!("Processing Cloudflare Status query: {}", base_query);
            process_cfstatus_query(&format!("{}-CFSTATUS", base_query)).await
        }
        QueryType::Minecraft(base_query) => {
            log_debug!("Processing Minecraft server query: {}", base_query);
            process_minecraft_query(&format!("{}-MC", base_query)).await
        }
        QueryType::MinecraftUser(base_query) => {
            log_debug!("Processing Minecraft user query: {}", base_query);
            process_minecraft_user_query(&format!("{}-MCU", base_query)).await
        }
        QueryType::Steam(base_query) => {
            log_debug!("Processing Steam game/user query: {}", base_query);
            process_steam_query(&format!("{}-STEAM", base_query)).await
        }
        QueryType::SteamSearch(base_query) => {
            log_debug!("Processing Steam game search query: {}", base_query);
            process_steam_search_query(&format!("{}-STEAMSEARCH", base_query)).await
        }
        QueryType::Imdb(base_query) => {
            log_debug!("Processing IMDb movie/TV show query: {}", base_query);
            process_imdb_query(&format!("{}-IMDB", base_query)).await
        }
        QueryType::ImdbSearch(base_query) => {
            log_debug!("Processing IMDb search query: {}", base_query);
            process_imdb_search_query(&format!("{}-IMDBSEARCH", base_query)).await
        }
        QueryType::Acgc(base_query) => {
            log_debug!("Processing ACGC character query: {}", base_query);
            process_acgc_query(&format!("{}-ACGC", base_query)).await
        }
        QueryType::Alma(base_query) => {
            log_debug!("Processing AlmaLinux package query: {}", base_query);
            process_alma_query(base_query).await
        }
        QueryType::Aosc(base_query) => {
            log_debug!("Processing AOSC package query: {}", base_query);
            process_aosc_query(base_query).await
        }
        QueryType::Aur(base_query) => {
            log_debug!("Processing AUR package query: {}", base_query);
            process_aur_query(base_query).await
        }
        QueryType::Debian(base_query) => {
            log_debug!("Processing Debian package query: {}", base_query);
            process_debian_query(base_query).await
        }
        QueryType::Epel(base_query) => {
            log_debug!("Processing EPEL package query: {}", base_query);
            process_epel_query(base_query).await
        }
        QueryType::Ubuntu(base_query) => {
            log_debug!("Processing Ubuntu package query: {}", base_query);
            process_ubuntu_query(base_query).await
        }
        QueryType::NixOs(base_query) => {
            log_debug!("Processing NixOS package query: {}", base_query);
            process_nixos_query(base_query).await
        }
        QueryType::OpenSuse(base_query) => {
            log_debug!("Processing OpenSUSE package query: {}", base_query);
            process_opensuse_query(base_query).await
        }
        QueryType::OpenWrt(base_query) => {
            log_debug!("Processing OpenWrt package query: {}", base_query);
            process_openwrt_query(base_query).await
        }
        QueryType::Npm(base_query) => {
            log_debug!("Processing NPM package query: {}", base_query);
            process_npm_query(base_query).await
        }
        QueryType::Pypi(base_query) => {
            log_debug!("Processing PyPI package query: {}", base_query);
            process_pypi_query(base_query).await
        }
        QueryType::Cargo(base_query) => {
            log_debug!("Processing Cargo (Rust) package query: {}", base_query);
            process_cargo_query(base_query).await
        }
        QueryType::Modrinth(base_query) => {
            log_debug!("Processing Modrinth mod/resource pack query: {}", base_query);
            query_modrinth(base_query).await
        }
        QueryType::CurseForge(base_query) => {
            log_debug!("Processing CurseForge mod query: {}", base_query);
            query_curseforge(base_query).await
        }
        QueryType::GitHub(base_query) => {
            log_debug!("Processing GitHub user/repository query: {}", base_query);
            process_github_query(base_query).await
        }
        QueryType::Wikipedia(base_query) => {
            log_debug!("Processing Wikipedia article query: {}", base_query);
            process_wikipedia_query(&format!("{}-WIKIPEDIA", base_query)).await
        }
        QueryType::Lyric(base_query) => {
            log_debug!("Processing Luotianyi lyric query: {}", base_query);
            process_lyric_query(&format!("{}-LYRIC", base_query)).await
        }
        QueryType::Desc(base_query) => {
            log_debug!("Processing description query: {}", base_query);
            process_desc_query(base_query).await
        }
        QueryType::PeeringDB(base_query) => {
            log_debug!("Processing PeeringDB query: {}", base_query);
            process_peeringdb_query(base_query).await
        }
        QueryType::Pen(base_query) => {
            log_debug!("Processing IANA Private Enterprise Numbers query: {}", base_query);
            process_pen_query(base_query).await
        }
        QueryType::Rdap(base_query) => {
            log_debug!("Processing RDAP query: {}", base_query);
            process_rdap_query(base_query).await
        }
        QueryType::Meal => {
            log_debug!("Processing meal suggestion query");
            query_random_meal().await
        }
        QueryType::MealCN => {
            log_debug!("Processing Chinese meal suggestion query");
            query_random_chinese_meal().await
        }
        QueryType::Help => {
            log_debug!("Processing HELP query");
            Ok(crate::services::help::generate_help_response())
        }
        QueryType::UpdatePatch => {
            log_debug!("Processing UPDATE-PATCH query");
            use crate::core::patch::process_update_patch_query;
            match process_update_patch_query().await {
                Ok(output) => Ok(output),
                Err(e) => Ok(format!("% Error: {}\n", e)),
            }
        }
        QueryType::Pixiv(base_query) => {
            log_debug!("Processing Pixiv query: {}", base_query);
            crate::services::pixiv::process_pixiv_query(base_query).await
        }
        QueryType::Icp(base_query) => {
            log_debug!("Processing ICP query: {}", base_query);
            Ok(crate::services::process_icp_query(base_query).await)
        }
        QueryType::Plugin(_, _) => {
            // Plugins should be handled by process_query, not here
            // This is a fallback path
            log_debug!("Plugin query routed to connection handler, using standard query processor");
            crate::core::query_processor::process_query(&query, &query_type, None, None).await
        }
        QueryType::Unknown(q) => {
            log_debug!("Unknown query type: {}", q);
            let q_upper = q.to_uppercase();
            if
                q_upper.ends_with("-DN42") ||
                q_upper.ends_with("-MNT") ||
                q_upper.ends_with("-NEONETWORK") ||
                q_upper.ends_with("-CRXN")
            {
                log_debug!("Detected DN42/NeoNetwork/CRXN related query ({}), using DN42 database", q);
                process_dn42_query_managed(q).await
            } else {
                let public_result = query_with_iana_referral(q).await;

                match &public_result {
                    Ok(response) if
                        response.trim().is_empty() ||
                        response.contains("No entries found") ||
                        response.contains("Not found")
                    => {
                        log_debug!("Public query returned no results, trying DN42 for: {}", q);
                        process_dn42_query_managed(q).await
                    }
                    Err(_) => {
                        log_debug!("Public query failed, trying DN42 for: {}", q);
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

            // Apply response patches (after colorization)
            let patched_content = apply_response_patches(&query, response_content);

            // Add the response content (colorized and patched)
            formatted.push_str(&patched_content);

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
        let timestamp = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        dump_to_file(&format!("{}/response_{}.txt", dump_dir, timestamp), &formatted_response);
    }

    // Log the response size (helpful for debugging)
    log_debug!("Sending response ({} bytes) for query: {}", formatted_response.len(), query);

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

            // Send telemetry data
            let response_time = start_time.elapsed().as_millis() as u64;
            let client_ip = addr.ip().to_string();
            let query_type_str = crate::core::telemetry::query_type_to_string(&query_type);

            let telemetry_data = crate::core::telemetry::TelemetryData::new(
                query.clone(),
                query_type_str,
                client_ip,
                response_time
            );

            crate::core::telemetry::send_telemetry(telemetry_data).await;
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
    client_ip: Option<String>
) -> Result<String> {
    crate::core::process_query(query, query_type, color_scheme, client_ip).await
}
