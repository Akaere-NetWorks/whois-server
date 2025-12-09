//! User-related API endpoints

use super::super::client::PixivClient;
use super::super::error::PixivResult;
use super::super::models::{UserProfile, Artwork};

impl PixivClient {
    /// Get user profile details
    pub async fn user_detail(&mut self, user_id: i64) -> PixivResult<UserProfile> {
        self.get_user_info(user_id).await
    }

    /// Get user's artworks
    pub async fn user_illusts(
        &mut self,
        user_id: i64,
        limit: Option<usize>,
        offset: Option<i32>,
        artwork_type: Option<&str>, // "illust", "manga", or None for both
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!(
            "https://app-api.pixiv.net/v1/user/illusts?user_id={}&filter=for_ios",
            user_id
        );

        let mut params = std::collections::HashMap::new();
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }
        if let Some(artwork_type) = artwork_type {
            params.insert("type".to_string(), artwork_type.to_string());
        }

        let response: serde_json::Value = self
            .authenticated_request(reqwest::Method::GET, &url, Some(&params))
            .await?;

        let mut artworks = Vec::new();

        // Process illusts
        if let Some(illusts) = response.get("illusts").and_then(|v| v.as_array()) {
            for item in illusts {
                if let Ok(artwork) = serde_json::from_value::<Artwork>(item.clone()) {
                    artworks.push(artwork);
                }
            }
        }

        // Process manga
        if let Some(manga) = response.get("manga").and_then(|v| v.as_array()) {
            for item in manga {
                if let Ok(artwork) = serde_json::from_value::<Artwork>(item.clone()) {
                    artworks.push(artwork);
                }
            }
        }

        // Apply proxy to images
        for artwork in &mut artworks {
            self.process_artwork(artwork);
        }

        // Apply limit
        if let Some(limit) = limit {
            artworks.truncate(limit);
        }

        Ok(artworks)
    }

    /// Get user's bookmarks
    pub async fn user_bookmarks_illust(
        &mut self,
        user_id: i64,
        _limit: Option<usize>,
        offset: Option<i32>,
        restrict: Option<&str>, // "public" or "private"
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!(
            "https://app-api.pixiv.net/v1/user/bookmarks/illust?user_id={}&filter=for_ios",
            user_id
        );

        let mut params = std::collections::HashMap::new();
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }
        if let Some(restrict) = restrict {
            params.insert("restrict".to_string(), restrict.to_string());
        }

        let response: serde_json::Value = self
            .authenticated_request(reqwest::Method::GET, &url, Some(&params))
            .await?;

        let mut artworks = Vec::new();
        if let Some(bookmarks) = response.get("illusts").and_then(|v| v.as_array()) {
            for item in bookmarks.iter().take(_limit.unwrap_or(30)) {
                if let Ok(artwork) = serde_json::from_value::<Artwork>(item.clone()) {
                    artworks.push(artwork);
                }
            }
        }

        // Apply proxy to images
        for artwork in &mut artworks {
            self.process_artwork(artwork);
        }

        Ok(artworks)
    }

    /// Get users that the user is following
    pub async fn user_following(
        &mut self,
        user_id: i64,
        _limit: Option<usize>,
        offset: Option<i32>,
        restrict: Option<&str>, // "public" or "private"
    ) -> PixivResult<serde_json::Value> {
        let url = format!(
            "https://app-api.pixiv.net/v1/user/following?user_id={}",
            user_id
        );

        let mut params = std::collections::HashMap::new();
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }
        if let Some(restrict) = restrict {
            params.insert("restrict".to_string(), restrict.to_string());
        }

        self.authenticated_request(
            reqwest::Method::GET,
            &url,
            Some(&params),
        ).await
    }
}