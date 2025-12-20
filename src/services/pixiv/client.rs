//! Main Pixiv API client

use super::auth::AuthManager;
use super::error::{PixivError, PixivResult};
use super::models::*;
use chrono::Utc;
use reqwest::{Method, StatusCode};
use std::collections::HashMap;
use std::env;

use crate::{log_info};
/// Main Pixiv API client
pub struct PixivClient {
    auth: AuthManager,
    proxy_base_url: Option<String>,
}

impl PixivClient {
    /// Create a new Pixiv client
    pub fn new() -> PixivResult<Self> {
        let proxy_base_url = env::var("PIXIV_PROXY_BASE_URL").ok();
        let mut client = Self {
            auth: AuthManager::new(),
            proxy_base_url,
        };

        // Initialize with refresh token if available
        if let Ok(refresh_token) = env::var("PIXIV_REFRESH_TOKEN") {
            // This will be done asynchronously in a real initialization
            client.initialize_with_refresh_token(&refresh_token);
        }

        Ok(client)
    }

    /// Initialize with refresh token (non-blocking)
    fn initialize_with_refresh_token(&mut self, _refresh_token: &str) {
        // In a real implementation, you might want to handle this asynchronously
        // For now, we'll just note that refresh token is available
        log_info!("Pixiv client initialized with refresh token");
    }

    /// Ensure we're authenticated, refreshing if needed
    async fn ensure_authenticated(&mut self) -> PixivResult<String> {
        if !self.auth.is_authenticated() {
            // Try to get refresh token from environment
            let refresh_token = env::var("PIXIV_REFRESH_TOKEN")
                .map_err(|_| PixivError::EnvVar("PIXIV_REFRESH_TOKEN".to_string()))?;

            self.auth.authenticate_with_refresh_token(&refresh_token).await?;
        }

        self.auth.get_access_token().await
    }

    /// Make an authenticated request to the Pixiv API
    pub async fn authenticated_request<T: serde::de::DeserializeOwned>(
        &mut self,
        method: Method,
        url: &str,
        params: Option<&HashMap<String, String>>,
    ) -> PixivResult<T> {
        let access_token = self.ensure_authenticated().await?;

        // Prepare headers
        let client_time = Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00").to_string();
        let client_hash = AuthManager::calculate_client_hash(&client_time);

        let mut request = self
            .auth
            .client()
            .request(method.clone(), url)
            .bearer_auth(access_token)
            .header("X-Client-Time", client_time)
            .header("X-Client-Hash", client_hash)
            .header("App-OS", "android")
            .header("App-OS-Version", "9.0")
            .header("App-Version", "5.0.64");

        // Add query parameters for GET requests
        if let Some(params) = params {
            request = request.query(params);
        }

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            let result: T = response.json().await?;
            Ok(result)
        } else {
            match status {
                StatusCode::UNAUTHORIZED => {
                    Err(PixivError::TokenExpired)
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    Err(PixivError::RateLimit)
                }
                _ => {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(PixivError::api_error(
                        status.to_string(),
                        error_text,
                    ))
                }
            }
        }
    }

    /// Process image URLs to use proxy if configured
    pub fn process_image_url(&self, url: &str) -> String {
        if let Some(proxy_base) = &self.proxy_base_url {
            format!("{}?url={}", proxy_base, urlencoding::encode(url))
        } else {
            url.to_string()
        }
    }

    /// Post-process artwork data to apply proxy
    pub fn process_artwork(&self, artwork: &mut Artwork) {
        // Apply proxy to all image URLs
        artwork.image_urls.square_medium = self.process_image_url(&artwork.image_urls.square_medium);
        artwork.image_urls.medium = self.process_image_url(&artwork.image_urls.medium);
        artwork.image_urls.large = self.process_image_url(&artwork.image_urls.large);

        if let Some(_original) = &artwork.meta_single_page.original_image_url {
            // For single page artwork, we need to update the artwork structure
            // This is a limitation of the current model structure
            // In a production system, you might want to make this mutable
        }

        for page in &mut artwork.meta_pages {
            page.image_urls.square_medium = self.process_image_url(&page.image_urls.square_medium);
            page.image_urls.medium = self.process_image_url(&page.image_urls.medium);
            page.image_urls.large = self.process_image_url(&page.image_urls.large);
        }

        // Also process user profile image
        artwork.user.profile_image_urls.medium = self.process_image_url(&artwork.user.profile_image_urls.medium);
    }

    /// Get artwork details
    pub async fn get_artwork_info(&mut self, artwork_id: i64) -> PixivResult<Artwork> {
        let url = format!("https://app-api.pixiv.net/v1/illust/detail?illust_id={}", artwork_id);

        let mut params = HashMap::new();
        params.insert("filter".to_string(), "for_ios".to_string());

        let response: PixivResponse<serde_json::Value> = self
            .authenticated_request(Method::GET, &url, Some(&params))
            .await?;

        if let Some(body) = response.body {
            let mut artwork: Artwork = serde_json::from_value(body["illust"].clone())?;
            self.process_artwork(&mut artwork);
            Ok(artwork)
        } else {
            Err(PixivError::Api {
                code: "NO_BODY".to_string(),
                message: "Response body is missing".to_string(),
            })
        }
    }

    /// Get user profile information
    pub async fn get_user_info(&mut self, user_id: i64) -> PixivResult<UserProfile> {
        let url = format!("https://app-api.pixiv.net/v1/user/detail?user_id={}", user_id);

        let mut params = HashMap::new();
        params.insert("filter".to_string(), "for_ios".to_string());
        params.insert("illust_id".to_string(), "0".to_string());

        let response: PixivResponse<serde_json::Value> = self
            .authenticated_request(Method::GET, &url, Some(&params))
            .await?;

        if let Some(body) = response.body {
            let profile: UserProfile = serde_json::from_value(body)?;
            Ok(profile)
        } else {
            Err(PixivError::Api {
                code: "NO_BODY".to_string(),
                message: "Response body is missing".to_string(),
            })
        }
    }

    /// Search artworks
    pub async fn search_artworks(
        &mut self,
        keyword: &str,
        limit: usize,
    ) -> PixivResult<Vec<Artwork>> {
        let url = "https://app-api.pixiv.net/v1/search/illust";

        let mut params = HashMap::new();
        params.insert("word".to_string(), keyword.to_string());
        params.insert("search_target".to_string(), "partial_match_for_tags".to_string());
        params.insert("sort".to_string(), "date_desc".to_string());
        params.insert("filter".to_string(), "for_ios".to_string());

        let response: SearchResults = self
            .authenticated_request(Method::GET, url, Some(&params))
            .await?;

        // Combine artworks and manga, apply limit
        let mut all_artworks = response.artworks;
        all_artworks.extend(response.manga);

        // Apply proxy to images
        for artwork in &mut all_artworks {
            self.process_artwork(artwork);
        }

        // Limit results
        all_artworks.truncate(limit);
        Ok(all_artworks)
    }

    /// Get ranking information
    pub async fn get_ranking(
        &mut self,
        mode: &str,
        limit: usize,
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!("https://app-api.pixiv.net/v1/ranking/{}", mode);

        let mut params = HashMap::new();
        params.insert("filter".to_string(), "for_ios".to_string());

        let response: RankingResults = self
            .authenticated_request(Method::GET, &url, Some(&params))
            .await?;

        // Convert ranking items to artworks (need to fetch full details)
        // For now, return basic info - in production, you'd fetch each artwork
        let mut artworks = Vec::new();
        for item in response.contents.iter().take(limit) {
            // This is a simplified conversion - real implementation would fetch full artwork details
            let artwork = self.get_artwork_info(item.illust_id).await;
            if let Ok(artwork) = artwork {
                artworks.push(artwork);
            }
        }

        Ok(artworks)
    }

    /// Get user's artworks
    pub async fn get_user_illusts(
        &mut self,
        user_id: i64,
        limit: usize,
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!("https://app-api.pixiv.net/v1/user/illusts?user_id={}", user_id);

        let mut params = HashMap::new();
        params.insert("filter".to_string(), "for_ios".to_string());

        let response: UserArtworks = self
            .authenticated_request(Method::GET, &url, Some(&params))
            .await?;

        // Combine artworks and manga, apply limit
        let mut all_artworks = response.artworks;
        all_artworks.extend(response.manga);

        // Apply proxy to images
        for artwork in &mut all_artworks {
            self.process_artwork(artwork);
        }

        // Limit results
        all_artworks.truncate(limit);
        Ok(all_artworks)
    }
}

impl Default for PixivClient {
    fn default() -> Self {
        Self::new().expect("Failed to create Pixiv client")
    }
}