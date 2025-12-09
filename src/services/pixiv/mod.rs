//! Pure Rust implementation of Pixiv API client

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod api;
pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod endpoints;
pub mod pixiv_impl;

// Re-export main components
pub use auth::{AuthManager, AuthToken};
pub use client::PixivClient;
pub use error::{PixivError, PixivResult};
pub use models::*;

// Re-export the implementation functions and API
pub use pixiv_impl::*;
pub use api::*;