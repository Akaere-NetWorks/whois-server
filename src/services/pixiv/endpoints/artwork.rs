//! Artwork-related API endpoints

use super::super::client::PixivClient;
use super::super::error::PixivResult;
use super::super::models::Artwork;

impl PixivClient {
    /// Get artwork details by ID
    pub async fn illust_detail(&mut self, illust_id: i64) -> PixivResult<Artwork> {
        self.get_artwork_info(illust_id).await
    }

    /// Get artwork comments
    pub async fn illust_comments(
        &mut self,
        illust_id: i64,
        offset: Option<i32>,
    ) -> PixivResult<serde_json::Value> {
        let url = format!("https://app-api.pixiv.net/v2/illust/comments?illust_id={}", illust_id);

        let mut params = std::collections::HashMap::new();
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        self.authenticated_request(
            reqwest::Method::GET,
            &url,
            Some(&params),
        ).await
    }

    /// Get related artworks
    pub async fn illust_related(
        &mut self,
        illust_id: i64,
        limit: Option<usize>,
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!("https://app-api.pixiv.net/v2/illust/related?illust_id={}", illust_id);

        let response: serde_json::Value = self
            .authenticated_request(reqwest::Method::GET, &url, None)
            .await?;

        let mut artworks = Vec::new();
        if let Some(illusts) = response.get("illusts").and_then(|v| v.as_array()) {
            for item in illusts.iter().take(limit.unwrap_or(30)) {
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
}