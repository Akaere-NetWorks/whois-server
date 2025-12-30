// WHOIS Server - Telemetry Module
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Telemetry collection module for query analytics

use serde::{ Deserialize, Serialize };
use std::sync::OnceLock;
use crate::{log_debug, log_warn};

/// HTTP request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 5;
/// Maximum time to wait for telemetry task completion before discarding (in seconds)
const TELEMETRY_TASK_TIMEOUT_SECS: u64 = 10;

/// Cached telemetry configuration
static TELEMETRY_CONFIG: OnceLock<TelemetryConfig> = OnceLock::new();

/// Telemetry configuration loaded from environment variables
///
/// Environment variables:
/// - `TELEMETRY_ENABLED`: Set to "true" or "1" to enable telemetry (default: false)
/// - `TELEMETRY_ENDPOINT`: The endpoint URL to send telemetry data to (required if enabled)
/// - `TELEMETRY_BEARER_TOKEN`: The bearer token for authentication (required if enabled)
#[derive(Debug, Clone)]
struct TelemetryConfig {
    enabled: bool,
    endpoint: Option<String>,
    bearer_token: Option<String>,
}

impl TelemetryConfig {
    fn from_env() -> Self {
        let enabled = std::env
            ::var("TELEMETRY_ENABLED")
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1"))
            .unwrap_or(false); // Default: disabled

        let endpoint = std::env::var("TELEMETRY_ENDPOINT").ok();
        let bearer_token = std::env::var("TELEMETRY_BEARER_TOKEN").ok();

        // Warn if enabled but missing required config
        if enabled {
            if endpoint.is_none() {
                log_warn!("TELEMETRY_ENABLED is true but TELEMETRY_ENDPOINT is not set");
            }
            if bearer_token.is_none() {
                log_warn!("TELEMETRY_ENABLED is true but TELEMETRY_BEARER_TOKEN is not set");
            }
        }

        Self {
            enabled,
            endpoint,
            bearer_token,
        }
    }

    fn is_valid(&self) -> bool {
        self.enabled && self.endpoint.is_some() && self.bearer_token.is_some()
    }
}

fn get_config() -> &'static TelemetryConfig {
    TELEMETRY_CONFIG.get_or_init(TelemetryConfig::from_env)
}

/// Telemetry data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryData {
    pub query_object: String,
    pub query_type: String,
    pub client_ip: String,
    pub response_time: u64,
}

impl TelemetryData {
    /// Create a new telemetry data instance
    pub fn new(
        query_object: String,
        query_type: String,
        client_ip: String,
        response_time: u64
    ) -> Self {
        Self {
            query_object,
            query_type,
            client_ip,
            response_time,
        }
    }
}

/// Send telemetry data to the collection endpoint
pub async fn send_telemetry(data: TelemetryData) {
    let config = get_config();

    // Check if telemetry is enabled and properly configured
    if !config.is_valid() {
        log_debug!("Telemetry is disabled or not configured, skipping");
        return;
    }

    // Run telemetry in background with timeout to avoid blocking
    let handle = tokio::spawn(async move {
        if let Err(e) = send_telemetry_internal(data).await {
            log_warn!("Failed to send telemetry data: {}", e);
        }
    });

    // Set a timeout for the telemetry task - if it takes too long, just discard it
    tokio::spawn(async move {
        match
            tokio::time::timeout(
                std::time::Duration::from_secs(TELEMETRY_TASK_TIMEOUT_SECS),
                handle
            ).await
        {
            Ok(_) => {
                // Task completed within timeout
            }
            Err(_) => {
                log_warn!("Telemetry task timed out after {}s, discarding", TELEMETRY_TASK_TIMEOUT_SECS);
            }
        }
    });
}

/// Internal function to send telemetry data
async fn send_telemetry_internal(data: TelemetryData) -> Result<(), anyhow::Error> {
    let config = get_config();

    // Safe to unwrap since is_valid() was checked before calling this
    let endpoint = config.endpoint.as_ref().unwrap();
    let bearer_token = config.bearer_token.as_ref().unwrap();

    log_debug!(
        "Sending telemetry: query={}, type={}, ip={}, time={}ms",
        data.query_object,
        data.query_type,
        data.client_ip,
        data.response_time
    );

    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;

    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", "Akaere-Networks-Whois")
        .json(&data)
        .send().await?;

    if !response.status().is_success() {
        log_warn!("Telemetry endpoint returned error status: {}", response.status());
    } else {
        log_debug!("Telemetry data sent successfully");
    }

    Ok(())
}

/// Convert QueryType to string representation for telemetry
pub fn query_type_to_string(query_type: &crate::core::QueryType) -> String {
    match query_type {
        crate::core::QueryType::Domain(_) => "domain".to_string(),
        crate::core::QueryType::IPv4(_) => "ipv4".to_string(),
        crate::core::QueryType::IPv6(_) => "ipv6".to_string(),
        crate::core::QueryType::ASN(_) => "asn".to_string(),
        crate::core::QueryType::EmailSearch(_) => "email_search".to_string(),
        crate::core::QueryType::BGPTool(_) => "bgptool".to_string(),
        crate::core::QueryType::Geo(_) => "geo".to_string(),
        crate::core::QueryType::RirGeo(_) => "rir_geo".to_string(),
        crate::core::QueryType::Prefixes(_) => "prefixes".to_string(),
        crate::core::QueryType::Radb(_) => "radb".to_string(),
        crate::core::QueryType::Altdb(_) => "altdb".to_string(),
        crate::core::QueryType::Afrinic(_) => "afrinic".to_string(),
        crate::core::QueryType::Apnic(_) => "apnic".to_string(),
        crate::core::QueryType::ArinIrr(_) => "arin_irr".to_string(),
        crate::core::QueryType::Bell(_) => "bell".to_string(),
        crate::core::QueryType::Jpirr(_) => "jpirr".to_string(),
        crate::core::QueryType::Lacnic(_) => "lacnic".to_string(),
        crate::core::QueryType::Level3(_) => "level3".to_string(),
        crate::core::QueryType::Nttcom(_) => "nttcom".to_string(),
        crate::core::QueryType::RipeIrr(_) => "ripe_irr".to_string(),
        crate::core::QueryType::Ris(_) => "ris".to_string(),
        crate::core::QueryType::Tc(_) => "tc".to_string(),
        crate::core::QueryType::Irr(_) => "irr".to_string(),
        crate::core::QueryType::LookingGlass(_) => "looking_glass".to_string(),
        crate::core::QueryType::Rpki(_, _) => "rpki".to_string(),
        crate::core::QueryType::Manrs(_) => "manrs".to_string(),
        crate::core::QueryType::Dns(_) => "dns".to_string(),
        crate::core::QueryType::Trace(_) => "traceroute".to_string(),
        crate::core::QueryType::Ssl(_) => "ssl".to_string(),
        crate::core::QueryType::Crt(_) => "certificate_transparency".to_string(),
        crate::core::QueryType::CfStatus(_) => "cloudflare_status".to_string(),
        crate::core::QueryType::Minecraft(_) => "minecraft".to_string(),
        crate::core::QueryType::MinecraftUser(_) => "minecraft_user".to_string(),
        crate::core::QueryType::Steam(_) => "steam".to_string(),
        crate::core::QueryType::SteamSearch(_) => "steam_search".to_string(),
        crate::core::QueryType::Imdb(_) => "imdb".to_string(),
        crate::core::QueryType::ImdbSearch(_) => "imdb_search".to_string(),
        crate::core::QueryType::Acgc(_) => "acgc".to_string(),
        crate::core::QueryType::Alma(_) => "alma".to_string(),
        crate::core::QueryType::Aosc(_) => "aosc".to_string(),
        crate::core::QueryType::Aur(_) => "aur".to_string(),
        crate::core::QueryType::Debian(_) => "debian".to_string(),
        crate::core::QueryType::Epel(_) => "epel".to_string(),
        crate::core::QueryType::Ubuntu(_) => "ubuntu".to_string(),
        crate::core::QueryType::NixOs(_) => "nixos".to_string(),
        crate::core::QueryType::OpenSuse(_) => "opensuse".to_string(),
        crate::core::QueryType::OpenWrt(_) => "openwrt".to_string(),
        crate::core::QueryType::Npm(_) => "npm".to_string(),
        crate::core::QueryType::Pypi(_) => "pypi".to_string(),
        crate::core::QueryType::Cargo(_) => "cargo".to_string(),
        crate::core::QueryType::Modrinth(_) => "modrinth".to_string(),
        crate::core::QueryType::CurseForge(_) => "curseforge".to_string(),
        crate::core::QueryType::GitHub(_) => "github".to_string(),
        crate::core::QueryType::Wikipedia(_) => "wikipedia".to_string(),
        crate::core::QueryType::Lyric(_) => "lyric".to_string(),
        crate::core::QueryType::Desc(_) => "description".to_string(),
        crate::core::QueryType::PeeringDB(_) => "peeringdb".to_string(),
        crate::core::QueryType::Pen(_) => "pen".to_string(),
        crate::core::QueryType::Rdap(_) => "rdap".to_string(),
        crate::core::QueryType::Pixiv(_) => "pixiv".to_string(),
        crate::core::QueryType::Icp(_) => "icp".to_string(),
        crate::core::QueryType::Meal => "meal".to_string(),
        crate::core::QueryType::MealCN => "meal_cn".to_string(),
        crate::core::QueryType::Ntp(_) => "ntp".to_string(),
        crate::core::QueryType::Ping(_) => "ping".to_string(),
        crate::core::QueryType::Help => "help".to_string(),
        crate::core::QueryType::UpdatePatch => "update_patch".to_string(),
        crate::core::QueryType::Plugin(_, _) => "plugin".to_string(),
        crate::core::QueryType::Unknown(_) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_data_creation() {
        let data = TelemetryData::new(
            "example.com".to_string(),
            "domain".to_string(),
            "1.2.3.4".to_string(),
            150
        );

        assert_eq!(data.query_object, "example.com");
        assert_eq!(data.query_type, "domain");
        assert_eq!(data.client_ip, "1.2.3.4");
        assert_eq!(data.response_time, 150);
    }

    #[test]
    fn test_query_type_to_string() {
        use crate::core::QueryType;

        assert_eq!(query_type_to_string(&QueryType::Domain("example.com".to_string())), "domain");
        assert_eq!(query_type_to_string(&QueryType::Help), "help");
    }
}
