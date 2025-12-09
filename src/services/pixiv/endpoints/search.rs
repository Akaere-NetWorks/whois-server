//! Search-related API endpoints

use super::super::client::PixivClient;
use super::super::error::PixivResult;
use super::super::models::Artwork;

impl PixivClient {
    /// Search artworks
    pub async fn search_illust(
        &mut self,
        word: &str,
        search_target: Option<&str>, // "partial_match_for_tags", "exact_match_for_tags", etc.
        sort: Option<&str>, // "date_desc", "date_asc", "popular_desc"
        filter: Option<&str>, // "for_ios", "safe"
        offset: Option<i32>,
        limit: Option<usize>,
    ) -> PixivResult<Vec<Artwork>> {
        let url = "https://app-api.pixiv.net/v1/search/illust";

        let mut params = std::collections::HashMap::new();
        params.insert("word".to_string(), word.to_string());

        if let Some(search_target) = search_target {
            params.insert("search_target".to_string(), search_target.to_string());
        }
        if let Some(sort) = sort {
            params.insert("sort".to_string(), sort.to_string());
        }
        if let Some(filter) = filter {
            params.insert("filter".to_string(), filter.to_string());
        }
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        let response: serde_json::Value = self
            .authenticated_request(reqwest::Method::GET, url, Some(&params))
            .await?;

        let mut artworks = Vec::new();

        // Process illusts
        if let Some(illusts) = response.get("illusts").and_then(|v| v.as_array()) {
            for item in illusts.iter().take(limit.unwrap_or(30)) {
                if let Ok(artwork) = serde_json::from_value::<Artwork>(item.clone()) {
                    artworks.push(artwork);
                }
            }
        }

        // Process manga
        if let Some(manga) = response.get("manga").and_then(|v| v.as_array()) {
            for item in manga.iter().take(limit.unwrap_or(30)) {
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

    /// Search novels (not fully implemented in our model)
    pub async fn search_novel(
        &mut self,
        word: &str,
        search_target: Option<&str>,
        sort: Option<&str>,
        filter: Option<&str>,
        offset: Option<i32>,
    ) -> PixivResult<serde_json::Value> {
        let url = "https://app-api.pixiv.net/v1/search/novel";

        let mut params = std::collections::HashMap::new();
        params.insert("word".to_string(), word.to_string());

        if let Some(search_target) = search_target {
            params.insert("search_target".to_string(), search_target.to_string());
        }
        if let Some(sort) = sort {
            params.insert("sort".to_string(), sort.to_string());
        }
        if let Some(filter) = filter {
            params.insert("filter".to_string(), filter.to_string());
        }
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        self.authenticated_request(
            reqwest::Method::GET,
            url,
            Some(&params),
        ).await
    }

    /// Search users
    pub async fn search_user(
        &mut self,
        word: &str,
        filter: Option<&str>, // "for_ios", "safe"
        offset: Option<i32>,
    ) -> PixivResult<serde_json::Value> {
        let url = "https://app-api.pixiv.net/v1/search/user";

        let mut params = std::collections::HashMap::new();
        params.insert("word".to_string(), word.to_string());

        if let Some(filter) = filter {
            params.insert("filter".to_string(), filter.to_string());
        }
        if let Some(offset) = offset {
            params.insert("offset".to_string(), offset.to_string());
        }

        self.authenticated_request(
            reqwest::Method::GET,
            url,
            Some(&params),
        ).await
    }
}