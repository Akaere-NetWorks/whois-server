// WHOIS Server - Query Processor
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Core query processing logic shared between different server implementations

use crate::core::{ColorScheme, Colorizer, QueryType, apply_response_patches};
#[cfg(feature = "dn42")]
use crate::core::{is_private_ipv4, is_private_ipv6};
use crate::log_debug;
use anyhow::Result;
use rdap::QueryType as RdapQueryType;

#[cfg(feature = "dn42")]
use crate::dn42::process_dn42_query_managed;

#[cfg(feature = "bgp-irr")]
use crate::config::{
    AFRINIC_WHOIS_PORT, AFRINIC_WHOIS_SERVER, ALTDB_WHOIS_PORT, ALTDB_WHOIS_SERVER,
    APNIC_WHOIS_PORT, APNIC_WHOIS_SERVER, ARIN_WHOIS_PORT, ARIN_WHOIS_SERVER, BELL_WHOIS_PORT,
    BELL_WHOIS_SERVER, JPIRR_WHOIS_PORT, JPIRR_WHOIS_SERVER, LACNIC_WHOIS_PORT,
    LACNIC_WHOIS_SERVER, LEVEL3_WHOIS_PORT, LEVEL3_WHOIS_SERVER, NTTCOM_WHOIS_PORT,
    NTTCOM_WHOIS_SERVER, RADB_WHOIS_PORT, RADB_WHOIS_SERVER, RIS_WHOIS_PORT, RIS_WHOIS_SERVER,
    TC_WHOIS_PORT, TC_WHOIS_SERVER,
};

// Core service imports (always compiled)
use crate::services::{
    process_desc_query, query_with_iana_referral, rdap::query_rdap_with_whois_fallback,
};
#[cfg(feature = "bgp-irr")]
use crate::services::query_whois;

#[cfg(feature = "bgp")]
use crate::services::{
    process_bgptool_query, process_irr_query, process_looking_glass_query, process_rpki_query,
};

#[cfg(feature = "bgp-irr")]
use crate::services::query_ripe_whois;

#[cfg(feature = "bgp-manrs")]
use crate::services::process_manrs_query;

#[cfg(feature = "peeringdb")]
use crate::services::process_peeringdb_query;

#[cfg(feature = "geo")]
use crate::services::{process_geo_query, process_rir_geo_query};

#[cfg(feature = "bgp-prefixes")]
use crate::services::process_prefixes_query;

#[cfg(feature = "dns")]
use crate::services::process_dns_query;

#[cfg(feature = "ntp")]
use crate::services::handle_ntp_query;

#[cfg(feature = "ping")]
use crate::services::process_ping_query;

#[cfg(feature = "traceroute")]
use crate::services::process_traceroute_query;

#[cfg(feature = "ssl")]
use crate::services::process_ssl_query;

#[cfg(feature = "crt")]
use crate::services::process_crt_query;

#[cfg(feature = "cfstatus")]
use crate::services::process_cfstatus_query;

#[cfg(feature = "email")]
use crate::services::process_email_search;

#[cfg(feature = "steam")]
use crate::services::{process_steam_query, process_steam_search_query};

#[cfg(feature = "imdb")]
use crate::services::{process_imdb_query, process_imdb_search_query};

#[cfg(feature = "minecraft")]
use crate::services::{process_minecraft_query, process_minecraft_user_query};

#[cfg(feature = "acgc")]
use crate::services::process_acgc_query;

#[cfg(feature = "lyric")]
use crate::services::process_lyric_query;

#[cfg(feature = "wikipedia")]
use crate::services::process_wikipedia_query;

#[cfg(feature = "github")]
use crate::services::process_github_query;

#[cfg(feature = "icp")]
use crate::services::process_icp_query;

#[cfg(feature = "pen")]
use crate::services::process_pen_query;

#[cfg(feature = "rdap-explicit")]
use crate::services::process_rdap_query;

#[cfg(feature = "pixiv")]
use crate::services::pixiv::process_pixiv_query;

#[cfg(feature = "meal")]
use crate::services::{query_random_meal, query_random_chinese_meal};

#[cfg(feature = "pkg-alma")]
use crate::services::process_alma_query;
#[cfg(feature = "pkg-aosc")]
use crate::services::process_aosc_query;
#[cfg(feature = "pkg-aur")]
use crate::services::process_aur_query;
#[cfg(feature = "pkg-debian")]
use crate::services::process_debian_query;
#[cfg(feature = "pkg-epel")]
use crate::services::process_epel_query;
#[cfg(feature = "pkg-ubuntu")]
use crate::services::process_ubuntu_query;
#[cfg(feature = "pkg-nixos")]
use crate::services::process_nixos_query;
#[cfg(feature = "pkg-opensuse")]
use crate::services::process_opensuse_query;
#[cfg(feature = "pkg-openwrt")]
use crate::services::process_openwrt_query;
#[cfg(feature = "pkg-npm")]
use crate::services::process_npm_query;
#[cfg(feature = "pkg-pypi")]
use crate::services::process_pypi_query;
#[cfg(feature = "pkg-cargo")]
use crate::services::process_cargo_query;
#[cfg(feature = "pkg-modrinth")]
use crate::services::query_modrinth;
#[cfg(feature = "pkg-curseforge")]
use crate::services::query_curseforge;

/// Process a WHOIS query and return the response (for use by SSH server and other modules)
pub async fn process_query(
    query: &str,
    query_type: &QueryType,
    color_scheme: Option<ColorScheme>,
    client_ip: Option<String>,
) -> Result<String> {
    log_debug!("Processing query: {} (type: {:?})", query, query_type);

    // Start timing the query
    let start_time = std::time::Instant::now();

    // Process the query based on its type
    let result = match query_type {
        QueryType::Domain(domain) => {
            log_debug!("Processing domain query: {}", domain);
            #[cfg(feature = "dn42")]
            if domain.to_lowercase().ends_with(".dn42") {
                log_debug!("Detected .dn42 domain, using DN42 query");
                return process_dn42_query_managed(domain).await;
            }
            let domain_clone = domain.clone();
            query_rdap_with_whois_fallback(
                domain,
                RdapQueryType::Domain,
                query_with_iana_referral(&domain_clone),
            )
            .await
        }
        QueryType::IPv4(ip) => {
            log_debug!("Processing IPv4 query: {}", ip);
            #[cfg(feature = "dn42")]
            if is_private_ipv4(*ip) {
                log_debug!("Detected private IPv4 address, using DN42 query");
                return process_dn42_query_managed(query).await;
            }
            let query_str = query.to_string();
            query_rdap_with_whois_fallback(
                query,
                RdapQueryType::Ip,
                query_with_iana_referral(&query_str),
            )
            .await
        }
        QueryType::IPv6(ip) => {
            log_debug!("Processing IPv6 query: {}", ip);
            #[cfg(feature = "dn42")]
            if is_private_ipv6(*ip) {
                log_debug!("Detected private IPv6 address, using DN42 query");
                return process_dn42_query_managed(query).await;
            }
            let query_str = query.to_string();
            query_rdap_with_whois_fallback(
                query,
                RdapQueryType::Ip,
                query_with_iana_referral(&query_str),
            )
            .await
        }
        QueryType::ASN(asn) => {
            log_debug!("Processing ASN query: {}", asn);
            #[cfg(feature = "dn42")]
            if asn.to_uppercase().starts_with("AS42424") {
                log_debug!("Detected DN42 ASN, using DN42 query");
                return process_dn42_query_managed(asn).await;
            }
            let asn_clone = asn.clone();
            query_rdap_with_whois_fallback(
                asn,
                RdapQueryType::Autnum,
                query_with_iana_referral(&asn_clone),
            )
            .await
        }
        QueryType::BareASN(num) => {
            let asn_query = format!("AS{}", num);
            log_debug!("Processing bare ASN query: {} -> {}", num, asn_query);
            #[cfg(feature = "dn42")]
            {
                match process_dn42_query_managed(num).await {
                    Ok(response)
                        if !response.contains("404 Not Found") && !response.trim().is_empty() =>
                    {
                        log_debug!("Found bare ASN in DN42: {}", num);
                        return Ok(response);
                    }
                    _ => {
                        log_debug!(
                            "Bare ASN not found in DN42, falling back to RDAP/WHOIS: {}",
                            asn_query
                        );
                    }
                }
            }
            let asn_clone = asn_query.clone();
            query_rdap_with_whois_fallback(
                &asn_query,
                RdapQueryType::Autnum,
                query_with_iana_referral(&asn_clone),
            )
            .await
        }

        #[cfg(feature = "email")]
        QueryType::EmailSearch(base_query) => {
            log_debug!("Processing email search query: {}", base_query);
            process_email_search(base_query).await
        }

        #[cfg(feature = "bgp")]
        QueryType::BGPTool(base_query) => {
            log_debug!("Processing BGP Tools query: {}", base_query);
            process_bgptool_query(base_query).await
        }

        #[cfg(feature = "geo")]
        QueryType::Geo(resource) => {
            log_debug!("Processing geo location query: {}", resource);
            process_geo_query(resource).await
        }

        #[cfg(feature = "geo")]
        QueryType::RirGeo(resource) => {
            log_debug!("Processing RIR geo location query: {}", resource);
            process_rir_geo_query(resource).await
        }

        #[cfg(feature = "bgp-prefixes")]
        QueryType::Prefixes(asn) => {
            log_debug!("Processing ASN prefixes query: {}", asn);
            process_prefixes_query(asn).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Radb(resource) => {
            log_debug!("Processing RADB query: {}", resource);
            query_whois(resource, RADB_WHOIS_SERVER, RADB_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Altdb(resource) => {
            log_debug!("Processing ALTDB query: {}", resource);
            query_whois(resource, ALTDB_WHOIS_SERVER, ALTDB_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Afrinic(resource) => {
            log_debug!("Processing AFRINIC query: {}", resource);
            query_whois(resource, AFRINIC_WHOIS_SERVER, AFRINIC_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Apnic(resource) => {
            log_debug!("Processing APNIC query: {}", resource);
            query_whois(resource, APNIC_WHOIS_SERVER, APNIC_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::ArinIrr(resource) => {
            log_debug!("Processing ARIN IRR query: {}", resource);
            query_whois(resource, ARIN_WHOIS_SERVER, ARIN_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Bell(resource) => {
            log_debug!("Processing BELL query: {}", resource);
            query_whois(resource, BELL_WHOIS_SERVER, BELL_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Jpirr(resource) => {
            log_debug!("Processing JPIRR query: {}", resource);
            query_whois(resource, JPIRR_WHOIS_SERVER, JPIRR_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Lacnic(resource) => {
            log_debug!("Processing LACNIC query: {}", resource);
            query_whois(resource, LACNIC_WHOIS_SERVER, LACNIC_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Level3(resource) => {
            log_debug!("Processing LEVEL3 query: {}", resource);
            query_whois(resource, LEVEL3_WHOIS_SERVER, LEVEL3_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Nttcom(resource) => {
            log_debug!("Processing NTTCOM query: {}", resource);
            query_whois(resource, NTTCOM_WHOIS_SERVER, NTTCOM_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::RipeIrr(resource) => {
            log_debug!("Processing RIPE IRR query: {}", resource);
            query_ripe_whois(resource).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Ris(resource) => {
            log_debug!("Processing RIS query: {}", resource);
            query_whois(resource, RIS_WHOIS_SERVER, RIS_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp-irr")]
        QueryType::Tc(resource) => {
            log_debug!("Processing TC query: {}", resource);
            query_whois(resource, TC_WHOIS_SERVER, TC_WHOIS_PORT).await
        }

        #[cfg(feature = "bgp")]
        QueryType::Irr(resource) => {
            log_debug!("Processing IRR Explorer query: {}", resource);
            process_irr_query(resource).await
        }

        #[cfg(feature = "bgp")]
        QueryType::LookingGlass(resource) => {
            log_debug!("Processing Looking Glass query: {}", resource);
            process_looking_glass_query(resource).await
        }

        #[cfg(feature = "bgp")]
        QueryType::Rpki(prefix, asn) => {
            log_debug!("Processing RPKI query: prefix={}, asn={}", prefix, asn);
            process_rpki_query(prefix, asn).await
        }

        #[cfg(feature = "bgp-manrs")]
        QueryType::Manrs(base_query) => {
            log_debug!("Processing MANRS query: {}", base_query);
            process_manrs_query(&format!("{}-MANRS", base_query)).await
        }

        #[cfg(feature = "dns")]
        QueryType::Dns(base_query) => {
            log_debug!("Processing DNS query: {}", base_query);
            process_dns_query(base_query).await
        }

        #[cfg(feature = "ntp")]
        QueryType::Ntp(base_query) => {
            log_debug!("Processing NTP query: {}", base_query);
            handle_ntp_query(base_query).await
        }

        #[cfg(feature = "ping")]
        QueryType::Ping(base_query) => {
            log_debug!("Processing ping query: {}", base_query);
            process_ping_query(base_query).await
        }

        #[cfg(feature = "traceroute")]
        QueryType::Trace(base_query) => {
            log_debug!("Processing traceroute query: {}", base_query);
            process_traceroute_query(base_query).await
        }

        #[cfg(feature = "ssl")]
        QueryType::Ssl(base_query) => {
            log_debug!("Processing SSL certificate query: {}", base_query);
            process_ssl_query(&format!("{}-SSL", base_query)).await
        }

        #[cfg(feature = "crt")]
        QueryType::Crt(base_query) => {
            log_debug!("Processing Certificate Transparency query: {}", base_query);
            process_crt_query(&format!("{}-CRT", base_query)).await
        }

        #[cfg(feature = "cfstatus")]
        QueryType::CfStatus(base_query) => {
            log_debug!("Processing Cloudflare Status query: {}", base_query);
            process_cfstatus_query(&format!("{}-CFSTATUS", base_query)).await
        }

        #[cfg(feature = "minecraft")]
        QueryType::Minecraft(base_query) => {
            log_debug!("Processing Minecraft server query: {}", base_query);
            process_minecraft_query(&format!("{}-MC", base_query)).await
        }

        #[cfg(feature = "minecraft")]
        QueryType::MinecraftUser(base_query) => {
            log_debug!("Processing Minecraft user query: {}", base_query);
            process_minecraft_user_query(&format!("{}-MCU", base_query)).await
        }

        #[cfg(feature = "steam")]
        QueryType::Steam(base_query) => {
            log_debug!("Processing Steam game/user query: {}", base_query);
            process_steam_query(&format!("{}-STEAM", base_query)).await
        }

        #[cfg(feature = "steam")]
        QueryType::SteamSearch(base_query) => {
            log_debug!("Processing Steam game search query: {}", base_query);
            process_steam_search_query(&format!("{}-STEAMSEARCH", base_query)).await
        }

        #[cfg(feature = "imdb")]
        QueryType::Imdb(base_query) => {
            log_debug!("Processing IMDb movie/TV show query: {}", base_query);
            process_imdb_query(&format!("{}-IMDB", base_query)).await
        }

        #[cfg(feature = "imdb")]
        QueryType::ImdbSearch(base_query) => {
            log_debug!("Processing IMDb search query: {}", base_query);
            process_imdb_search_query(&format!("{}-IMDBSEARCH", base_query)).await
        }

        #[cfg(feature = "acgc")]
        QueryType::Acgc(base_query) => {
            log_debug!("Processing ACGC character query: {}", base_query);
            process_acgc_query(&format!("{}-ACGC", base_query)).await
        }

        #[cfg(feature = "pkg-alma")]
        QueryType::Alma(base_query) => {
            log_debug!("Processing AlmaLinux package query: {}", base_query);
            process_alma_query(base_query).await
        }

        #[cfg(feature = "pkg-aosc")]
        QueryType::Aosc(base_query) => {
            log_debug!("Processing AOSC package query: {}", base_query);
            process_aosc_query(base_query).await
        }

        #[cfg(feature = "pkg-aur")]
        QueryType::Aur(base_query) => {
            log_debug!("Processing AUR package query: {}", base_query);
            process_aur_query(base_query).await
        }

        #[cfg(feature = "pkg-debian")]
        QueryType::Debian(base_query) => {
            log_debug!("Processing Debian package query: {}", base_query);
            process_debian_query(base_query).await
        }

        #[cfg(feature = "pkg-epel")]
        QueryType::Epel(base_query) => {
            log_debug!("Processing EPEL package query: {}", base_query);
            process_epel_query(base_query).await
        }

        #[cfg(feature = "pkg-ubuntu")]
        QueryType::Ubuntu(base_query) => {
            log_debug!("Processing Ubuntu package query: {}", base_query);
            process_ubuntu_query(base_query).await
        }

        #[cfg(feature = "pkg-nixos")]
        QueryType::NixOs(base_query) => {
            log_debug!("Processing NixOS package query: {}", base_query);
            process_nixos_query(base_query).await
        }

        #[cfg(feature = "pkg-opensuse")]
        QueryType::OpenSuse(base_query) => {
            log_debug!("Processing OpenSUSE package query: {}", base_query);
            process_opensuse_query(base_query).await
        }

        #[cfg(feature = "pkg-openwrt")]
        QueryType::OpenWrt(base_query) => {
            log_debug!("Processing OpenWrt package query: {}", base_query);
            process_openwrt_query(base_query).await
        }

        #[cfg(feature = "pkg-npm")]
        QueryType::Npm(base_query) => {
            log_debug!("Processing NPM package query: {}", base_query);
            process_npm_query(base_query).await
        }

        #[cfg(feature = "pkg-pypi")]
        QueryType::Pypi(base_query) => {
            log_debug!("Processing PyPI package query: {}", base_query);
            process_pypi_query(base_query).await
        }

        #[cfg(feature = "pkg-cargo")]
        QueryType::Cargo(base_query) => {
            log_debug!("Processing Cargo (Rust) package query: {}", base_query);
            process_cargo_query(base_query).await
        }

        #[cfg(feature = "pkg-modrinth")]
        QueryType::Modrinth(base_query) => {
            log_debug!(
                "Processing Modrinth mod/resource pack query: {}",
                base_query
            );
            query_modrinth(base_query).await
        }

        #[cfg(feature = "pkg-curseforge")]
        QueryType::CurseForge(base_query) => {
            log_debug!("Processing CurseForge mod query: {}", base_query);
            query_curseforge(base_query).await
        }

        #[cfg(feature = "github")]
        QueryType::GitHub(base_query) => {
            log_debug!("Processing GitHub user/repository query: {}", base_query);
            process_github_query(base_query).await
        }

        #[cfg(feature = "wikipedia")]
        QueryType::Wikipedia(base_query) => {
            log_debug!("Processing Wikipedia article query: {}", base_query);
            process_wikipedia_query(&format!("{}-WIKIPEDIA", base_query)).await
        }

        #[cfg(feature = "lyric")]
        QueryType::Lyric(base_query) => {
            log_debug!("Processing Luotianyi lyric query: {}", base_query);
            process_lyric_query(&format!("{}-LYRIC", base_query)).await
        }

        QueryType::Desc(base_query) => {
            log_debug!("Processing description query: {}", base_query);
            process_desc_query(base_query).await
        }

        #[cfg(feature = "peeringdb")]
        QueryType::PeeringDB(base_query) => {
            log_debug!("Processing PeeringDB query: {}", base_query);
            process_peeringdb_query(base_query).await
        }

        #[cfg(feature = "pen")]
        QueryType::Pen(base_query) => {
            log_debug!(
                "Processing IANA Private Enterprise Numbers query: {}",
                base_query
            );
            process_pen_query(base_query).await
        }

        QueryType::Rdap(base_query) => {
            log_debug!("Processing RDAP query: {}", base_query);
            #[cfg(feature = "rdap-explicit")]
            {
                process_rdap_query(base_query).await
            }
            #[cfg(not(feature = "rdap-explicit"))]
            {
                use rdap::QueryType as RdapQueryType;
                let q_clone = base_query.clone();
                crate::services::rdap::query_rdap_with_whois_fallback(
                    base_query,
                    RdapQueryType::Domain,
                    crate::services::query_with_iana_referral(&q_clone),
                )
                .await
            }
        }

        #[cfg(feature = "pixiv")]
        QueryType::Pixiv(base_query) => {
            log_debug!("Processing Pixiv query: {}", base_query);
            process_pixiv_query(base_query).await
        }

        #[cfg(feature = "icp")]
        QueryType::Icp(base_query) => {
            log_debug!("Processing ICP query: {}", base_query);
            Ok(process_icp_query(base_query).await)
        }

        #[cfg(feature = "meal")]
        QueryType::Meal => {
            log_debug!("Processing meal suggestion query");
            query_random_meal().await
        }

        #[cfg(feature = "meal")]
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

        #[cfg(feature = "plugins")]
        QueryType::Plugin(suffix, base_query) => {
            log_debug!(
                "Processing plugin query: suffix={}, query={}",
                suffix,
                base_query
            );
            process_plugin_query(suffix, base_query, client_ip.clone()).await
        }

        QueryType::Unknown(q) => {
            log_debug!("Unknown query type: {}", q);
            #[cfg(feature = "dn42")]
            if q.to_uppercase().ends_with("-DN42") || q.to_uppercase().ends_with("-MNT") {
                log_debug!("Detected DN42 related query ({}), using DN42 query", q);
                return process_dn42_query_managed(q).await;
            }
            // RDAP-first with WHOIS fallback, then DN42 if still not found
            let rdap_whois_result = match crate::services::rdap::detect_rdap_query_type(q) {
                Some(rdap_qt) => {
                    let q_clone = q.to_string();
                    query_rdap_with_whois_fallback(
                        q,
                        rdap_qt,
                        query_with_iana_referral(&q_clone),
                    )
                    .await
                }
                None => query_with_iana_referral(q).await,
            };

            #[cfg(feature = "dn42")]
            match &rdap_whois_result {
                Ok(response)
                    if response.trim().is_empty()
                        || response.contains("No entries found")
                        || response.contains("Not found") =>
                {
                    log_debug!("Public query returned no results, trying DN42 for: {}", q);
                    return process_dn42_query_managed(q).await;
                }
                Err(_) => {
                    log_debug!("Public query failed, trying DN42 for: {}", q);
                    return process_dn42_query_managed(q).await;
                }
                _ => {}
            }

            rdap_whois_result
        }
    };

    // Calculate response time
    let response_time = start_time.elapsed().as_millis() as u64;

    // Send telemetry data if client IP is provided
    #[cfg(feature = "telemetry")]
    if let Some(ip) = client_ip {
        let query_object = query.to_string();
        let query_type_str = crate::core::telemetry::query_type_to_string(query_type);

        let telemetry_data = crate::core::telemetry::TelemetryData::new(
            query_object,
            query_type_str,
            ip,
            response_time,
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

/// Process a plugin query
///
/// This function executes the plugin's handle_query function with the provided input.
/// The timeout is read from the plugin's metadata (default: 5 seconds).
#[cfg(feature = "plugins")]
async fn process_plugin_query(
    suffix: &str,
    base_query: &str,
    _client_ip: Option<String>,
) -> Result<String> {
    use crate::core::query::get_plugin_registry;

    // Get the plugin registry
    let plugin_registry =
        get_plugin_registry().ok_or_else(|| anyhow::anyhow!("Plugin registry not initialized"))?;

    // Find the plugin by suffix
    let plugin = plugin_registry
        .get_plugin(suffix)
        .ok_or_else(|| anyhow::anyhow!("Plugin not found for suffix: {}", suffix))?;

    // Get timeout from plugin metadata (in seconds)
    let timeout_secs = plugin.metadata.plugin.timeout;

    // Execute the plugin with configured timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        execute_plugin(&plugin, base_query),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Plugin execution timeout ({}s)", timeout_secs))??;

    Ok(result)
}

/// Execute a plugin's handle_query function
#[cfg(feature = "plugins")]
async fn execute_plugin(
    plugin: &std::sync::Arc<crate::plugins::LoadedPlugin>,
    query: &str,
) -> Result<String> {
    use mlua::Function;

    let lua = &plugin.lua;

    // Get the handle_query function
    let handle: Function = lua
        .globals()
        .get("handle_query")
        .map_err(|e| anyhow::anyhow!("Plugin missing handle_query function: {}", e))?;

    // Call the function asynchronously
    let result: String = handle
        .call_async(query)
        .await
        .map_err(|e| anyhow::anyhow!("Plugin execution error: {}", e))?;

    Ok(result)
}
