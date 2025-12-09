//! Ranking-related API endpoints

use super::super::client::PixivClient;
use super::super::error::PixivResult;
use super::super::models::Artwork;

impl PixivClient {
    /// Get ranking information
    pub async fn illust_ranking(
        &mut self,
        mode: &str, // "daily", "weekly", "monthly", "daily_r18", etc.
        filter: Option<&str>, // "for_ios", "safe"
        offset: Option<i32>,
        limit: Option<usize>,
    ) -> PixivResult<Vec<Artwork>> {
        let url = format!("https://app-api.pixiv.net/v1/ranking/{}", mode);

        let mut params = std::collections::HashMap::new();
        if let Some(filter) = filter {
            params.insert("filter".to_string(), filter.to_string());
        }
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        let response: serde_json::Value = self
            .authenticated_request(reqwest::Method::GET, &url, Some(&params))
            .await?;

        let mut artworks = Vec::new();

        if let Some(contents) = response.get("contents").and_then(|v| v.as_array()) {
            for item in contents.iter().take(limit.unwrap_or(30)) {
                if let Some(illust_id) = item.get("illust_id").and_then(|v| v.as_i64()) {
                    // For ranking, we need to fetch the full artwork details
                    match self.get_artwork_info(illust_id).await {
                        Ok(artwork) => artworks.push(artwork),
                        Err(_) => continue, // Skip if we can't fetch details
                    }
                }
            }
        }

        Ok(artworks)
    }

    /// Get previous ranking
    pub async fn illust_ranking_prev(
        &mut self,
        mode: &str,
        filter: Option<&str>,
        offset: Option<i32>,
    ) -> PixivResult<serde_json::Value> {
        let url = format!("https://app-api.pixiv.net/v1/ranking/{}/prev", mode);

        let mut params = std::collections::HashMap::new();
        if let Some(filter) = filter {
            params.insert("filter".to_string(), filter.to_string());
        }
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        self.authenticated_request(
            reqwest::Method::GET,
            &url,
            Some(&params),
        ).await
    }
}