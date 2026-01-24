//! Plugin system for custom query suffix handlers
//!
//! This module provides a Lua-based plugin system that allows users to register
//! custom query suffixes (e.g., `-WEATHER`, `-CUSTOM`) by placing plugins in the
//! `plugins/` directory.
//!
//! # Plugin Structure
//!
//! Each plugin is a directory containing:
//! - `meta.toml` - Plugin metadata (name, version, suffix, permissions)
//! - `init.lua` - Lua plugin code
//!
//! # Plugin API
//!
//! Plugins must implement a `handle_query(query: string) -> string` function.
//! Optional functions: `init()` and `cleanup()`.
//!
//! # Security
//!
//! Plugins run in a secure sandbox with:
//! - No file I/O access
//! - No shell execution capabilities
//! - Network access restricted to whitelisted domains from meta.toml
//! - Resource limits (memory, execution time)

pub mod api;
pub mod env;
pub mod loader;
pub mod registry;
pub mod sandbox;

pub use loader::load_all_plugins;
pub use registry::{LoadedPlugin, PluginRegistry};
