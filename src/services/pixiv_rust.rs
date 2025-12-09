//! Pure Rust implementation of Pixiv API client
//!
//! This module provides a complete Rust implementation of the Pixiv API,
//! replacing the Python-based pixivpy3 dependency.

pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod endpoints;

// Re-export main components
pub use client::PixivClient;
pub use auth::AuthToken;
pub use error::{PixivError, PixivResult};
pub use models::*;