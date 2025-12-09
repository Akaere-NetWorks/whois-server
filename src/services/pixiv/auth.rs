//! Authentication handling for Pixiv API
//!
//! Implements OAuth 2.0 + PKCE flow for Pixiv authentication.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use super::error::{PixivError, PixivResult};

/// Pixiv API authentication constants
pub mod constants {
    pub const CLIENT_ID: &str = "MOBrBDS8blbauoSck0ZfDbtuzpyT";
    pub const CLIENT_SECRET: &str = "lsACyCD94FhDUtGTXi3QzcFE2uU1hqtDaKeqrdwj";
    pub const HASH_SECRET: &str = "28c1fdd170a5204386cb1313c7077b34f83e4aaf4aa829ce78c231e05b0bae2c";
    pub const AUTH_URL: &str = "https://oauth.secure.pixiv.net/auth/token";
    pub const USER_AGENT: &str = "PixivAndroidApp/5.0.64 (Android 6.0; Pixiv)";
}

/// Authentication token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// Access token for API calls
    pub access_token: String,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Time when the token expires
    pub expires_at: DateTime<Utc>,
    /// Scope of the token
    pub scope: Option<String>,
}

impl AuthToken {
    /// Create a new auth token from response
    pub fn from_response(
        access_token: String,
        refresh_token: String,
        expires_in: i64,
        scope: Option<String>,
    ) -> Self {
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_at,
            scope,
        }
    }

    /// Check if the token is expired or will expire within the buffer time
    pub fn is_expired(&self) -> bool {
        // Add 5-minute buffer before expiration
        let buffer = Duration::minutes(5);
        Utc::now() + buffer >= self.expires_at
    }

    /// Refresh the token using refresh token
    pub async fn refresh(&mut self, client: &reqwest::Client) -> PixivResult<()> {
        let form_data = [
            ("client_id", constants::CLIENT_ID),
            ("client_secret", constants::CLIENT_SECRET),
            ("grant_type", "refresh_token"),
            ("refresh_token", &self.refresh_token),
            ("include_policy", "true"),
        ];

        let response = client
            .post(constants::AUTH_URL)
            .header("User-Agent", constants::USER_AGENT)
            .form(&form_data)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: AuthResponse = response.json().await?;
            *self = token_response.into();
            Ok(())
        } else {
            Err(PixivError::Authentication(
                "Failed to refresh token".to_string(),
            ))
        }
    }
}

/// Authentication response from Pixiv OAuth endpoint
#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    token_type: String,
    scope: Option<String>,
    user: Option<serde_json::Value>,
}

impl From<AuthResponse> for AuthToken {
    fn from(response: AuthResponse) -> Self {
        Self::from_response(
            response.access_token,
            response.refresh_token,
            response.expires_in,
            response.scope,
        )
    }
}

/// PKCE (Proof Key for Code Exchange) implementation
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge
    pub fn generate() -> Self {
        use rand::Rng;
        use sha2::{Digest, Sha256};
        use base64::Engine;

        // Generate random code verifier (43-128 characters)
        let mut rng = rand::thread_rng();
        let code_verifier: String = (0..64)
            .map(|_| {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                CHARSET[rng.gen_range(0..CHARSET.len())] as char
            })
            .collect();

        // Calculate code challenge (SHA256 hash, base64url encoded)
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();

        let code_challenge = base64::engine::general_purpose::STANDARD
            .encode(hash)
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_string();

        Self {
            code_verifier,
            code_challenge,
        }
    }
}

/// Authentication manager
pub struct AuthManager {
    client: reqwest::Client,
    token: Option<AuthToken>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(constants::USER_AGENT)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            token: None,
        }
    }

    /// Authenticate using refresh token
    pub async fn authenticate_with_refresh_token(&mut self, refresh_token: &str) -> PixivResult<()> {
        let form_data = [
            ("client_id", constants::CLIENT_ID),
            ("client_secret", constants::CLIENT_SECRET),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("include_policy", "true"),
        ];

        let response = self
            .client
            .post(constants::AUTH_URL)
            .form(&form_data)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: AuthResponse = response.json().await?;
            self.token = Some(token_response.into());
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(PixivError::Authentication(format!(
                "Authentication failed: {}",
                error_text
            )))
        }
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_access_token(&mut self) -> PixivResult<String> {
        if let Some(token) = &mut self.token {
            if token.is_expired() {
                token.refresh(&self.client).await?;
            }
            Ok(token.access_token.clone())
        } else {
            Err(PixivError::Authentication(
                "No authentication token available".to_string(),
            ))
        }
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Get the HTTP client with authentication
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Calculate X-Client-Hash header
    pub fn calculate_client_hash(client_time: &str) -> String {
        use md5::compute;
        use base64::Engine;

        let hash = compute(format!("{}{}", client_time, constants::HASH_SECRET));
        base64::engine::general_purpose::STANDARD.encode(hash.0)
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}