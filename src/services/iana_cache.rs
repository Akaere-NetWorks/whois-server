use std::time::{SystemTime, UNIX_EPOCH};
use crate::storage::lmdb::LmdbStorage;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, error};
use regex::Regex;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IanaReferral {
    pub whois_server: String,
    pub description: String,
    pub cached_at: u64,
    pub as_block_start: Option<u32>,
    pub as_block_end: Option<u32>,
    pub ipv4_block_start: Option<std::net::Ipv4Addr>,
    pub ipv4_block_end: Option<std::net::Ipv4Addr>,
    pub ipv6_block_start: Option<std::net::Ipv6Addr>,
    pub ipv6_block_end: Option<std::net::Ipv6Addr>,
}

impl IanaReferral {
    fn new(whois_server: String, description: String) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            whois_server,
            description,
            cached_at,
            as_block_start: None,
            as_block_end: None,
            ipv4_block_start: None,
            ipv4_block_end: None,
            ipv6_block_start: None,
            ipv6_block_end: None,
        }
    }

    fn new_with_as_block(whois_server: String, description: String, as_block_start: u32, as_block_end: u32) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            whois_server,
            description,
            cached_at,
            as_block_start: Some(as_block_start),
            as_block_end: Some(as_block_end),
            ipv4_block_start: None,
            ipv4_block_end: None,
            ipv6_block_start: None,
            ipv6_block_end: None,
        }
    }

    fn new_with_ipv4_block(whois_server: String, description: String, start: std::net::Ipv4Addr, end: std::net::Ipv4Addr) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            whois_server,
            description,
            cached_at,
            as_block_start: None,
            as_block_end: None,
            ipv4_block_start: Some(start),
            ipv4_block_end: Some(end),
            ipv6_block_start: None,
            ipv6_block_end: None,
        }
    }

    fn new_with_ipv6_block(whois_server: String, description: String, start: std::net::Ipv6Addr, end: std::net::Ipv6Addr) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            whois_server,
            description,
            cached_at,
            as_block_start: None,
            as_block_end: None,
            ipv4_block_start: None,
            ipv4_block_end: None,
            ipv6_block_start: Some(start),
            ipv6_block_end: Some(end),
        }
    }

    fn contains_asn(&self, asn: u32) -> bool {
        match (self.as_block_start, self.as_block_end) {
            (Some(start), Some(end)) => asn >= start && asn <= end,
            _ => false,
        }
    }

    fn contains_ipv4(&self, ip: std::net::Ipv4Addr) -> bool {
        match (self.ipv4_block_start, self.ipv4_block_end) {
            (Some(start), Some(end)) => ip >= start && ip <= end,
            _ => false,
        }
    }

    fn contains_ipv6(&self, ip: std::net::Ipv6Addr) -> bool {
        match (self.ipv6_block_start, self.ipv6_block_end) {
            (Some(start), Some(end)) => ip >= start && ip <= end,
            _ => false,
        }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 7 days = 7 * 24 * 60 * 60 = 604800 seconds
        now - self.cached_at > 604800
    }
}

pub struct IanaCache {
    storage: LmdbStorage,
}

impl IanaCache {
    pub fn new() -> Result<Self> {
        let storage = LmdbStorage::new("./cache/iana_cache")?;
        Ok(Self { storage })
    }

    pub async fn get_whois_server(&self, query: &str) -> Option<String> {
        // For ASN queries, check if any existing block contains this ASN
        if let Some(asn) = self.extract_asn(query) {
            if let Some(server) = self.find_server_for_asn(asn) {
                return Some(server);
            }
        }

        // For IP queries, check if any existing block contains this IP
        if let Some(ip) = self.extract_ip(query) {
            if let Some(server) = self.find_server_for_ip(&ip) {
                return Some(server);
            }
        }

        // Fallback to regular cache key lookup for non-ASN/IP or no block match
        let cache_key = self.get_cache_key(query);
        
        match self.storage.get_json::<IanaReferral>(&cache_key) {
            Ok(Some(referral)) => {
                if !referral.is_expired() {
                    debug!("IANA cache hit for {}: {}", query, referral.whois_server);
                    return Some(referral.whois_server);
                } else {
                    debug!("IANA cache entry expired for {}", query);
                    let _ = self.storage.delete(&cache_key);
                }
            }
            Ok(None) => {
                debug!("IANA cache miss for {}", query);
            }
            Err(e) => {
                warn!("Failed to read IANA cache for {}: {}", query, e);
            }
        }

        // Cache miss or expired, query IANA
        match self.query_iana(query).await {
            Ok(Some(referral)) => {
                let cache_key = if referral.as_block_start.is_some() && referral.as_block_end.is_some() {
                    // Use block range as cache key for ASN blocks
                    format!("asn_block_{}_{}", referral.as_block_start.unwrap(), referral.as_block_end.unwrap())
                } else if referral.ipv4_block_start.is_some() && referral.ipv4_block_end.is_some() {
                    // Use block range as cache key for IPv4 blocks
                    format!("ipv4_block_{}_{}", referral.ipv4_block_start.unwrap(), referral.ipv4_block_end.unwrap())
                } else if referral.ipv6_block_start.is_some() && referral.ipv6_block_end.is_some() {
                    // Use block range as cache key for IPv6 blocks
                    format!("ipv6_block_{}_{}", referral.ipv6_block_start.unwrap(), referral.ipv6_block_end.unwrap())
                } else {
                    cache_key
                };
                
                if let Err(e) = self.storage.put_json(&cache_key, &referral) {
                    warn!("Failed to cache IANA referral for {}: {}", query, e);
                }
                Some(referral.whois_server)
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to query IANA for {}: {}", query, e);
                None
            }
        }
    }

    fn extract_asn(&self, query: &str) -> Option<u32> {
        if query.starts_with("AS") {
            query[2..].parse::<u32>().ok()
        } else {
            query.parse::<u32>().ok()
        }
    }

    fn extract_ip(&self, query: &str) -> Option<std::net::IpAddr> {
        use std::net::IpAddr;
        
        // Try to parse as IP address directly
        if let Ok(ip) = query.parse::<IpAddr>() {
            return Some(ip);
        }
        
        // Try to parse as CIDR and extract the network address
        if let Ok(cidr) = query.parse::<cidr::Ipv4Cidr>() {
            return Some(IpAddr::V4(cidr.first_address()));
        }
        
        if let Ok(cidr) = query.parse::<cidr::Ipv6Cidr>() {
            return Some(IpAddr::V6(cidr.first_address()));
        }
        
        None
    }

    fn find_server_for_asn(&self, asn: u32) -> Option<String> {
        // Search through existing ASN block cache entries
        if let Ok(keys) = self.storage.list_keys() {
            for key in keys {
                if key.starts_with("asn_block_") {
                    if let Ok(Some(referral)) = self.storage.get_json::<IanaReferral>(&key) {
                        if !referral.is_expired() && referral.contains_asn(asn) {
                            debug!("IANA block cache hit for AS{}: {} (block: {:?}-{:?})", 
                                   asn, referral.whois_server, referral.as_block_start, referral.as_block_end);
                            return Some(referral.whois_server);
                        }
                    }
                }
            }
        }
        None
    }

    fn find_server_for_ip(&self, ip: &std::net::IpAddr) -> Option<String> {
        // Search through existing IP block cache entries
        if let Ok(keys) = self.storage.list_keys() {
            for key in keys {
                if key.starts_with("ipv4_block_") || key.starts_with("ipv6_block_") {
                    if let Ok(Some(referral)) = self.storage.get_json::<IanaReferral>(&key) {
                        if !referral.is_expired() {
                            match ip {
                                std::net::IpAddr::V4(ipv4) => {
                                    if referral.contains_ipv4(*ipv4) {
                                        debug!("IANA IPv4 block cache hit for {}: {} (block: {:?}-{:?})", 
                                               ip, referral.whois_server, referral.ipv4_block_start, referral.ipv4_block_end);
                                        return Some(referral.whois_server);
                                    }
                                }
                                std::net::IpAddr::V6(ipv6) => {
                                    if referral.contains_ipv6(*ipv6) {
                                        debug!("IANA IPv6 block cache hit for {}: {} (block: {:?}-{:?})", 
                                               ip, referral.whois_server, referral.ipv6_block_start, referral.ipv6_block_end);
                                        return Some(referral.whois_server);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn refresh_cache_on_failure(&self, query: &str) -> Option<String> {
        debug!("Refreshing IANA cache for {} due to query failure", query);
        
        // For ASN queries, also clear any existing block cache that might contain this ASN
        if let Some(asn) = self.extract_asn(query) {
            if let Ok(keys) = self.storage.list_keys() {
                for key in keys {
                    if key.starts_with("asn_block_") {
                        if let Ok(Some(referral)) = self.storage.get_json::<IanaReferral>(&key) {
                            if referral.contains_asn(asn) {
                                debug!("Clearing expired ASN block cache: {}", key);
                                let _ = self.storage.delete(&key);
                            }
                        }
                    }
                }
            }
        }

        // For IP queries, also clear any existing IP block cache that might contain this IP
        if let Some(ip) = self.extract_ip(query) {
            if let Ok(keys) = self.storage.list_keys() {
                for key in keys {
                    if key.starts_with("ipv4_block_") || key.starts_with("ipv6_block_") {
                        if let Ok(Some(referral)) = self.storage.get_json::<IanaReferral>(&key) {
                            let contains = match ip {
                                std::net::IpAddr::V4(ipv4) => referral.contains_ipv4(ipv4),
                                std::net::IpAddr::V6(ipv6) => referral.contains_ipv6(ipv6),
                            };
                            if contains {
                                debug!("Clearing expired IP block cache: {}", key);
                                let _ = self.storage.delete(&key);
                            }
                        }
                    }
                }
            }
        }
        
        let cache_key = self.get_cache_key(query);
        let _ = self.storage.delete(&cache_key);
        
        match self.query_iana(query).await {
            Ok(Some(referral)) => {
                let cache_key = if referral.as_block_start.is_some() && referral.as_block_end.is_some() {
                    format!("asn_block_{}_{}", referral.as_block_start.unwrap(), referral.as_block_end.unwrap())
                } else if referral.ipv4_block_start.is_some() && referral.ipv4_block_end.is_some() {
                    format!("ipv4_block_{}_{}", referral.ipv4_block_start.unwrap(), referral.ipv4_block_end.unwrap())
                } else if referral.ipv6_block_start.is_some() && referral.ipv6_block_end.is_some() {
                    format!("ipv6_block_{}_{}", referral.ipv6_block_start.unwrap(), referral.ipv6_block_end.unwrap())
                } else {
                    cache_key
                };
                
                if let Err(e) = self.storage.put_json(&cache_key, &referral) {
                    warn!("Failed to cache refreshed IANA referral for {}: {}", query, e);
                }
                Some(referral.whois_server)
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to refresh IANA query for {}: {}", query, e);
                None
            }
        }
    }

    async fn query_iana(&self, query: &str) -> Result<Option<IanaReferral>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};

        debug!("Querying IANA for: {}", query);

        let mut stream = timeout(
            Duration::from_secs(10),
            TcpStream::connect("whois.iana.org:43")
        ).await??;

        let query_str = format!("{}\r\n", query);
        stream.write_all(query_str.as_bytes()).await?;

        let mut response = Vec::new();
        timeout(
            Duration::from_secs(10),
            stream.read_to_end(&mut response)
        ).await??;

        let response_str = String::from_utf8_lossy(&response);
        debug!("IANA response for {}: {}", query, response_str);

        self.parse_iana_response(&response_str)
    }

    fn parse_iana_response(&self, response: &str) -> Result<Option<IanaReferral>> {
        // Look for "refer: whois.server.net" pattern
        let refer_regex = Regex::new(r"(?i)refer:\s*([^\r\n\s]+)")?;
        
        // Look for "whois: whois.server.net" pattern as fallback
        let whois_regex = Regex::new(r"(?i)whois:\s*([^\r\n\s]+)")?;
        
        let whois_server = if let Some(caps) = refer_regex.captures(response) {
            caps.get(1).unwrap().as_str().to_string()
        } else if let Some(caps) = whois_regex.captures(response) {
            caps.get(1).unwrap().as_str().to_string()
        } else {
            debug!("No WHOIS server found in IANA response");
            return Ok(None);
        };

        // Extract description from various possible patterns
        let description = self.extract_description(response);

        // Check for AS block range
        let as_block_regex = Regex::new(r"(?i)as-block:\s*(\d+)-(\d+)")?;
        if let Some(caps) = as_block_regex.captures(response) {
            let start = caps.get(1).unwrap().as_str().parse::<u32>()?;
            let end = caps.get(2).unwrap().as_str().parse::<u32>()?;
            debug!("Found AS block range: {}-{}", start, end);
            return Ok(Some(IanaReferral::new_with_as_block(whois_server, description, start, end)));
        }

        // Check for IPv4 inetnum block
        let ipv4_block_regex = Regex::new(r"(?i)inetnum:\s*([0-9.]+)\s*-\s*([0-9.]+)")?;
        if let Some(caps) = ipv4_block_regex.captures(response) {
            let start_str = caps.get(1).unwrap().as_str();
            let end_str = caps.get(2).unwrap().as_str();
            if let (Ok(start), Ok(end)) = (start_str.parse::<std::net::Ipv4Addr>(), end_str.parse::<std::net::Ipv4Addr>()) {
                debug!("Found IPv4 block range: {}-{}", start, end);
                return Ok(Some(IanaReferral::new_with_ipv4_block(whois_server, description, start, end)));
            }
        }

        // Check for IPv6 inet6num block
        let ipv6_block_regex = Regex::new(r"(?i)inet6num:\s*([0-9a-fA-F:]+)/(\d+)")?;
        if let Some(caps) = ipv6_block_regex.captures(response) {
            let network_str = caps.get(1).unwrap().as_str();
            let prefix_len = caps.get(2).unwrap().as_str().parse::<u8>().unwrap_or(128);
            
            if let Ok(network) = network_str.parse::<std::net::Ipv6Addr>() {
                // Calculate the end address of the IPv6 block
                if let Some(end_addr) = self.calculate_ipv6_block_end(network, prefix_len) {
                    debug!("Found IPv6 block range: {}/{} ({}-{})", network, prefix_len, network, end_addr);
                    return Ok(Some(IanaReferral::new_with_ipv6_block(whois_server, description, network, end_addr)));
                }
            }
        }

        Ok(Some(IanaReferral::new(whois_server, description)))
    }

    fn extract_description(&self, response: &str) -> String {
        // Try to extract organization or description
        let patterns = [
            r"(?i)organisation:\s*([^\r\n]+)",
            r"(?i)organization:\s*([^\r\n]+)", 
            r"(?i)descr:\s*([^\r\n]+)",
            r"(?i)description:\s*([^\r\n]+)",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(caps) = regex.captures(response) {
                    return caps.get(1).unwrap().as_str().trim().to_string();
                }
            }
        }

        "IANA referral".to_string()
    }

    fn calculate_ipv6_block_end(&self, start: std::net::Ipv6Addr, prefix_len: u8) -> Option<std::net::Ipv6Addr> {
        if prefix_len > 128 {
            return None;
        }

        let start_u128 = u128::from(start);
        let host_bits = 128 - prefix_len as u32;
        let mask = if host_bits >= 128 { 0 } else { (1u128 << host_bits) - 1 };
        let end_u128 = start_u128 | mask;
        
        Some(std::net::Ipv6Addr::from(end_u128))
    }

    fn get_cache_key(&self, query: &str) -> String {
        use std::net::IpAddr;
        use cidr::{Ipv4Cidr, Ipv6Cidr};

        // For IP addresses, use the appropriate range key
        if let Ok(ip) = query.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(ipv4) => {
                    // Use /8 for IPv4 caching granularity
                    let octets = ipv4.octets();
                    format!("ipv4_{}", octets[0])
                }
                IpAddr::V6(ipv6) => {
                    // Use /32 for IPv6 caching granularity
                    let segments = ipv6.segments();
                    format!("ipv6_{:x}_{:x}", segments[0], segments[1])
                }
            }
        } else if let Ok(cidr) = query.parse::<Ipv4Cidr>() {
            // For IPv4 CIDR, use first octet
            let octets = cidr.first_address().octets();
            format!("ipv4_{}", octets[0])
        } else if let Ok(cidr) = query.parse::<Ipv6Cidr>() {
            // For IPv6 CIDR, use first two segments
            let segments = cidr.first_address().segments();
            format!("ipv6_{:x}_{:x}", segments[0], segments[1])
        } else if query.starts_with("AS") || query.parse::<u32>().is_ok() {
            // For ASN, use individual ASN numbers since IANA blocks vary
            let asn = if query.starts_with("AS") {
                query[2..].parse::<u32>().unwrap_or(0)
            } else {
                query.parse::<u32>().unwrap_or(0)
            };
            format!("asn_{}", asn)
        } else {
            // For domains, use TLD
            let parts: Vec<&str> = query.split('.').collect();
            if parts.len() > 1 {
                format!("domain_{}", parts.last().unwrap().to_lowercase())
            } else {
                format!("other_{}", query.to_lowercase())
            }
        }
    }

    pub fn clear_expired_entries(&self) -> Result<usize> {
        let mut removed_count = 0;
        let keys = self.storage.list_keys()?;
        
        for key in keys {
            if let Ok(Some(referral)) = self.storage.get_json::<IanaReferral>(&key) {
                if referral.is_expired() {
                    if self.storage.delete(&key).is_ok() {
                        removed_count += 1;
                    }
                }
            }
        }
        
        debug!("Cleared {} expired IANA cache entries", removed_count);
        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let cache = IanaCache::new().unwrap();
        
        // Test IPv4
        assert_eq!(cache.get_cache_key("1.1.1.1"), "ipv4_1");
        assert_eq!(cache.get_cache_key("8.8.8.8"), "ipv4_8");
        
        // Test domains
        assert_eq!(cache.get_cache_key("example.com"), "domain_com");
        assert_eq!(cache.get_cache_key("test.online"), "domain_online");
        
        // Test ASN
        assert_eq!(cache.get_cache_key("AS64512"), "asn_64512");
        assert_eq!(cache.get_cache_key("1234"), "asn_1234");
    }

    #[test]
    fn test_referral_expiration() {
        let referral = IanaReferral {
            whois_server: "whois.example.com".to_string(),
            description: "Test".to_string(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - 604801, // 7 days + 1 second ago
            as_block_start: None,
            as_block_end: None,
            ipv4_block_start: None,
            ipv4_block_end: None,
            ipv6_block_start: None,
            ipv6_block_end: None,
        };
        
        assert!(referral.is_expired());
        
        let fresh_referral = IanaReferral::new(
            "whois.example.com".to_string(),
            "Test".to_string()
        );
        
        assert!(!fresh_referral.is_expired());
    }

    #[test]
    fn test_asn_block_contains() {
        let referral = IanaReferral::new_with_as_block(
            "whois.ripe.net".to_string(),
            "RIPE NCC".to_string(),
            213404,
            214427
        );
        
        assert!(referral.contains_asn(213404)); // Start of range
        assert!(referral.contains_asn(213405)); // Inside range  
        assert!(referral.contains_asn(214427)); // End of range
        assert!(!referral.contains_asn(213403)); // Before range
        assert!(!referral.contains_asn(214428)); // After range
    }

    #[test]
    fn test_ipv4_block_contains() {
        use std::net::Ipv4Addr;
        
        let referral = IanaReferral::new_with_ipv4_block(
            "whois.apnic.net".to_string(),
            "APNIC".to_string(),
            Ipv4Addr::new(1, 0, 0, 0),
            Ipv4Addr::new(1, 255, 255, 255)
        );
        
        assert!(referral.contains_ipv4(Ipv4Addr::new(1, 0, 0, 0))); // Start
        assert!(referral.contains_ipv4(Ipv4Addr::new(1, 1, 1, 1))); // Inside
        assert!(referral.contains_ipv4(Ipv4Addr::new(1, 255, 255, 255))); // End
        assert!(!referral.contains_ipv4(Ipv4Addr::new(0, 255, 255, 255))); // Before
        assert!(!referral.contains_ipv4(Ipv4Addr::new(2, 0, 0, 0))); // After
    }
}