// WHOIS Server - Query Processor
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Core query processing logic shared between different server implementations

use anyhow::Result;
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
    TC_WHOIS_PORT,
    TC_WHOIS_SERVER,
};
use crate::core::{
    ColorScheme,
    Colorizer,
    QueryType,
    apply_response_patches,
    is_private_ipv4,
    is_private_ipv6,
};
use crate::log_debug;
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
    process_icp_query,
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

/// Process a WHOIS query and return the response (for use by SSH server and other modules)
pub async fn process_query(
    query: &str,
    query_type: &QueryType,
    color_scheme: Option<ColorScheme>,
    client_ip: Option<String>
) -> Result<String> {
    log_debug!("Processing query: {} (type: {:?})", query, query_type);

    // Start timing the query
    let start_time = std::time::Instant::now();

    // Process the query based on its type
    let result = match query_type {
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
                process_dn42_query_managed(query).await
            } else {
                query_with_iana_referral(query).await
            }
        }
        QueryType::IPv6(ip) => {
            log_debug!("Processing IPv6 query: {}", ip);
            if is_private_ipv6(*ip) {
                log_debug!("Detected private IPv6 address, using DN42 query");
                process_dn42_query_managed(query).await
            } else {
                query_with_iana_referral(query).await
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
        QueryType::Pixiv(base_query) => {
            log_debug!("Processing Pixiv query: {}", base_query);
            crate::services::pixiv::process_pixiv_query(base_query).await
        }
        QueryType::Icp(base_query) => {
            log_debug!("Processing ICP query: {}", base_query);
            Ok(process_icp_query(base_query).await)
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
        QueryType::Unknown(q) => {
            log_debug!("Unknown query type: {}", q);
            if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                log_debug!("Detected DN42 related query ({}), using DN42 query", q);
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

    // Calculate response time
    let response_time = start_time.elapsed().as_millis() as u64;

    // Send telemetry data if client IP is provided
    if let Some(ip) = client_ip {
        let query_object = query.to_string();
        let query_type_str = crate::core::telemetry::query_type_to_string(query_type);

        let telemetry_data = crate::core::telemetry::TelemetryData::new(
            query_object,
            query_type_str,
            ip,
            response_time
        );

        crate::core::telemetry::send_telemetry(telemetry_data).await;
    }

    // Apply colorization if scheme is provided, then apply patches
    match result {
        Ok(response) => {
            // First apply colorization if requested
            let colored_response = if let Some(scheme) = color_scheme {
                let colorizer = Colorizer::new(scheme);
                colorizer.colorize_response(&response, query_type)
            } else {
                response
            };

            // Then apply response patches
            let patched_response = apply_response_patches(query, colored_response);
            Ok(patched_response)
        }
        Err(e) => Err(e),
    }
}
