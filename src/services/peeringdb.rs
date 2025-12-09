use crate::config::{PEERINGDB_CACHE_TTL, PEERINGDB_LMDB_PATH};
use crate::storage::lmdb::LmdbStorage;
use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

#[derive(Debug, Deserialize, Serialize)]
pub struct PeeringDBResponse<T> {
    pub data: Vec<T>,
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeeringDBCacheEntry {
    pub response: String,
    pub cached_at: u64,
}

impl PeeringDBCacheEntry {
    fn new(response: String) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after Unix epoch")
            .as_secs();
        Self {
            response,
            cached_at,
        }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after Unix epoch")
            .as_secs();
        (now - self.cached_at) > PEERINGDB_CACHE_TTL
    }
}

/// PeeringDB cache manager
pub struct PeeringDBCache {
    storage: LmdbStorage,
}

impl PeeringDBCache {
    pub fn new() -> Result<Self> {
        let storage = LmdbStorage::new(PEERINGDB_LMDB_PATH)?;
        Ok(Self { storage })
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if let Some(cached_data) = self.storage.get(key)? {
            let cache_entry: PeeringDBCacheEntry = serde_json::from_str(&cached_data)?;
            if !cache_entry.is_expired() {
                debug!("PeeringDB cache hit for key: {}", key);
                return Ok(Some(cache_entry.response));
            } else {
                debug!("PeeringDB cache expired for key: {}", key);
                // Remove expired entry
                self.storage.delete(key).ok(); // Ignore errors
            }
        }
        debug!("PeeringDB cache miss for key: {}", key);
        Ok(None)
    }

    pub fn put(&self, key: &str, response: &str) -> Result<()> {
        let cache_entry = PeeringDBCacheEntry::new(response.to_string());
        let cache_data = serde_json::to_string(&cache_entry)?;
        self.storage.put(key, &cache_data)?;
        debug!("PeeringDB cached response for key: {}", key);
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkIXLAN {
    pub id: u32,
    pub ix_id: u32,
    pub name: Option<String>,
    pub ixlan_id: u32,
    pub notes: Option<String>,
    pub speed: u32,
    pub asn: u32,
    pub ipaddr4: Option<String>,
    pub ipaddr6: Option<String>,
    pub is_rs_peer: bool,
    pub bfd_support: bool,
    pub operational: bool,
    pub created: String,
    pub updated: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkInfo {
    pub id: u32,
    pub org_id: u32,
    pub name: String,
    pub aka: Option<String>,
    pub name_long: Option<String>,
    pub asn: u32,
    pub website: Option<String>,
    pub social_media: Option<Vec<Value>>,
    pub looking_glass: Option<String>,
    pub route_server: Option<String>,
    pub irr_as_set: Option<String>,
    pub info_traffic: Option<String>,
    pub info_ratio: Option<String>,
    pub info_scope: Option<String>,
    pub info_type: Option<String>,
    pub info_types: Option<Vec<String>>,
    pub info_prefixes4: Option<u32>,
    pub info_prefixes6: Option<u32>,
    pub info_unicast: Option<bool>,
    pub info_multicast: Option<bool>,
    pub info_ipv6: Option<bool>,
    pub info_never_via_route_servers: Option<bool>,
    pub ix_count: Option<u32>,
    pub fac_count: Option<u32>,
    pub policy_url: Option<String>,
    pub policy_general: Option<String>,
    pub policy_locations: Option<String>,
    pub policy_ratio: Option<bool>,
    pub policy_contracts: Option<String>,
    pub notes: Option<String>,
    pub allow_ixp_update: Option<bool>,
    pub status: String,
    pub created: String,
    pub updated: String,
    pub netixlan_updated: Option<String>,
    pub netfac_updated: Option<String>,
    pub poc_updated: Option<String>,
    pub netixlan_set: Option<Vec<NetworkIXLAN>>,
    pub netfac_set: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InternetExchangeInfo {
    pub id: u32,
    pub org_id: u32,
    pub name: String,
    pub aka: Option<String>,
    pub name_long: Option<String>,
    pub city: String,
    pub country: String,
    pub region_continent: String,
    pub media: Option<String>,
    pub notes: Option<String>,
    pub proto_unicast: Option<bool>,
    pub proto_multicast: Option<bool>,
    pub proto_ipv6: Option<bool>,
    pub website: Option<String>,
    pub social_media: Option<Vec<Value>>,
    pub url_stats: Option<String>,
    pub tech_email: Option<String>,
    pub tech_phone: Option<String>,
    pub policy_email: Option<String>,
    pub policy_phone: Option<String>,
    pub sales_email: Option<String>,
    pub sales_phone: Option<String>,
    pub net_count: Option<u32>,
    pub fac_count: Option<u32>,
    pub ixf_net_count: Option<u32>,
    pub ixf_last_import: Option<String>,
    pub ixf_import_request: Option<String>,
    pub ixf_import_request_status: Option<String>,
    pub service_level: Option<String>,
    pub terms: Option<String>,
    pub status_dashboard: Option<String>,
    pub logo: Option<String>,
    pub status: String,
    pub created: String,
    pub updated: String,
    pub ixlan_set: Option<Vec<Value>>,
    pub ixfac_set: Option<Vec<Value>>,
}

/// Query PeeringDB API for ASN information
pub async fn query_peeringdb_asn(asn: &str) -> Result<String> {
    // Parse ASN number (remove AS prefix if present)
    let asn_number = if asn.to_uppercase().starts_with("AS") {
        &asn[2..]
    } else {
        asn
    };

    let asn_num: u32 = asn_number
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid ASN format: {}", asn))?;

    debug!("Querying PeeringDB for ASN: {}", asn_num);

    // Check cache first
    let cache_key = format!("asn:{}", asn_num);
    let cache = PeeringDBCache::new()?;

    if let Some(cached_response) = cache.get(&cache_key)? {
        debug!("Returning cached PeeringDB response for ASN: {}", asn_num);
        return Ok(cached_response);
    }

    let client = reqwest::Client::new();
    let url = format!("https://www.peeringdb.com/api/net?asn={}&depth=2", asn_num);

    debug!("PeeringDB API URL: {}", url);

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36")
        .send()
        .await?;

    debug!("PeeringDB API response status: {}", response.status());

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        debug!("PeeringDB API error response: {}", error_body);
        return Err(anyhow::anyhow!(
            "PeeringDB API request failed: {} - {}",
            status,
            error_body
        ));
    }

    let body = response.text().await?;
    let pdb_response: PeeringDBResponse<NetworkInfo> = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse PeeringDB response: {}", e))?;

    if pdb_response.data.is_empty() {
        let no_data_response = format!(
            "% No network information found for ASN {} in PeeringDB",
            asn_num
        );
        // Cache negative response for shorter time (1 hour)
        let cache_entry = PeeringDBCacheEntry {
            response: no_data_response.clone(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time should be after Unix epoch")
                .as_secs(),
        };
        let cache_data = serde_json::to_string(&cache_entry).unwrap_or_default();
        cache.put(&cache_key, &cache_data).ok(); // Ignore cache errors
        return Ok(no_data_response);
    }

    let mut result = String::new();
    result.push_str(&format!(
        "% PeeringDB Network Information for AS{}\n",
        asn_num
    ));
    result.push_str("% Source: https://www.peeringdb.com/\n\n");

    for network in &pdb_response.data {
        result.push_str(&format_network_info(network));
        result.push('\n');
    }

    // Cache the successful response
    cache.put(&cache_key, &result).ok(); // Ignore cache errors

    Ok(result)
}

/// Query PeeringDB API for Internet Exchange information
pub async fn query_peeringdb_ix(ix_id: &str) -> Result<String> {
    let ix_num: u32 = ix_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid IX ID format: {}", ix_id))?;

    debug!("Querying PeeringDB for IX ID: {}", ix_num);

    // Check cache first
    let cache_key = format!("ix:{}", ix_num);
    let cache = PeeringDBCache::new()?;

    if let Some(cached_response) = cache.get(&cache_key)? {
        debug!("Returning cached PeeringDB response for IX: {}", ix_num);
        return Ok(cached_response);
    }

    let client = reqwest::Client::new();
    let url = format!("https://www.peeringdb.com/api/ix?id={}&depth=2", ix_num);

    debug!("PeeringDB API URL: {}", url);

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36")
        .send()
        .await?;

    debug!("PeeringDB API response status: {}", response.status());

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        debug!("PeeringDB API error response: {}", error_body);
        return Err(anyhow::anyhow!(
            "PeeringDB API request failed: {} - {}",
            status,
            error_body
        ));
    }

    let body = response.text().await?;
    let pdb_response: PeeringDBResponse<InternetExchangeInfo> = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse PeeringDB response: {}", e))?;

    if pdb_response.data.is_empty() {
        let no_data_response = format!(
            "% No Internet Exchange information found for ID {} in PeeringDB",
            ix_num
        );
        // Cache negative response for shorter time (1 hour)
        let cache_entry = PeeringDBCacheEntry {
            response: no_data_response.clone(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time should be after Unix epoch")
                .as_secs(),
        };
        let cache_data = serde_json::to_string(&cache_entry).unwrap_or_default();
        cache.put(&cache_key, &cache_data).ok(); // Ignore cache errors
        return Ok(no_data_response);
    }

    let mut result = String::new();
    result.push_str(&format!(
        "% PeeringDB Internet Exchange Information for ID {}\n",
        ix_num
    ));
    result.push_str("% Source: https://www.peeringdb.com/\n\n");

    for ix in &pdb_response.data {
        result.push_str(&format_ix_info(ix));
        result.push('\n');
    }

    // Cache the successful response
    cache.put(&cache_key, &result).ok(); // Ignore cache errors

    Ok(result)
}

/// Format network information for display
fn format_network_info(network: &NetworkInfo) -> String {
    let mut info = String::new();

    info.push_str(&format!("ASN:                AS{}\n", network.asn));
    info.push_str(&format!("Network Name:       {}\n", network.name));

    if let Some(name_long) = &network.name_long {
        info.push_str(&format!("Full Name:          {}\n", name_long));
    }

    if let Some(aka) = &network.aka {
        info.push_str(&format!("Also Known As:      {}\n", aka));
    }

    info.push_str(&format!("PeeringDB ID:       {}\n", network.id));
    info.push_str(&format!("Organization ID:    {}\n", network.org_id));

    if let Some(website) = &network.website {
        info.push_str(&format!("Website:            {}\n", website));
    }

    if let Some(info_type) = &network.info_type {
        info.push_str(&format!("Network Type:       {}\n", info_type));
    }

    if let Some(info_scope) = &network.info_scope {
        info.push_str(&format!("Geographic Scope:   {}\n", info_scope));
    }

    if let Some(info_traffic) = &network.info_traffic {
        info.push_str(&format!("Traffic Volume:     {}\n", info_traffic));
    }

    if let Some(info_ratio) = &network.info_ratio {
        info.push_str(&format!("Traffic Ratio:      {}\n", info_ratio));
    }

    if let Some(prefixes4) = network.info_prefixes4 {
        info.push_str(&format!("IPv4 Prefixes:      {}\n", prefixes4));
    }

    if let Some(prefixes6) = network.info_prefixes6 {
        info.push_str(&format!("IPv6 Prefixes:      {}\n", prefixes6));
    }

    if let Some(policy_url) = &network.policy_url {
        info.push_str(&format!("Policy URL:         {}\n", policy_url));
    }

    if let Some(policy_general) = &network.policy_general {
        info.push_str(&format!("Peering Policy:     {}\n", policy_general));
    }

    if let Some(policy_locations) = &network.policy_locations {
        info.push_str(&format!("Location Policy:    {}\n", policy_locations));
    }

    if let Some(ipv6) = network.info_ipv6 {
        info.push_str(&format!(
            "IPv6 Support:       {}\n",
            if ipv6 { "Yes" } else { "No" }
        ));
    }

    if let Some(unicast) = network.info_unicast {
        info.push_str(&format!(
            "Unicast:            {}\n",
            if unicast { "Yes" } else { "No" }
        ));
    }

    if let Some(multicast) = network.info_multicast {
        info.push_str(&format!(
            "Multicast:          {}\n",
            if multicast { "Yes" } else { "No" }
        ));
    }

    if let Some(never_route_servers) = network.info_never_via_route_servers {
        info.push_str(&format!(
            "Route Server Policy: {}\n",
            if never_route_servers {
                "Never via route servers"
            } else {
                "Route servers OK"
            }
        ));
    }

    if let Some(looking_glass) = &network.looking_glass {
        if !looking_glass.trim().is_empty() {
            info.push_str(&format!("Looking Glass:      {}\n", looking_glass.trim()));
        }
    }

    if let Some(route_server) = &network.route_server {
        if !route_server.trim().is_empty() {
            info.push_str(&format!("Route Server:       {}\n", route_server.trim()));
        }
    }

    if let Some(irr_as_set) = &network.irr_as_set {
        if !irr_as_set.trim().is_empty() {
            info.push_str(&format!("IRR AS-SET:         {}\n", irr_as_set.trim()));
        }
    }

    if let Some(notes) = &network.notes {
        if !notes.trim().is_empty() {
            info.push_str(&format!("Notes:              {}\n", notes.trim()));
        }
    }

    // Show exchange and facility counts if available
    if let Some(ix_count) = network.ix_count {
        info.push_str(&format!("Exchange Presence:  {} exchanges\n", ix_count));
    }

    if let Some(fac_count) = network.fac_count {
        info.push_str(&format!("Facility Presence:  {} facilities\n", fac_count));
    }

    // Show detailed IX information if available
    if let Some(netixlan_set) = &network.netixlan_set {
        if !netixlan_set.is_empty() {
            info.push_str("\nInternet Exchange Connections:\n");
            info.push_str(&format!("{:-<60}\n", ""));

            for (i, ix_connection) in netixlan_set.iter().enumerate() {
                info.push_str(&format!(
                    "IX #{}: {}\n",
                    i + 1,
                    ix_connection
                        .name
                        .as_ref()
                        .unwrap_or(&format!("IX ID {}", ix_connection.ix_id))
                ));
                info.push_str(&format!("  IX ID:            {}\n", ix_connection.ix_id));
                info.push_str(&format!(
                    "  Speed:            {} Mbps\n",
                    ix_connection.speed
                ));

                if let Some(ipv4) = &ix_connection.ipaddr4 {
                    info.push_str(&format!("  IPv4 Address:     {}\n", ipv4));
                }

                if let Some(ipv6) = &ix_connection.ipaddr6 {
                    info.push_str(&format!("  IPv6 Address:     {}\n", ipv6));
                }

                info.push_str(&format!(
                    "  Route Server:     {}\n",
                    if ix_connection.is_rs_peer {
                        "Yes"
                    } else {
                        "No"
                    }
                ));
                info.push_str(&format!(
                    "  BFD Support:      {}\n",
                    if ix_connection.bfd_support {
                        "Yes"
                    } else {
                        "No"
                    }
                ));
                info.push_str(&format!(
                    "  Operational:      {}\n",
                    if ix_connection.operational {
                        "Yes"
                    } else {
                        "No"
                    }
                ));

                if let Some(notes) = &ix_connection.notes {
                    if !notes.trim().is_empty() {
                        info.push_str(&format!("  Notes:            {}\n", notes.trim()));
                    }
                }

                if i < netixlan_set.len() - 1 {
                    info.push('\n');
                }
            }
        }
    }

    info.push('\n');

    info.push_str(&format!("Status:             {}\n", network.status));
    info.push_str(&format!("Created:            {}\n", network.created));
    info.push_str(&format!("Last Updated:       {}\n", network.updated));

    info
}

/// Format Internet Exchange information for display
fn format_ix_info(ix: &InternetExchangeInfo) -> String {
    let mut info = String::new();

    info.push_str(&format!("Exchange Name:      {}\n", ix.name));

    if let Some(name_long) = &ix.name_long {
        info.push_str(&format!("Full Name:          {}\n", name_long));
    }

    if let Some(aka) = &ix.aka {
        info.push_str(&format!("Also Known As:      {}\n", aka));
    }

    info.push_str(&format!("PeeringDB ID:       {}\n", ix.id));
    info.push_str(&format!("Organization ID:    {}\n", ix.org_id));
    info.push_str(&format!("City:               {}\n", ix.city));
    info.push_str(&format!("Country:            {}\n", ix.country));
    info.push_str(&format!("Continent:          {}\n", ix.region_continent));

    if let Some(media) = &ix.media {
        info.push_str(&format!("Media Type:         {}\n", media));
    }

    if let Some(website) = &ix.website {
        info.push_str(&format!("Website:            {}\n", website));
    }

    if let Some(url_stats) = &ix.url_stats {
        info.push_str(&format!("Statistics URL:     {}\n", url_stats));
    }

    if let Some(tech_email) = &ix.tech_email {
        info.push_str(&format!("Technical Email:    {}\n", tech_email));
    }

    if let Some(tech_phone) = &ix.tech_phone {
        info.push_str(&format!("Technical Phone:    {}\n", tech_phone));
    }

    if let Some(policy_email) = &ix.policy_email {
        info.push_str(&format!("Policy Email:       {}\n", policy_email));
    }

    if let Some(policy_phone) = &ix.policy_phone {
        info.push_str(&format!("Policy Phone:       {}\n", policy_phone));
    }

    if let Some(sales_email) = &ix.sales_email {
        info.push_str(&format!("Sales Email:        {}\n", sales_email));
    }

    if let Some(sales_phone) = &ix.sales_phone {
        info.push_str(&format!("Sales Phone:        {}\n", sales_phone));
    }

    if let Some(service_level) = &ix.service_level {
        info.push_str(&format!("Service Level:      {}\n", service_level));
    }

    if let Some(terms) = &ix.terms {
        info.push_str(&format!("Terms:              {}\n", terms));
    }

    if let Some(status_dashboard) = &ix.status_dashboard {
        info.push_str(&format!("Status Dashboard:   {}\n", status_dashboard));
    }

    if let Some(ipv6) = ix.proto_ipv6 {
        info.push_str(&format!(
            "IPv6 Support:       {}\n",
            if ipv6 { "Yes" } else { "No" }
        ));
    }

    if let Some(unicast) = ix.proto_unicast {
        info.push_str(&format!(
            "Unicast:            {}\n",
            if unicast { "Yes" } else { "No" }
        ));
    }

    if let Some(multicast) = ix.proto_multicast {
        info.push_str(&format!(
            "Multicast:          {}\n",
            if multicast { "Yes" } else { "No" }
        ));
    }

    // Show network and facility counts
    if let Some(net_count) = ix.net_count {
        info.push_str(&format!("Connected Networks: {} networks\n", net_count));
    }

    if let Some(fac_count) = ix.fac_count {
        info.push_str(&format!("Connected Facilities: {} facilities\n", fac_count));
    }

    if let Some(notes) = &ix.notes {
        if !notes.trim().is_empty() {
            info.push_str(&format!("Notes:\n{}\n", notes.trim()));
        }
    }

    info.push_str(&format!("Status:             {}\n", ix.status));
    info.push_str(&format!("Created:            {}\n", ix.created));
    info.push_str(&format!("Last Updated:       {}\n", ix.updated));

    info
}

/// Process PeeringDB queries based on query type
pub async fn process_peeringdb_query(query: &str) -> Result<String> {
    debug!("Processing PeeringDB query: {}", query);

    // Check if query looks like an ASN (starts with AS)
    if query.to_uppercase().starts_with("AS") {
        query_peeringdb_asn(query).await
    } else if query.parse::<u32>().is_ok() {
        // Pure numbers are IX IDs
        query_peeringdb_ix(query).await
    } else {
        Err(anyhow::anyhow!(
            "Invalid PeeringDB query format. Use 'AS12345' for ASN or '1234' for IX ID"
        ))
    }
}
