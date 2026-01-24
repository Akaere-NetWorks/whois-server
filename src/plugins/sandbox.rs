//! Security sandbox for plugin execution
//!
//! This module creates a secure Lua environment that restricts dangerous operations
//! while providing safe APIs for plugins.

use crate::plugins::api::{
    register_cache_api,
    register_env_api,
    register_http_api,
    register_logging_api,
};
use crate::plugins::registry::PluginMetadata;
use mlua::{ Table, Value };
use std::collections::HashMap;

/// Create a secure Lua state for plugin execution
///
/// This function:
/// - Removes dangerous libraries (os, io, load, etc.)
/// - Restricts package loading
/// - Sets memory limits
/// - Registers safe APIs (HTTP, cache, logging, environment variables)
pub fn create_secure_lua_state(
    metadata: &PluginMetadata,
    env_vars: &HashMap<String, String>
) -> mlua::Result<mlua::Lua> {
    let lua = mlua::Lua::new();

    // Remove dangerous libraries
    lua.globals().set("os", Value::Nil)?;
    lua.globals().set("io", Value::Nil)?;
    lua.globals().set("load", Value::Nil)?;
    lua.globals().set("loadfile", Value::Nil)?;
    lua.globals().set("dofile", Value::Nil)?;
    lua.globals().set("print", Value::Nil)?;

    // Restrict package module to prevent loading external libraries
    if let Ok(package) = lua.globals().get::<Table>("package") {
        package.set("loadlib", Value::Nil)?;
        package.set("cpath", Value::Nil)?;
    }

    // Remove debug library
    lua.globals().set("debug", Value::Nil)?;

    // Set memory limit (10 MB)
    lua.set_memory_limit(10_000_000)?;

    // Register safe APIs
    register_http_api(&lua, &metadata.permissions)?;

    // Only register cache API if permissions allow
    if metadata.permissions.cache_read || metadata.permissions.cache_write {
        register_cache_api(&lua, &metadata.permissions)?;
    }

    // Register environment variable API if any env vars are configured
    if !env_vars.is_empty() {
        register_env_api(&lua, env_vars)?;
    }

    register_logging_api(&lua)?;

    // Add a safe print replacement that logs
    let log_info = lua.globals().get::<mlua::Function>("log_info")?;
    lua.globals().set("print", log_info)?;

    Ok(lua)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_blocks_dangerous_libs() {
        let metadata = create_test_metadata();
        let env_vars = HashMap::new();
        let lua = create_secure_lua_state(&metadata, &env_vars).unwrap();

        // Verify dangerous libraries are removed
        assert!(lua.globals().get::<Value>("os").unwrap().is_nil());
        assert!(lua.globals().get::<Value>("io").unwrap().is_nil());
        assert!(lua.globals().get::<Value>("load").unwrap().is_nil());
        assert!(lua.globals().get::<Value>("loadfile").unwrap().is_nil());
        assert!(lua.globals().get::<Value>("dofile").unwrap().is_nil());
        assert!(lua.globals().get::<Value>("debug").unwrap().is_nil());
    }

    #[test]
    fn test_sandbox_allows_safe_apis() {
        let metadata = create_test_metadata();
        let env_vars = HashMap::new();
        let lua = create_secure_lua_state(&metadata, &env_vars).unwrap();

        // Verify safe APIs are available
        assert!(lua.globals().get::<Value>("log_info").unwrap().is_function());
        assert!(lua.globals().get::<Value>("log_warn").unwrap().is_function());
        assert!(lua.globals().get::<Value>("log_error").unwrap().is_function());
        assert!(lua.globals().get::<Value>("http_get").unwrap().is_function());
    }

    fn create_test_metadata() -> PluginMetadata {
        use crate::plugins::registry::{ PluginInfo, PluginPermissions };

        PluginMetadata {
            plugin: PluginInfo {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                suffix: "-TEST".to_string(),
                author: None,
                description: None,
                enabled: true,
                timeout: 5,
            },
            permissions: PluginPermissions {
                network: true,
                allowed_domains: vec!["example.com".to_string()],
                cache_read: true,
                cache_write: true,
                user_agent: None,
                env_vars: Vec::new(),
            },
        }
    }
}
