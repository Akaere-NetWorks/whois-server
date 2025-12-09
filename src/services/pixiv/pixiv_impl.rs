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
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{debug, error};

// Import our new Rust pixiv client from current module
use super::PixivClient;

lazy_static::lazy_static! {
    static ref PIXIV_CLIENT: Mutex<PixivClient> = Mutex::new(PixivClient::default());
}

/// Initialize the Pixiv client if not already done
async fn get_client() -> Result<PixivClient> {
    // Create a new client instance each time
    // In a production system, you might want to cache this
    Ok(PixivClient::new()?)
}

/// Query Pixiv artwork information by ID (returns formatted text)
pub async fn query_pixiv_artwork_rust(artwork_id: &str) -> Result<String> {
    query_pixiv_artwork_internal_rust(artwork_id, false).await
}

/// Query Pixiv artwork information by ID (returns JSON)
pub async fn query_pixiv_artwork_json_rust(artwork_id: &str) -> Result<String> {
    query_pixiv_artwork_internal_rust(artwork_id, true).await
}

/// Internal function to query Pixiv artwork using Rust implementation
async fn query_pixiv_artwork_internal_rust(artwork_id: &str, json_output: bool) -> Result<String> {
    debug!("Querying Pixiv artwork (Rust): {}", artwork_id);

    // Parse artwork ID
    let id: i64 = artwork_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid artwork ID: {}", artwork_id))?;

    // Use Rust client
    let mut client = get_client().await?;

    match client.get_artwork_info(id).await {
        Ok(artwork) => {
            let json = serde_json::to_value(&artwork)?;
            if json_output {
                Ok(serde_json::to_string_pretty(&json)?)
            } else {
                format_artwork_info_rust(&json)
            }
        }
        Err(e) => {
            error!("Pixiv API error: {:?}", e);
            Ok(format!("% Error querying Pixiv artwork: {}\n", e))
        }
    }
}

/// Query Pixiv user information by ID
pub async fn query_pixiv_user_rust(user_id: &str) -> Result<String> {
    query_pixiv_user_internal_rust(user_id, false).await
}

/// Query Pixiv user information by ID (returns JSON)
pub async fn query_pixiv_user_json_rust(user_id: &str) -> Result<String> {
    query_pixiv_user_internal_rust(user_id, true).await
}

/// Internal function to query Pixiv user using Rust implementation
async fn query_pixiv_user_internal_rust(user_id: &str, json_output: bool) -> Result<String> {
    debug!("Querying Pixiv user (Rust): {}", user_id);

    // Parse user ID
    let id: i64 = user_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid user ID: {}", user_id))?;

    // Use Rust client
    let mut client = get_client().await?;

    match client.get_user_info(id).await {
        Ok(profile) => {
            let json = serde_json::to_value(&profile)?;
            if json_output {
                Ok(serde_json::to_string_pretty(&json)?)
            } else {
                format_user_info_rust(&json)
            }
        }
        Err(e) => {
            error!("Pixiv API error: {:?}", e);
            Ok(format!("% Error querying Pixiv user: {}\n", e))
        }
    }
}

/// Search Pixiv artworks by keyword (returns formatted text)
pub async fn search_pixiv_artworks_rust(keyword: &str, limit: Option<i32>) -> Result<String> {
    search_pixiv_artworks_internal_rust(keyword, limit, false).await
}

/// Search Pixiv artworks by keyword (returns JSON)
pub async fn search_pixiv_artworks_json_rust(keyword: &str, limit: Option<i32>) -> Result<String> {
    search_pixiv_artworks_internal_rust(keyword, limit, true).await
}

/// Internal function to search Pixiv artworks using Rust implementation
async fn search_pixiv_artworks_internal_rust(keyword: &str, limit: Option<i32>, json_output: bool) -> Result<String> {
    debug!("Searching Pixiv artworks (Rust): {}", keyword);

    let limit = limit.unwrap_or(10);

    // Use Rust client
    let mut client = get_client().await?;

    match client.search_artworks(keyword, limit as usize).await {
        Ok(artworks) => {
            let json = serde_json::json!({
                "keyword": keyword,
                "total": artworks.len(),
                "results": artworks
            });

            if json_output {
                Ok(serde_json::to_string_pretty(&json)?)
            } else {
                format_search_results_rust(&json)
            }
        }
        Err(e) => {
            error!("Pixiv API error: {:?}", e);
            Ok(format!("% Error searching Pixiv: {}\n", e))
        }
    }
}

/// Get Pixiv ranking (returns formatted text)
pub async fn query_pixiv_ranking_rust(mode: Option<&str>, limit: Option<i32>) -> Result<String> {
    query_pixiv_ranking_internal_rust(mode, limit, false).await
}

/// Get Pixiv ranking (returns JSON)
pub async fn query_pixiv_ranking_json_rust(mode: Option<&str>, limit: Option<i32>) -> Result<String> {
    query_pixiv_ranking_internal_rust(mode, limit, true).await
}

/// Internal function to get Pixiv ranking using Rust implementation
async fn query_pixiv_ranking_internal_rust(mode: Option<&str>, limit: Option<i32>, json_output: bool) -> Result<String> {
    let mode = mode.unwrap_or("day");
    let limit = limit.unwrap_or(10);

    debug!("Querying Pixiv ranking (Rust): mode={}, limit={}", mode, limit);

    // Use Rust client
    let mut client = get_client().await?;

    match client.get_ranking(mode, limit as usize).await {
        Ok(artworks) => {
            let json = serde_json::json!({
                "mode": mode,
                "total": artworks.len(),
                "results": artworks
            });

            if json_output {
                Ok(serde_json::to_string_pretty(&json)?)
            } else {
                format_ranking_results_rust(&json, mode)
            }
        }
        Err(e) => {
            error!("Pixiv API error: {:?}", e);
            Ok(format!("% Error querying Pixiv ranking: {}\n", e))
        }
    }
}

/// Get user's artworks (returns formatted text)
pub async fn query_pixiv_user_illusts_rust(user_id: &str, limit: Option<i32>) -> Result<String> {
    query_pixiv_user_illusts_internal_rust(user_id, limit, false).await
}

/// Get user's artworks (returns JSON)
pub async fn query_pixiv_user_illusts_json_rust(user_id: &str, limit: Option<i32>) -> Result<String> {
    query_pixiv_user_illusts_internal_rust(user_id, limit, true).await
}

/// Internal function to get user's artworks using Rust implementation
async fn query_pixiv_user_illusts_internal_rust(user_id: &str, limit: Option<i32>, json_output: bool) -> Result<String> {
    debug!("Querying Pixiv user illusts (Rust): {}", user_id);

    let id: i64 = user_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid user ID: {}", user_id))?;

    let limit = limit.unwrap_or(10);

    // Use Rust client
    let mut client = get_client().await?;

    match client.get_user_illusts(id, limit as usize).await {
        Ok(artworks) => {
            let json = serde_json::json!({
                "user_id": user_id,
                "total": artworks.len(),
                "results": artworks
            });

            if json_output {
                Ok(serde_json::to_string_pretty(&json)?)
            } else {
                format_user_illusts_results_rust(&json)
            }
        }
        Err(e) => {
            error!("Pixiv API error: {:?}", e);
            Ok(format!("% Error querying user illusts: {}\n", e))
        }
    }
}

/// Format artwork information for display (adapted from original)
fn format_artwork_info_rust(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV ARTWORK INFORMATION (Rust)\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(id) = data.get("id") {
        output.push_str(&format!("Artwork ID:      {}\n", id));
    }

    if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
        output.push_str(&format!("Title:           {}\n", title));
    }

    if let Some(art_type) = data.get("artwork_type").and_then(|v| v.as_str()) {
        output.push_str(&format!("Type:            {}\n", art_type));
    }

    if let Some(user) = data.get("user") {
        if let Some(name) = user.get("name").and_then(|v| v.as_str()) {
            output.push_str(&format!("Artist:          {}\n", name));
        }
        if let Some(user_id) = user.get("id") {
            output.push_str(&format!("Artist ID:       {}\n", user_id));
        }
    }

    if let Some(created_at) = data.get("created_at").and_then(|v| v.as_str()) {
        output.push_str(&format!("Created:         {}\n", created_at));
    }

    if let Some(width) = data.get("width") {
        if let Some(height) = data.get("height") {
            output.push_str(&format!("Dimensions:      {}x{}\n", width, height));
        }
    }

    if let Some(page_count) = data.get("page_count") {
        output.push_str(&format!("Pages:           {}\n", page_count));
    }

    if let Some(views) = data.get("total_view") {
        output.push_str(&format!("Total Views:     {}\n", views));
    }

    if let Some(bookmarks) = data.get("total_bookmarks") {
        output.push_str(&format!("Bookmarks:       {}\n", bookmarks));
    }

    if let Some(tags) = data.get("tags").and_then(|v| v.as_array()) {
        output.push_str("Tags:            ");
        let tag_names: Vec<String> = tags
            .iter()
            .filter_map(|t| {
                if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        output.push_str(&tag_names.join(", "));
        output.push('\n');
    }

    // 显示图片链接
    if let Some(image_urls) = data.get("image_urls") {
        output.push_str("\nImage URLs:\n");

        if let Some(large) = image_urls.get("large").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Large:         {}\n", large));
        }

        if let Some(medium) = image_urls.get("medium").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Medium:        {}\n", medium));
        }

        if let Some(square) = image_urls.get("square_medium").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Square:        {}\n", square));
        }
    }

    // 显示多页作品的所有页面
    if let Some(meta_pages) = data.get("meta_pages").and_then(|v| v.as_array()) {
        if !meta_pages.is_empty() {
            output.push_str(&format!(
                "\nMulti-page Artwork ({} pages):\n",
                meta_pages.len()
            ));
            for (i, page) in meta_pages.iter().enumerate() {
                output.push_str(&format!("  Page {}:\n", i + 1));

                if let Some(large) = page.get("image_urls").and_then(|u| u.get("large")).and_then(|v| v.as_str()) {
                    output.push_str(&format!("    Large:     {}\n", large));
                }

                if let Some(medium) = page.get("image_urls").and_then(|u| u.get("medium")).and_then(|v| v.as_str()) {
                    output.push_str(&format!("    Medium:    {}\n", medium));
                }
            }
        }
    }

    if let Some(description) = data.get("description").and_then(|v| v.as_str()) {
        if !description.is_empty() {
            output.push_str(&format!("\nCaption: {}\n", description));
        }
    }

    Ok(output)
}

/// Format user information for display (adapted from original)
fn format_user_info_rust(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV USER INFORMATION (Rust)\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(id) = data.get("user").and_then(|u| u.get("id")) {
        output.push_str(&format!("User ID:         {}\n", id));
    }

    if let Some(name) = data.get("user").and_then(|u| u.get("name").and_then(|v| v.as_str())) {
        output.push_str(&format!("Name:            {}\n", name));
    }

    if let Some(account) = data.get("user").and_then(|u| u.get("account").and_then(|v| v.as_str())) {
        output.push_str(&format!("Account:         {}\n", account));
    }

    if let Some(profile) = data.get("profile") {
        if let Some(illusts) = profile.get("total_illusts") {
            output.push_str(&format!("Total Illusts:   {}\n", illusts));
        }

        if let Some(manga) = profile.get("total_manga") {
            output.push_str(&format!("Total Manga:     {}\n", manga));
        }

        if let Some(novels) = profile.get("total_novels") {
            output.push_str(&format!("Total Novels:    {}\n", novels));
        }

        if let Some(webpage) = profile.get("webpage").and_then(|v| v.as_str()) {
            if !webpage.is_empty() {
                output.push_str(&format!("Webpage:         {}\n", webpage));
            }
        }

        if let Some(twitter) = profile.get("twitter_account").and_then(|v| v.as_str()) {
            if !twitter.is_empty() {
                output.push_str(&format!("Twitter:         @{}\n", twitter));
            }
        }
    }

    Ok(output)
}

/// Format search results for display (adapted from original)
fn format_search_results_rust(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV SEARCH RESULTS (Rust)\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(keyword) = data.get("keyword").and_then(|v| v.as_str()) {
        output.push_str(&format!("Keyword:         {}\n", keyword));
    }

    if let Some(total) = data.get("total") {
        output.push_str(&format!("Results:         {}\n\n", total));
    }

    if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("{}. ", i + 1));

            if let Some(title) = result.get("title").and_then(|v| v.as_str()) {
                output.push_str(&format!("{}", title));
            }

            if let Some(id) = result.get("id") {
                output.push_str(&format!(" (ID: {})", id));
            }

            output.push('\n');

            if let Some(user) = result.get("user") {
                if let Some(name) = user.get("name").and_then(|v| v.as_str()) {
                    output.push_str(&format!("   Artist: {}\n", name));
                }
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("image_urls").and_then(|u| u.get("large").and_then(|v| v.as_str())) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Format ranking results for display (adapted from original)
fn format_ranking_results_rust(data: &Value, mode: &str) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV RANKING (Rust)\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    output.push_str(&format!("Mode:            {}\n", mode));

    if let Some(total) = data.get("total") {
        output.push_str(&format!("Results:         {}\n\n", total));
    }

    if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("{}. ", i + 1));

            if let Some(title) = result.get("title").and_then(|v| v.as_str()) {
                output.push_str(&format!("{}", title));
            }

            if let Some(id) = result.get("id") {
                output.push_str(&format!(" (ID: {})", id));
            }

            output.push('\n');

            if let Some(user) = result.get("user") {
                if let Some(name) = user.get("name").and_then(|v| v.as_str()) {
                    output.push_str(&format!("   Artist: {}\n", name));
                }
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("image_urls").and_then(|u| u.get("large").and_then(|v| v.as_str())) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Format user illusts results for display (adapted from original)
fn format_user_illusts_results_rust(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV USER ARTWORKS (Rust)\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(user_id) = data.get("user_id") {
        output.push_str(&format!("User ID:         {}\n", user_id));
    }

    if let Some(total) = data.get("total") {
        output.push_str(&format!("Results:         {}\n\n", total));
    }

    if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("{}. ", i + 1));

            if let Some(title) = result.get("title").and_then(|v| v.as_str()) {
                output.push_str(&format!("{}", title));
            }

            if let Some(id) = result.get("id") {
                output.push_str(&format!(" (ID: {})", id));
            }

            output.push('\n');

            if let Some(created_at) = result.get("created_at").and_then(|v| v.as_str()) {
                output.push_str(&format!("   Created: {}\n", created_at));
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("image_urls").and_then(|u| u.get("large").and_then(|v| v.as_str())) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Main entry point for Pixiv queries using Rust implementation
pub async fn process_pixiv_query_rust(query: &str) -> Result<String> {
    process_pixiv_query_internal_rust(query, false).await
}

pub async fn process_pixiv_query_json_rust(query: &str) -> Result<String> {
    process_pixiv_query_internal_rust(query, true).await
}

async fn process_pixiv_query_internal_rust(query: &str, json_output: bool) -> Result<String> {
    debug!("Processing Pixiv query (Rust): {}", query);

    // Remove -PIXIV suffix if present
    let base_query = if query.to_uppercase().ends_with("-PIXIV") {
        &query[..query.len() - 6]
    } else {
        query
    };

    // Parse query format:
    // - Pure number: artwork ID
    // - user:ID: user info
    // - search:keyword: search
    // - ranking or ranking:mode: ranking
    // - illusts:ID: user's artworks

    if base_query.starts_with("user:") {
        let user_id = &base_query[5..];
        if json_output {
            query_pixiv_user_json_rust(user_id).await
        } else {
            query_pixiv_user_rust(user_id).await
        }
    } else if base_query.starts_with("search:") {
        let keyword = &base_query[7..];
        if json_output {
            search_pixiv_artworks_json_rust(keyword, None).await
        } else {
            search_pixiv_artworks_rust(keyword, None).await
        }
    } else if base_query.starts_with("ranking") {
        let mode = if base_query.contains(':') {
            Some(&base_query[base_query.find(':').unwrap() + 1..])
        } else {
            None
        };
        if json_output {
            query_pixiv_ranking_json_rust(mode, None).await
        } else {
            query_pixiv_ranking_rust(mode, None).await
        }
    } else if base_query.starts_with("illusts:") {
        let user_id = &base_query[8..];
        if json_output {
            query_pixiv_user_illusts_json_rust(user_id, None).await
        } else {
            query_pixiv_user_illusts_rust(user_id, None).await
        }
    } else {
        // Default: treat as artwork ID
        if json_output {
            query_pixiv_artwork_json_rust(base_query).await
        } else {
            query_pixiv_artwork_rust(base_query).await
        }
    }
}