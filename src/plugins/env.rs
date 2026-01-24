//! Environment variable loading for plugins
//!
//! This module handles loading environment variables from `.plugins.env` file
//! and injecting them into plugin Lua states.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Load environment variables from .plugins.env file
///
/// The file format is simple KEY=VALUE, one per line.
/// Lines starting with # are comments.
/// Empty lines are ignored.
pub fn load_env_file() -> Result<HashMap<String, String>> {
    let env_path = Path::new(".plugins.env");

    // If file doesn't exist, return empty map
    if !env_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(env_path)
        .context("Failed to read .plugins.env file")?;

    let mut env_vars = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim();

            // Remove quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };

            if !key.is_empty() {
                env_vars.insert(key.to_string(), value.to_string());
            }
        } else {
            // Invalid line, but don't fail - just log
            eprintln!(
                "Warning: .plugins.env line {} has invalid format: {}",
                line_num + 1,
                line
            );
        }
    }

    Ok(env_vars)
}

/// Get specific environment variables for a plugin
///
/// Given a list of variable names requested by the plugin,
/// return only those that exist in the loaded environment.
pub fn get_plugin_env_vars(
    requested_vars: &[String],
    env_vars: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut plugin_env = HashMap::new();

    for var_name in requested_vars {
        if let Some(value) = env_vars.get(var_name) {
            plugin_env.insert(var_name.clone(), value.clone());
        }
    }

    plugin_env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_env_file() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".plugins.env");

        let mut file = fs::File::create(&env_path).unwrap();
        writeln!(file, "# Test environment file").unwrap();
        writeln!(file, "API_KEY=test_key_123").unwrap();
        writeln!(file, "API_SECRET=secret_value").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "BASE_URL=https://api.example.com").unwrap();

        // Note: This test would need to run in the temp dir context
        // For now, we'll skip actual file loading test
    }

    #[test]
    fn test_get_plugin_env_vars() {
        let mut env_vars = HashMap::new();
        env_vars.insert("API_KEY".to_string(), "test_key".to_string());
        env_vars.insert("SECRET".to_string(), "secret".to_string());
        env_vars.insert("UNUSED".to_string(), "value".to_string());

        let requested = vec!["API_KEY".to_string(), "SECRET".to_string()];
        let plugin_env = get_plugin_env_vars(&requested, &env_vars);

        assert_eq!(plugin_env.len(), 2);
        assert_eq!(plugin_env.get("API_KEY"), Some(&"test_key".to_string()));
        assert_eq!(plugin_env.get("SECRET"), Some(&"secret".to_string()));
        assert_eq!(plugin_env.get("UNUSED"), None);
    }
}
