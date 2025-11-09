/*
 * WHOIS Server with DN42 Support - Pixiv Service
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
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_json::Value;
use std::path::PathBuf;
use tracing::{debug, error};

/// Initialize Python interpreter and add pixiv module to path
fn init_python_env(py: Python) -> PyResult<()> {
    // Get the path to the services directory (parent of pixiv)
    let services_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services");

    // Add the services path to sys.path so Python can find the pixiv package
    let sys = py.import("sys")?;
    let path = sys.getattr("path")?;

    // Convert path to string and add to sys.path if not already present
    let path_str = services_path.to_string_lossy().to_string();

    // Check if path is already in sys.path
    let contains = path.call_method1("__contains__", (&path_str,))?;
    let contains_bool: bool = contains.extract()?;

    if !contains_bool {
        path.call_method1("insert", (0, path_str))?;
    }

    Ok(())
}

/// Query Pixiv artwork information by ID
pub async fn query_pixiv_artwork(artwork_id: &str) -> Result<String> {
    debug!("Querying Pixiv artwork: {}", artwork_id);

    // Parse artwork ID
    let id: i64 = artwork_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid artwork ID: {}", artwork_id))?;

    // Call Python function
    let result = Python::attach(|py| -> PyResult<String> {
        init_python_env(py)?;

        // Import the pixiv module
        let pixiv_module = PyModule::import(py, "pixiv")?;
        let get_artwork_info = pixiv_module.getattr("get_artwork_info")?;

        // Call the function
        let result = get_artwork_info.call1((id,))?;

        // Convert result to JSON string
        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((result,))?
            .extract::<String>()?;

        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let data: Value = serde_json::from_str(&json_str)?;

            if let Some(error) = data.get("error") {
                return Ok(format!(
                    "% Error querying Pixiv artwork: {}\n",
                    error.as_str().unwrap_or("Unknown error")
                ));
            }

            format_artwork_info(&data)
        }
        Err(e) => {
            error!("Python error: {}", e);
            Ok(format!("% Error: Failed to query Pixiv: {}\n", e))
        }
    }
}

/// Query Pixiv user information by ID
pub async fn query_pixiv_user(user_id: &str) -> Result<String> {
    debug!("Querying Pixiv user: {}", user_id);

    // Parse user ID
    let id: i64 = user_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid user ID: {}", user_id))?;

    // Call Python function
    let result = Python::attach(|py| -> PyResult<String> {
        init_python_env(py)?;

        let pixiv_module = PyModule::import(py, "pixiv")?;
        let get_user_info = pixiv_module.getattr("get_user_info")?;

        let result = get_user_info.call1((id,))?;

        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((result,))?
            .extract::<String>()?;

        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let data: Value = serde_json::from_str(&json_str)?;

            if let Some(error) = data.get("error") {
                return Ok(format!(
                    "% Error querying Pixiv user: {}\n",
                    error.as_str().unwrap_or("Unknown error")
                ));
            }

            format_user_info(&data)
        }
        Err(e) => {
            error!("Python error: {}", e);
            Ok(format!("% Error: Failed to query Pixiv: {}\n", e))
        }
    }
}

/// Search Pixiv artworks by keyword
pub async fn search_pixiv_artworks(keyword: &str, limit: Option<i32>) -> Result<String> {
    debug!("Searching Pixiv artworks: {}", keyword);

    let limit = limit.unwrap_or(10);

    // Call Python function
    let result = Python::attach(|py| -> PyResult<String> {
        init_python_env(py)?;

        let pixiv_module = PyModule::import(py, "pixiv")?;
        let search_artworks = pixiv_module.getattr("search_artworks")?;

        let result = search_artworks.call1((keyword, limit))?;

        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((result,))?
            .extract::<String>()?;

        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let data: Value = serde_json::from_str(&json_str)?;

            if let Some(error) = data.get("error") {
                return Ok(format!(
                    "% Error searching Pixiv: {}\n",
                    error.as_str().unwrap_or("Unknown error")
                ));
            }

            format_search_results(&data)
        }
        Err(e) => {
            error!("Python error: {}", e);
            Ok(format!("% Error: Failed to search Pixiv: {}\n", e))
        }
    }
}

/// Get Pixiv ranking
pub async fn query_pixiv_ranking(mode: Option<&str>, limit: Option<i32>) -> Result<String> {
    let mode = mode.unwrap_or("day");
    let limit = limit.unwrap_or(10);

    debug!("Querying Pixiv ranking: mode={}, limit={}", mode, limit);

    // Call Python function
    let result = Python::attach(|py| -> PyResult<String> {
        init_python_env(py)?;

        let pixiv_module = PyModule::import(py, "pixiv")?;
        let get_ranking = pixiv_module.getattr("get_ranking")?;

        let result = get_ranking.call1((mode, limit))?;

        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((result,))?
            .extract::<String>()?;

        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let data: Value = serde_json::from_str(&json_str)?;

            if let Some(error) = data.get("error") {
                return Ok(format!(
                    "% Error querying Pixiv ranking: {}\n",
                    error.as_str().unwrap_or("Unknown error")
                ));
            }

            format_ranking_results(&data, mode)
        }
        Err(e) => {
            error!("Python error: {}", e);
            Ok(format!("% Error: Failed to query Pixiv ranking: {}\n", e))
        }
    }
}

/// Get user's artworks
pub async fn query_pixiv_user_illusts(user_id: &str, limit: Option<i32>) -> Result<String> {
    debug!("Querying Pixiv user illusts: {}", user_id);

    let id: i64 = user_id
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid user ID: {}", user_id))?;

    let limit = limit.unwrap_or(10);

    // Call Python function
    let result = Python::attach(|py| -> PyResult<String> {
        init_python_env(py)?;

        let pixiv_module = PyModule::import(py, "pixiv")?;
        let get_user_illusts = pixiv_module.getattr("get_user_illusts")?;

        let result = get_user_illusts.call1((id, limit))?;

        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((result,))?
            .extract::<String>()?;

        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let data: Value = serde_json::from_str(&json_str)?;

            if let Some(error) = data.get("error") {
                return Ok(format!(
                    "% Error querying user illusts: {}\n",
                    error.as_str().unwrap_or("Unknown error")
                ));
            }

            format_user_illusts_results(&data)
        }
        Err(e) => {
            error!("Python error: {}", e);
            Ok(format!("% Error: Failed to query user illusts: {}\n", e))
        }
    }
}

/// Format artwork information for display
fn format_artwork_info(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV ARTWORK INFORMATION\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(id) = data.get("id") {
        output.push_str(&format!("Artwork ID:      {}\n", id));
    }

    if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
        output.push_str(&format!("Title:           {}\n", title));
    }

    if let Some(art_type) = data.get("type").and_then(|v| v.as_str()) {
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

    if let Some(create_date) = data.get("create_date").and_then(|v| v.as_str()) {
        output.push_str(&format!("Created:         {}\n", create_date));
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
            .filter_map(|t| t.as_str())
            .map(|s| s.to_string())
            .collect();
        output.push_str(&tag_names.join(", "));
        output.push('\n');
    }

    if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
        output.push_str(&format!("\nURL:             {}\n", url));
    }

    // 显示图片链接
    if let Some(image_urls) = data.get("image_urls") {
        output.push_str("\nImage URLs:\n");

        if let Some(original) = image_urls.get("original").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Original:      {}\n", original));
        }

        if let Some(large) = image_urls.get("large").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Large:         {}\n", large));
        }

        if let Some(medium) = image_urls.get("medium").and_then(|v| v.as_str()) {
            output.push_str(&format!("  Medium:        {}\n", medium));
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

                if let Some(original) = page.get("original").and_then(|v| v.as_str()) {
                    output.push_str(&format!("    Original:  {}\n", original));
                }

                if let Some(large) = page.get("large").and_then(|v| v.as_str()) {
                    output.push_str(&format!("    Large:     {}\n", large));
                }
            }
        }
    }

    if let Some(caption) = data.get("caption").and_then(|v| v.as_str()) {
        if !caption.is_empty() {
            output.push_str(&format!("\nCaption:\n{}\n", caption));
        }
    }

    Ok(output)
}

/// Format user information for display
fn format_user_info(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV USER INFORMATION\n");
    output.push_str("=".repeat(60).as_str());
    output.push('\n');
    output.push('\n');

    if let Some(id) = data.get("id") {
        output.push_str(&format!("User ID:         {}\n", id));
    }

    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        output.push_str(&format!("Name:            {}\n", name));
    }

    if let Some(account) = data.get("account").and_then(|v| v.as_str()) {
        output.push_str(&format!("Account:         {}\n", account));
    }

    if let Some(illusts) = data.get("total_illusts") {
        output.push_str(&format!("Total Illusts:   {}\n", illusts));
    }

    if let Some(manga) = data.get("total_manga") {
        output.push_str(&format!("Total Manga:     {}\n", manga));
    }

    if let Some(novels) = data.get("total_novels") {
        output.push_str(&format!("Total Novels:    {}\n", novels));
    }

    if let Some(bookmarks) = data.get("total_bookmarks") {
        output.push_str(&format!("Public Bookmarks:{}\n", bookmarks));
    }

    if let Some(twitter) = data.get("twitter_account").and_then(|v| v.as_str()) {
        if !twitter.is_empty() {
            output.push_str(&format!("Twitter:         @{}\n", twitter));
        }
    }

    if let Some(webpage) = data.get("webpage").and_then(|v| v.as_str()) {
        if !webpage.is_empty() {
            output.push_str(&format!("Webpage:         {}\n", webpage));
        }
    }

    if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
        output.push_str(&format!("\nProfile URL:     {}\n", url));
    }

    if let Some(comment) = data.get("comment").and_then(|v| v.as_str()) {
        if !comment.is_empty() {
            output.push_str(&format!("\nComment:\n{}\n", comment));
        }
    }

    Ok(output)
}

/// Format search results for display
fn format_search_results(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV SEARCH RESULTS\n");
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

            if let Some(user_name) = result.get("user_name").and_then(|v| v.as_str()) {
                output.push_str(&format!("   Artist: {}\n", user_name));
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Format ranking results for display
fn format_ranking_results(data: &Value, mode: &str) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV RANKING\n");
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

            if let Some(user_name) = result.get("user_name").and_then(|v| v.as_str()) {
                output.push_str(&format!("   Artist: {}\n", user_name));
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Format user illusts results for display
fn format_user_illusts_results(data: &Value) -> Result<String> {
    let mut output = String::new();

    output.push_str("PIXIV USER ARTWORKS\n");
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

            if let Some(create_date) = result.get("create_date").and_then(|v| v.as_str()) {
                output.push_str(&format!("   Created: {}\n", create_date));
            }

            if let Some(bookmarks) = result.get("total_bookmarks") {
                output.push_str(&format!("   Bookmarks: {}\n", bookmarks));
            }

            if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                output.push_str(&format!("   URL: {}\n", url));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Main entry point for Pixiv queries
pub async fn process_pixiv_query(query: &str) -> Result<String> {
    debug!("Processing Pixiv query: {}", query);

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
        query_pixiv_user(user_id).await
    } else if base_query.starts_with("search:") {
        let keyword = &base_query[7..];
        search_pixiv_artworks(keyword, None).await
    } else if base_query.starts_with("ranking") {
        let mode = if base_query.contains(':') {
            Some(&base_query[base_query.find(':').unwrap() + 1..])
        } else {
            None
        };
        query_pixiv_ranking(mode, None).await
    } else if base_query.starts_with("illusts:") {
        let user_id = &base_query[8..];
        query_pixiv_user_illusts(user_id, None).await
    } else {
        // Default: treat as artwork ID
        query_pixiv_artwork(base_query).await
    }
}
