//! Plugin registry for managing loaded plugins
//!
//! This module provides the central registry that stores all loaded plugins
//! and allows querying them by their registered suffixes.

use mlua::Lua;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin metadata parsed from meta.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin: PluginInfo,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub suffix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Execution timeout in seconds (default: 5)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout() -> u64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub cache_read: bool,
    #[serde(default)]
    pub cache_write: bool,
    /// Custom User-Agent for HTTP requests (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Environment variables to inject into the plugin (optional)
    #[serde(default)]
    pub env_vars: Vec<String>,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            network: false,
            allowed_domains: Vec::new(),
            cache_read: false,
            cache_write: false,
            user_agent: None,
            env_vars: Vec::new(),
        }
    }
}

/// A loaded plugin with its Lua state and metadata
pub struct LoadedPlugin {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Lua state for this plugin
    pub lua: Lua,
}

impl LoadedPlugin {
    /// Call the plugin's cleanup function if it exists
    pub fn call_cleanup(&self) {
        if let Ok(cleanup) = self.lua.globals().get::<mlua::Function>("cleanup") {
            if let Err(e) = cleanup.call::<()>(()) {
                eprintln!("Plugin {} cleanup error: {}", self.metadata.plugin.name, e);
            }
        }
    }

    /// Get the suffix this plugin handles
    pub fn suffix(&self) -> &str {
        &self.metadata.plugin.suffix
    }

    /// Get the plugin name
    pub fn name(&self) -> &str {
        &self.metadata.plugin.name
    }
}

/// Global plugin registry
///
/// This stores all loaded plugins indexed by their suffix.
pub struct PluginRegistry {
    /// Map from suffix (e.g., "-WEATHER") to the loaded plugin
    plugins: HashMap<String, Arc<LoadedPlugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin in the registry
    ///
    /// # Errors
    /// Returns an error if a plugin with the same suffix is already registered
    pub fn register(&mut self, plugin: LoadedPlugin) -> Result<(), anyhow::Error> {
        let suffix = plugin.suffix().to_uppercase();

        if self.plugins.contains_key(&suffix) {
            return Err(
                anyhow::anyhow!(
                    "Plugin suffix {} is already registered by {}",
                    suffix,
                    self.plugins[&suffix].name()
                )
            );
        }

        crate::log_info!("Registered plugin '{}' with suffix '{}'", plugin.name(), suffix);

        self.plugins.insert(suffix, Arc::new(plugin));
        Ok(())
    }

    /// Get a plugin by its suffix
    ///
    /// The suffix is case-insensitive and will be converted to uppercase.
    pub fn get_plugin(&self, suffix: &str) -> Option<Arc<LoadedPlugin>> {
        self.plugins.get(&suffix.to_uppercase()).cloned()
    }

    /// Get all registered suffixes
    pub fn get_all_suffixes(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get the number of registered plugins
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if the registry is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_suffix_case_insensitive() {
        let mut registry = PluginRegistry::new();
        let lua = Lua::new();

        let mut metadata = PluginMetadata {
            plugin: PluginInfo {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                suffix: "-TEST".to_string(),
                author: None,
                description: None,
                enabled: true,
                timeout: 5, // default timeout
            },
            permissions: PluginPermissions::default(),
        };

        // Test with lowercase suffix
        metadata.plugin.suffix = "-test".to_string();
        let plugin = LoadedPlugin {
            metadata: metadata.clone(),
            lua,
        };

        registry.register(plugin).unwrap();

        // Should be accessible with any case
        assert!(registry.get_plugin("-TEST").is_some());
        assert!(registry.get_plugin("-test").is_some());
        assert!(registry.get_plugin("-Test").is_some());
    }
}
