//! Error types for the Pixiv client

#![allow(dead_code)]

use thiserror::Error;

/// Pixiv client error type
#[derive(Error, Debug)]
pub enum PixivError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("API error: {message} (code: {code})")]
    Api { code: String, message: String },

    #[error("Token expired")]
    TokenExpired,

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Environment variable not found: {0}")]
    EnvVar(String),

    #[error("Invalid artwork ID: {0}")]
    InvalidArtworkId(String),

    #[error("Invalid user ID: {0}")]
    InvalidUserId(String),

    #[error("Invalid search query: {0}")]
    InvalidSearchQuery(String),
}

/// Result type for Pixiv operations
pub type PixivResult<T> = Result<T, PixivError>;

impl PixivError {
    /// Create an API error from response
    pub fn api_error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Api {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Check if error is recoverable (should retry)
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Network(_) | Self::RateLimit => true,
            Self::Api { message, .. } => message.contains("too many requests") || message.contains("Rate Limit"),
            _ => false,
        }
    }
}