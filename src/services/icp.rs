// WHOIS Server - ICP Filing Query Service
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ICP (Internet Content Provider) filing query service for Chinese domains
//! Queries multiple external providers for ICP registration information

use crate::config::{ICP_CACHE_TTL, ICP_LMDB_PATH};
use crate::storage::lmdb::LmdbStorage;
use crate::{log_debug, log_error};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RETRIES: u32 = 3;

/// ICP cache entry with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICPCacheEntry {
    pub response: String,
    pub cached_at: u64,
}

impl ICPCacheEntry {
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
        (now - self.cached_at) > ICP_CACHE_TTL
    }
}

/// ICP cache manager
pub struct ICPCache {
    storage: LmdbStorage,
}

impl ICPCache {
    pub fn new() -> Result<Self> {
        let storage = LmdbStorage::new(ICP_LMDB_PATH)?;
        Ok(Self { storage })
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if let Some(cached_data) = self.storage.get(key)? {
            let cache_entry: ICPCacheEntry = serde_json::from_str(&cached_data)?;
            if !cache_entry.is_expired() {
                log_debug!("ICP cache hit for key: {}", key);
                return Ok(Some(cache_entry.response));
            } else {
                log_debug!("ICP cache expired for key: {}", key);
                // Remove expired entry
                self.storage.delete(key).ok();
            }
        }
        log_debug!("ICP cache miss for key: {}", key);
        Ok(None)
    }

    pub fn put(&self, key: &str, response: &str) -> Result<()> {
        let cache_entry = ICPCacheEntry::new(response.to_string());
        let cache_data = serde_json::to_string(&cache_entry)?;
        self.storage.put(key, &cache_data)?;
        log_debug!("ICP cached response for key: {}", key);
        Ok(())
    }
}

// Baidu ICP API structures
#[derive(Serialize)]
struct BaiduICPRequest {
    host: String,
    domain: String,
}

#[derive(Deserialize)]
struct BaiduICPResponse {
    result: BaiduICPResult,
    status: i32,
    success: bool,
}

#[derive(Deserialize)]
struct BaiduICPResult {
    #[serde(rename = "auditTime")]
    audit_time: String,
    company: String,
    #[allow(dead_code)]
    domain: String,
    exists: bool,
    number: String,
    #[serde(rename = "siteName")]
    site_name: String,
    #[serde(rename = "type")]
    icp_type: String,
}

// DNSPod ICP API structures
#[derive(Serialize)]
struct DNSPodRequest {
    #[serde(rename = "ori_domain")]
    ori_domain: String,
    api: String,
}

#[derive(Deserialize)]
struct DNSPodResponse {
    code: i32,
    status: DNSPodStatus,
    data: DNSPodData,
}

#[derive(Deserialize)]
struct DNSPodStatus {
    #[serde(rename = "code")]
    _code: String,
    message: String,
    #[serde(rename = "created_at")]
    _created_at: String,
}

#[derive(Deserialize)]
struct DNSPodData {
    #[serde(rename = "if_beian")]
    if_beian: DNSPodBeian,
}

#[derive(Deserialize)]
struct DNSPodBeian {
    status: String,
    info: String,
}

/// Clean and validate domain input
fn clean_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().to_lowercase();

    // Remove common prefixes
    let domain = domain
        .strip_prefix("http://")
        .or_else(|| domain.strip_prefix("https://"))
        .or_else(|| domain.strip_prefix("www."))
        .unwrap_or(&domain)
        .to_string();

    // Remove port if present
    let domain = domain.split(':').next().unwrap_or(&domain);

    // Validate domain format
    let domain_regex = Regex::new(r"^[\w\-]+(\.[\w\-]+)+[\w\-]*$").unwrap();
    if !domain_regex.is_match(domain) {
        return Err(anyhow::anyhow!("Invalid domain format: {}", domain));
    }

    Ok(domain.to_string())
}

/// Query Baidu ICP API
async fn query_baidu_icp(domain: &str) -> Result<String> {
    const BAIDU_ICP_URL: &str = "https://cloud.baidu.com/api/sme/aladdin/icpquery";

    let req_body = BaiduICPRequest {
        host: domain.to_string(),
        domain: domain.to_string(),
    };

    let client = reqwest::Client::new();
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        let response = client
            .post(BAIDU_ICP_URL)
            .header("Host", "cloud.baidu.com")
            .header("Origin", "https://cloud.baidu.com")
            .header(
                "Referer",
                "https://cloud.baidu.com/product/bcd/toolPack.html?pageTitle=whois",
            )
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
            )
            .json(&req_body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let text = resp.text().await?;
                    let baidu_resp: BaiduICPResponse = serde_json::from_str(&text)
                        .map_err(|e| anyhow::anyhow!("Failed to parse Baidu response: {}", e))?;

                    if !baidu_resp.success {
                        last_error = Some(anyhow::anyhow!(
                            "Baidu API returned error: status={}, success={}",
                            baidu_resp.status,
                            baidu_resp.success
                        ));

                        if attempt < MAX_RETRIES {
                            log_debug!("Retrying Baidu API, attempt {}", attempt);
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            continue;
                        }
                        break;
                    }

                    return Ok(format_icp_result(
                        domain,
                        "baidu",
                        baidu_resp.result.exists,
                        &baidu_resp.result.number,
                        &baidu_resp.result.company,
                        Some(&baidu_resp.result.site_name),
                        Some(&baidu_resp.result.audit_time),
                        Some(&baidu_resp.result.icp_type),
                    ));
                } else {
                    last_error = Some(anyhow::anyhow!(
                        "Baidu HTTP error: status={}",
                        resp.status()
                    ));
                }
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("Baidu request failed: {}", e));
            }
        }

        if attempt < MAX_RETRIES {
            log_debug!("Retrying Baidu API, attempt {}", attempt);
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed after {} retries", MAX_RETRIES)))
}

/// Query DNSPod ICP API
async fn query_dnspod_icp(domain: &str) -> Result<String> {
    const DNSPOD_URL: &str = "https://www.dnspod.cn/cgi/dnsapi";

    let req_body = DNSPodRequest {
        ori_domain: domain.to_string(),
        api: "Tools.Check.Website".to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post(DNSPOD_URL)
        .header("Origin", "https://tool.dnspod.cn")
        .header("Referer", "https://tool.dnspod.cn")
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
        )
        .json(&req_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("DNSPod request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("DNSPod HTTP error: status={}", response.status()));
    }

    let text = response.text().await?;
    let dnspod_resp: DNSPodResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse DNSPod response: {}", e))?;

    if dnspod_resp.code != 0 {
        return Err(anyhow::anyhow!(
            "DNSPod API returned error: code={}, message={}",
            dnspod_resp.code,
            dnspod_resp.status.message
        ));
    }

    let exists = dnspod_resp.data.if_beian.status == "1";
    let number = if exists {
        dnspod_resp.data.if_beian.info
    } else {
        String::new()
    };

    Ok(format_icp_result(
        domain,
        "dnspod",
        exists,
        &number,
        "", // DNSPod doesn't return company info
        None,
        None,
        None,
    ))
}

/// Format ICP result in WHOIS style
fn format_icp_result(
    domain: &str,
    provider: &str,
    exists: bool,
    number: &str,
    company: &str,
    site_name: Option<&str>,
    audit_time: Option<&str>,
    icp_type: Option<&str>,
) -> String {
    let mut result = String::new();

    result.push_str(&format!("% ICP Filing Information for {}\n", domain));
    result.push_str(&format!("% Data source: {}\n\n", provider));

    if exists {
        if !number.is_empty() {
            result.push_str(&format!("ICP Number:      {}\n", number));
        }
        if !company.is_empty() {
            result.push_str(&format!("Company:          {}\n", company));
        }
        if let Some(site) = site_name {
            if !site.is_empty() {
                result.push_str(&format!("Site Name:        {}\n", site));
            }
        }
        if let Some(typ) = icp_type {
            if !typ.is_empty() {
                result.push_str(&format!("Type:             {}\n", typ));
            }
        }
        if let Some(audit) = audit_time {
            if !audit.is_empty() {
                result.push_str(&format!("Audit Time:       {}\n", audit));
            }
        }
    } else {
        result.push_str("Status:           No ICP filing found\n");
    }

    result.push('\n');
    result
}

/// Process ICP query (public interface)
pub async fn process_icp_query(domain: &str) -> String {
    // Clean domain input
    let clean_domain = match clean_domain(domain) {
        Ok(d) => d,
        Err(e) => {
            log_error!("Invalid domain for ICP query: {} - {}", domain, e);
            return format!("% ICP Query Failed\n% Error: Invalid domain format: {}\n", domain);
        }
    };

    log_debug!("Querying ICP information for domain: {}", clean_domain);

    // Check cache first
    let cache = match ICPCache::new() {
        Ok(c) => c,
        Err(e) => {
            log_error!("Failed to initialize ICP cache: {}", e);
            return format!(
                "% ICP Query Failed for {}\n% Error: Cache initialization failed\n",
                clean_domain
            );
        }
    };

    let cache_key = format!("domain:{}", clean_domain);
    if let Ok(Some(cached_response)) = cache.get(&cache_key) {
        log_debug!("Returning cached ICP response for domain: {}", clean_domain);
        return cached_response;
    }

    // Try providers in order: baidu -> dnspod
    let providers = vec!["baidu", "dnspod"];

    for provider in providers {
        log_debug!("Trying provider {} for domain: {}", provider, clean_domain);

        let result = match provider {
            "baidu" => query_baidu_icp(&clean_domain).await,
            "dnspod" => query_dnspod_icp(&clean_domain).await,
            _ => continue,
        };

        match result {
            Ok(response) => {
                log_debug!(
                    "Query success with provider {} for domain: {}",
                    provider,
                    clean_domain
                );

                // Cache the result
                let cache_result = cache.put(&cache_key, &response);
                if let Err(e) = cache_result {
                    log_error!("Failed to cache ICP result: {}", e);
                }

                return response;
            }
            Err(e) => {
                log_debug!(
                    "Query failed with provider {} for domain: {} - {}",
                    provider,
                    clean_domain,
                    e
                );
            }
        }
    }

    log_error!("All ICP providers failed for domain: {}", clean_domain);
    format!(
        "% ICP Query Failed for {}\n% Error: All providers returned no results\n",
        clean_domain
    )
}
