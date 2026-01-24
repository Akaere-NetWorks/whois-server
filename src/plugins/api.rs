//! Plugin-provided APIs
//!
//! This module provides safe APIs that plugins can use:
//! - HTTP client (with domain whitelist enforcement and custom User-Agent)
//! - Cache access (shared LMDB cache)
//! - Environment variable access
//! - Logging functions

use crate::plugins::registry::PluginPermissions;
use mlua::Lua;
use std::collections::{ HashMap, HashSet };

/// Register HTTP client API with domain whitelist enforcement
pub fn register_http_api(lua: &Lua, permissions: &PluginPermissions) -> mlua::Result<()> {
    if !permissions.network {
        // Don't register HTTP API if network permission is not granted
        return Ok(());
    }

    // Build whitelist set for fast lookup
    let whitelist: HashSet<String> = permissions.allowed_domains
        .iter()
        .map(|d| d.to_lowercase())
        .collect();

    // Get custom User-Agent or use default
    let user_agent = permissions.user_agent
        .clone()
        .unwrap_or_else(|| { format!("whois-server-plugin/{}", env!("CARGO_PKG_VERSION")) });

    let http_get = lua.create_async_function(move |_lua, url: String| {
        let whitelist = whitelist.clone();
        let user_agent = user_agent.clone();
        async move {
            // Extract domain from URL
            let domain = extract_domain(&url)?;

            // Check against whitelist
            if !whitelist.is_empty() && !whitelist.contains(&domain.to_lowercase()) {
                return Err(
                    mlua::Error::runtime(
                        format!("Domain '{}' is not in the allowed domains whitelist", domain)
                    )
                );
            }

            // Make HTTP request with custom User-Agent
            let client = reqwest::Client
                ::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|e| mlua::Error::runtime(format!("Failed to create HTTP client: {}", e)))?;

            let response = client
                .get(&url)
                .header("User-Agent", &user_agent)
                .send().await
                .map_err(|e| mlua::Error::runtime(format!("HTTP request failed: {}", e)))?;

            let status = response.status().as_u16();
            let body = response
                .text().await
                .map_err(|e| mlua::Error::runtime(format!("Failed to read response body: {}", e)))?;

            // Return as JSON string
            let result =
                serde_json::json!({
                "status": status,
                "body": body
            });

            Ok(result.to_string())
        }
    })?;

    lua.globals().set("http_get", http_get)?;
    Ok(())
}

/// Register cache access API
///
/// Plugins can read/write to the shared LMDB cache used by the main server.
/// The permissions parameter controls which operations are allowed.
///
/// Note: For now, cache operations are simplified and stored in-memory.
/// Future implementation will integrate with LMDB.
pub fn register_cache_api(lua: &Lua, permissions: &PluginPermissions) -> mlua::Result<()> {
    use std::sync::Mutex;
    use std::collections::HashMap;
    use std::time::{ SystemTime, UNIX_EPOCH };
    use once_cell::sync::Lazy;

    // Simple in-memory cache for plugins
    // TODO: Integrate with LMDB storage
    static CACHE: Lazy<Mutex<HashMap<String, (String, u64)>>> = Lazy::new(||
        Mutex::new(HashMap::new())
    );

    // Register cache_get if read permission is granted
    if permissions.cache_read {
        let cache_get = lua.create_function(move |_lua, key: String| {
            let cache = CACHE.lock().unwrap();
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            if let Some((value, expiry)) = cache.get(&key) {
                if *expiry > now {
                    return Ok(Some(value.clone()));
                }
            }
            Ok(None)
        })?;

        lua.globals().set("cache_get", cache_get)?;
    }

    // Register cache_set if write permission is granted
    if permissions.cache_write {
        let cache_set = lua.create_function(
            move |_lua, (key, value, ttl): (String, String, Option<u32>)| {
                let ttl = ttl.unwrap_or(3600) as u64; // Default 1 hour
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let expiry = now + ttl;

                let mut cache = CACHE.lock().unwrap();
                cache.insert(key, (value, expiry));
                Ok(())
            }
        )?;

        lua.globals().set("cache_set", cache_set)?;
    }

    Ok(())
}

/// Register logging API
///
/// Plugins can log messages that will be handled by the server's logger.
pub fn register_logging_api(lua: &Lua) -> mlua::Result<()> {
    // Create safe logging wrappers
    let log_info_fn = lua.create_function(move |_lua, msg: String| {
        crate::log_info!("[plugin] {}", msg);
        Ok(())
    })?;

    let log_warn_fn = lua.create_function(move |_lua, msg: String| {
        crate::log_warn!("[plugin] {}", msg);
        Ok(())
    })?;

    let log_error_fn = lua.create_function(move |_lua, msg: String| {
        crate::log_warn!("[plugin] ERROR: {}", msg);
        Ok(())
    })?;

    lua.globals().set("log_info", log_info_fn)?;
    lua.globals().set("log_warn", log_warn_fn)?;
    lua.globals().set("log_error", log_error_fn)?;

    Ok(())
}

/// Register environment variable API
///
/// Plugins can access environment variables that were loaded from .plugins.env
/// and are explicitly allowed in their metadata.
pub fn register_env_api(lua: &Lua, env_vars: &HashMap<String, String>) -> mlua::Result<()> {
    // Clone for use in closures to avoid lifetime issues
    let env_vars_for_get = env_vars.clone();
    let env_vars_for_list = env_vars.clone();

    // Create a read-only env_get function
    let env_get = lua.create_function(move |_lua, key: String| {
        env_vars_for_get
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                mlua::Error::runtime(
                    format!("Environment variable '{}' not found or not allowed", key)
                )
            })
    })?;

    lua.globals().set("env_get", env_get)?;

    // Also provide env_list to show available environment variables
    let env_list = lua.create_function(move |_lua, ()| {
        let keys: Vec<String> = env_vars_for_list.keys().cloned().collect();
        Ok(keys)
    })?;

    lua.globals().set("env_list", env_list)?;

    Ok(())
}

/// Extract domain from URL
///
/// # Examples
/// - `https://example.com/path` -> `example.com`
/// - `http://api.example.com:8080/v1` -> `api.example.com`
fn extract_domain(url: &str) -> mlua::Result<String> {
    // Parse URL
    let parsed = url::Url
        ::parse(url)
        .map_err(|e| mlua::Error::runtime(format!("Invalid URL: {}", e)))?;

    // Get host (domain)
    let host = parsed.host_str().ok_or_else(|| mlua::Error::runtime("URL has no host"))?;

    Ok(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path").unwrap(), "example.com");
        assert_eq!(extract_domain("http://api.example.com:8080/v1").unwrap(), "api.example.com");
        assert_eq!(extract_domain("https://sub.domain.co.uk").unwrap(), "sub.domain.co.uk");
    }

    #[test]
    fn test_extract_domain_invalid() {
        assert!(extract_domain("not a url").is_err());
        assert!(extract_domain("://no-protocol").is_err());
    }
}
