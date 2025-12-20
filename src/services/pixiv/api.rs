/*
 * WHOIS Server with DN42 Support - Pure Rust Pixiv Service
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::Result;
// Use the pure Rust implementation from the current module
use super::pixiv_impl::*;

use crate::{log_info};
/// Query Pixiv artwork information by ID (returns formatted text)
pub async fn query_pixiv_artwork(artwork_id: &str) -> Result<String> {
    log_info!("Querying Pixiv artwork: {}", artwork_id);
    query_pixiv_artwork_rust(artwork_id).await
}

/// Query Pixiv artwork information by ID (returns JSON)
pub async fn query_pixiv_artwork_json(artwork_id: &str) -> Result<String> {
    log_info!("Querying Pixiv artwork (JSON): {}", artwork_id);
    query_pixiv_artwork_json_rust(artwork_id).await
}

/// Query Pixiv user information by ID (returns formatted text)
pub async fn query_pixiv_user(user_id: &str) -> Result<String> {
    log_info!("Querying Pixiv user: {}", user_id);
    query_pixiv_user_rust(user_id).await
}

/// Query Pixiv user information by ID (returns JSON)
pub async fn query_pixiv_user_json(user_id: &str) -> Result<String> {
    log_info!("Querying Pixiv user (JSON): {}", user_id);
    query_pixiv_user_json_rust(user_id).await
}

/// Search Pixiv artworks by keyword (returns formatted text)
pub async fn search_pixiv_artworks(keyword: &str, limit: Option<i32>) -> Result<String> {
    log_info!("Searching Pixiv artworks: {}", keyword);
    search_pixiv_artworks_rust(keyword, limit).await
}

/// Search Pixiv artworks by keyword (returns JSON)
pub async fn search_pixiv_artworks_json(keyword: &str, limit: Option<i32>) -> Result<String> {
    log_info!("Searching Pixiv artworks (JSON): {}", keyword);
    search_pixiv_artworks_json_rust(keyword, limit).await
}

/// Get Pixiv ranking (returns formatted text)
pub async fn query_pixiv_ranking(mode: Option<&str>, limit: Option<i32>) -> Result<String> {
    log_info!("Querying Pixiv ranking: mode={:?}, limit={:?}", mode, limit);
    query_pixiv_ranking_rust(mode, limit).await
}

/// Get Pixiv ranking (returns JSON)
pub async fn query_pixiv_ranking_json(mode: Option<&str>, limit: Option<i32>) -> Result<String> {
    log_info!("Querying Pixiv ranking (JSON): mode={:?}, limit={:?}", mode, limit);
    query_pixiv_ranking_json_rust(mode, limit).await
}

/// Get user's artworks (returns formatted text)
pub async fn query_pixiv_user_illusts(user_id: &str, limit: Option<i32>) -> Result<String> {
    log_info!("Querying Pixiv user artworks: {}", user_id);
    query_pixiv_user_illusts_rust(user_id, limit).await
}

/// Get user's artworks (returns JSON)
pub async fn query_pixiv_user_illusts_json(user_id: &str, limit: Option<i32>) -> Result<String> {
    log_info!("Querying Pixiv user artworks (JSON): {}", user_id);
    query_pixiv_user_illusts_json_rust(user_id, limit).await
}

/// Main entry point for Pixiv queries
pub async fn process_pixiv_query(query: &str) -> Result<String> {
    log_info!("Processing Pixiv query: {}", query);
    process_pixiv_query_rust(query).await
}

/// Main entry point for Pixiv queries (JSON output)
pub async fn process_pixiv_query_json(query: &str) -> Result<String> {
    log_info!("Processing Pixiv query (JSON): {}", query);
    process_pixiv_query_json_rust(query).await
}