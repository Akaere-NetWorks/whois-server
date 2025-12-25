//! Plugin loading logic
//!
//! This module handles discovering and loading plugins from the plugins directory.

use crate::plugins::registry::{LoadedPlugin, PluginMetadata, PluginRegistry};
use crate::plugins::sandbox::create_secure_lua_state;
use anyhow::Context;
use std::fs;
use std::path::Path;

/// Load all plugins from the plugins directory
///
/// This function scans the `plugins/` directory and loads all enabled plugins.
pub async fn load_all_plugins() -> anyhow::Result<PluginRegistry> {
    let plugins_dir = Path::new("plugins");

    // Create plugins directory if it doesn't exist
    if !plugins_dir.exists() {
        crate::log_info!("Creating plugins directory");
        fs::create_dir_all(plugins_dir)?;
        return Ok(PluginRegistry::new());
    }

    let mut registry = PluginRegistry::new();
    let mut loaded_count = 0;
    let mut skipped_count = 0;

    // Read all entries in the plugins directory
    let entries = fs::read_dir(plugins_dir)
        .context("Failed to read plugins directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Skip non-directories
        if !path.is_dir() {
            continue;
        }

        // Try to load plugin from this directory
        match load_plugin_from_dir(&path).await {
            Ok(plugin) => {
                if let Err(e) = registry.register(plugin) {
                    crate::log_warn!("Failed to register plugin from {:?}: {}", path, e);
                    skipped_count += 1;
                } else {
                    loaded_count += 1;
                }
            }
            Err(e) => {
                crate::log_warn!("Failed to load plugin from {:?}: {}", path, e);
                skipped_count += 1;
            }
        }
    }

    crate::log_info!(
        "Loaded {} plugin(s), skipped {}",
        loaded_count, skipped_count
    );

    Ok(registry)
}

/// Load a plugin from a directory
///
/// The directory must contain:
/// - `meta.toml` - Plugin metadata
/// - `init.lua` - Plugin code
async fn load_plugin_from_dir(dir: &Path) -> anyhow::Result<LoadedPlugin> {
    let meta_path = dir.join("meta.toml");
    let init_path = dir.join("init.lua");

    // Check that required files exist
    if !meta_path.exists() {
        return Err(anyhow::anyhow!("meta.toml not found"));
    }

    if !init_path.exists() {
        return Err(anyhow::anyhow!("init.lua not found"));
    }

    // Read metadata
    let metadata = read_metadata(&meta_path)?;

    // Check if plugin is enabled
    if !metadata.plugin.enabled {
        return Err(anyhow::anyhow!("Plugin is disabled in meta.toml"));
    }

    // Validate suffix format
    let suffix = &metadata.plugin.suffix;
    if !suffix.starts_with('-') {
        return Err(anyhow::anyhow!(
            "Plugin suffix must start with '-', got: {}",
            suffix
        ));
    }

    // Create secure Lua state
    let lua = create_secure_lua_state(&metadata)
        .map_err(|e| anyhow::anyhow!("Failed to create Lua state: {}", e))?;

    // Load plugin code
    let code = fs::read_to_string(&init_path)
        .context("Failed to read init.lua")?;

    // Execute the plugin code
    lua.load(&code)
        .exec()
        .map_err(|e| anyhow::anyhow!("Failed to execute plugin code: {}", e))?;

    // Verify required function exists
    let has_handle_query: Result<mlua::Function, _> = lua.globals().get("handle_query");
    if has_handle_query.is_err() {
        return Err(anyhow::anyhow!(
            "Plugin must define a handle_query(query: string) -> string function"
        ));
    }

    // Call init function if it exists
    if let Ok(init) = lua.globals().get::<mlua::Function>("init") {
        if let Err(e) = init.call::<()>(()) {
            crate::log_warn!(
                "Plugin {} init function failed: {}",
                metadata.plugin.name, e
            );
        }
    }

    crate::log_info!(
        "Loaded plugin '{}' v{} (suffix: {})",
        metadata.plugin.name, metadata.plugin.version, suffix
    );

    Ok(LoadedPlugin { metadata, lua })
}

/// Read plugin metadata from meta.toml
fn read_metadata(path: &Path) -> anyhow::Result<PluginMetadata> {
    let content = fs::read_to_string(path)
        .context("Failed to read meta.toml")?;

    let metadata: PluginMetadata = toml::from_str(&content)
        .context("Failed to parse meta.toml")?;

    // Validate required fields
    if metadata.plugin.name.is_empty() {
        return Err(anyhow::anyhow!("Plugin name cannot be empty"));
    }

    if metadata.plugin.suffix.is_empty() {
        return Err(anyhow::anyhow!("Plugin suffix cannot be empty"));
    }

    if metadata.plugin.version.is_empty() {
        return Err(anyhow::anyhow!("Plugin version cannot be empty"));
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_read_metadata_valid() {
        let temp_dir = TempDir::new().unwrap();
        let meta_path = temp_dir.path().join("meta.toml");

        let mut file = File::create(&meta_path).unwrap();
        writeln!(
            file,
            r#"[plugin]
name = "test-plugin"
version = "1.0.0"
suffix = "-TEST"
author = "Test Author"
description = "Test plugin"
enabled = true

[permissions]
network = false
cache_read = false
cache_write = false"#
        ).unwrap();

        let metadata = read_metadata(&meta_path).unwrap();
        assert_eq!(metadata.plugin.name, "test-plugin");
        assert_eq!(metadata.plugin.version, "1.0.0");
        assert_eq!(metadata.plugin.suffix, "-TEST");
        assert!(metadata.plugin.enabled);
    }

    #[test]
    fn test_read_metadata_missing_fields() {
        let temp_dir = TempDir::new().unwrap();
        let meta_path = temp_dir.path().join("meta.toml");

        let mut file = File::create(&meta_path).unwrap();
        writeln!(file, r#"[plugin]
name = "test"
suffix = "-TEST"
"#).unwrap();

        let result = read_metadata(&meta_path);
        assert!(result.is_err() || result.unwrap().plugin.version.is_empty());
    }
}
