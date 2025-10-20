/*
 * WHOIS Server with DN42 Support
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

use anyhow::{ Context, Result };
use reqwest;
use serde::{ Deserialize, Serialize };
use tracing::{ debug, error };
use std::collections::HashMap;

const CRATES_IO_API_URL: &str = "https://crates.io/api/v1/crates/";

#[derive(Debug, Deserialize, Serialize)]
struct CratesResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
    versions: Vec<CrateVersion>,
    keywords: Option<Vec<CrateKeyword>>,
    categories: Option<Vec<CrateCategory>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateInfo {
    id: String,
    name: String,
    description: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    repository: Option<String>,
    downloads: u64,
    recent_downloads: Option<u64>,
    max_stable_version: Option<String>,
    max_version: String,
    newest_version: String,
    created_at: String,
    updated_at: String,
    exact_match: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateVersion {
    id: u64,
    #[serde(rename = "crate")]
    crate_name: String,
    num: String,
    dl_path: String,
    readme_path: Option<String>,
    updated_at: String,
    created_at: String,
    downloads: u64,
    features: Option<HashMap<String, Vec<String>>>,
    yanked: bool,
    license: Option<String>,
    crate_size: Option<u64>,
    published_by: Option<CrateUser>,
    audit_actions: Option<Vec<serde_json::Value>>,
    links: Option<HashMap<String, String>>,
    checksum: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateUser {
    id: u64,
    login: String,
    name: Option<String>,
    avatar: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateKeyword {
    id: String,
    keyword: String,
    crates_cnt: u64,
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateCategory {
    id: String,
    category: String,
    slug: String,
    description: String,
    crates_cnt: u64,
    created_at: String,
}

pub async fn process_cargo_query(crate_name: &str) -> Result<String> {
    debug!("Processing Cargo query for crate: {}", crate_name);

    if crate_name.is_empty() {
        return Err(anyhow::anyhow!("Crate name cannot be empty"));
    }

    // Validate Cargo crate name format
    if
        crate_name.len() > 64 ||
        !crate_name.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) ||
        crate_name.starts_with('-') ||
        crate_name.ends_with('-')
    {
        return Err(anyhow::anyhow!("Invalid Cargo crate name format"));
    }

    match query_crates_io_crate(crate_name).await {
        Ok(crate_data) => Ok(format_cargo_response(&crate_data, crate_name)),
        Err(e) => {
            error!("Cargo crate query failed for {}: {}", crate_name, e);
            Ok(format_cargo_not_found(crate_name))
        }
    }
}

async fn query_crates_io_crate(crate_name: &str) -> Result<CratesResponse> {
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WHOIS-Server/1.0)")
        .build()
        .context("Failed to create HTTP client")?;

    let crate_url = format!("{}{}", CRATES_IO_API_URL, urlencoding::encode(crate_name));

    debug!("Querying crates.io API: {}", crate_url);

    let response = client
        .get(&crate_url)
        .send().await
        .context("Failed to send request to crates.io API")?;

    if response.status() == 404 {
        return Err(anyhow::anyhow!("Crate not found"));
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("crates.io API returned status: {}", response.status()));
    }

    let crate_data: CratesResponse = response
        .json().await
        .context("Failed to parse crates.io response")?;

    Ok(crate_data)
}

fn format_cargo_response(crate_data: &CratesResponse, query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("Rust Crate Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    let crate_info = &crate_data.crate_info;

    output.push_str(&format!("crate-name: {}\n", crate_info.name));
    output.push_str(&format!("version: {}\n", crate_info.newest_version));

    if let Some(max_stable) = &crate_info.max_stable_version
        && max_stable != &crate_info.newest_version {
            output.push_str(&format!("stable-version: {}\n", max_stable));
        }

    if let Some(description) = &crate_info.description {
        output.push_str(&format!("description: {}\n", description));
    }

    // Version info from the latest version
    if let Some(latest_version) = crate_data.versions.first() {
        if let Some(license) = &latest_version.license {
            output.push_str(&format!("license: {}\n", license));
        }

        if let Some(published_by) = &latest_version.published_by {
            if let Some(name) = &published_by.name {
                output.push_str(&format!("published-by: {} ({})\n", name, published_by.login));
            } else {
                output.push_str(&format!("published-by: {}\n", published_by.login));
            }
        }

        if latest_version.yanked {
            output.push_str("yanked: true\n");
        }

        if let Some(crate_size) = latest_version.crate_size {
            let size_kb = (crate_size as f64) / 1024.0;
            if size_kb >= 1024.0 {
                output.push_str(&format!("package-size: {:.2} MB\n", size_kb / 1024.0));
            } else {
                output.push_str(&format!("package-size: {:.2} KB\n", size_kb));
            }
        }
    }

    // URLs
    if let Some(homepage) = &crate_info.homepage
        && !homepage.is_empty() {
            output.push_str(&format!("homepage: {}\n", homepage));
        }

    if let Some(repository) = &crate_info.repository
        && !repository.is_empty() {
            output.push_str(&format!("repository: {}\n", repository));
        }

    if let Some(documentation) = &crate_info.documentation
        && !documentation.is_empty() {
            output.push_str(&format!("documentation: {}\n", documentation));
        }

    // Download statistics
    output.push_str(&format!("total-downloads: {}\n", format_number(crate_info.downloads)));
    if let Some(recent) = crate_info.recent_downloads {
        output.push_str(&format!("recent-downloads: {}\n", format_number(recent)));
    }

    // Categories
    if let Some(categories) = &crate_data.categories
        && !categories.is_empty() {
            let cat_names: Vec<String> = categories
                .iter()
                .take(5)
                .map(|c| c.category.clone())
                .collect();
            output.push_str(&format!("categories: {}\n", cat_names.join(", ")));
        }

    // Keywords
    if let Some(keywords) = &crate_data.keywords
        && !keywords.is_empty() {
            let keyword_names: Vec<String> = keywords
                .iter()
                .take(10)
                .map(|k| k.keyword.clone())
                .collect();
            output.push_str(&format!("keywords: {}\n", keyword_names.join(", ")));
        }

    // Features from latest version
    if let Some(latest_version) = crate_data.versions.first()
        && let Some(features) = &latest_version.features {
            let feature_count = features.len();
            if feature_count > 0 {
                output.push_str(&format!("features: {} available\n", feature_count));

                // Show default features if available
                if let Some(default_features) = features.get("default")
                    && !default_features.is_empty() {
                        output.push_str(
                            &format!("default-features: {}\n", default_features.join(", "))
                        );
                    }
            }
        }

    // Version history (show last 5 versions)
    let version_count = crate_data.versions.len();
    if version_count > 1 {
        output.push_str(&format!("total-versions: {}\n", version_count));
        let recent_versions: Vec<String> = crate_data.versions
            .iter()
            .take(5)
            .map(|v| {
                if v.yanked { format!("{} (yanked)", v.num) } else { v.num.clone() }
            })
            .collect();
        output.push_str(&format!("recent-versions: {}\n", recent_versions.join(", ")));
    }

    // Timestamps
    output.push_str(&format!("created: {}\n", format_timestamp(&crate_info.created_at)));
    output.push_str(&format!("updated: {}\n", format_timestamp(&crate_info.updated_at)));

    // URLs
    output.push_str(
        &format!(
            "crates-io-url: https://crates.io/crates/{}\n",
            urlencoding::encode(&crate_info.name)
        )
    );
    output.push_str(
        &format!("docs-rs-url: https://docs.rs/{}\n", urlencoding::encode(&crate_info.name))
    );
    output.push_str(
        &format!("api-url: {}{}\n", CRATES_IO_API_URL, urlencoding::encode(&crate_info.name))
    );
    output.push_str("registry: crates.io (Rust Package Registry)\n");
    output.push_str("source: crates.io API\n");
    output.push('\n');
    output.push_str("% Information retrieved from crates.io\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_cargo_not_found(crate_name: &str) -> String {
    format!(
        "Rust Crate Not Found: {}\n\
        No crate with this name was found in crates.io.\n\
        \n\
        You can search manually at: https://crates.io/search?q={}\n\
        \n\
        % Crate not found in crates.io\n\
        % Query processed by WHOIS server\n",
        crate_name,
        urlencoding::encode(crate_name)
    )
}

fn format_number(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", (num as f64) / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", (num as f64) / 1_000.0)
    } else {
        num.to_string()
    }
}

fn format_timestamp(timestamp: &str) -> String {
    // Convert ISO timestamp to more readable format
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        timestamp.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cargo_crate_name_validation() {
        // Valid crate names
        assert!(process_cargo_query("serde").await.is_ok());
        assert!(process_cargo_query("tokio").await.is_ok());
        assert!(process_cargo_query("serde_json").await.is_ok());
        assert!(process_cargo_query("my-crate").await.is_ok());

        // Invalid crate names
        assert!(process_cargo_query("").await.is_err());
        assert!(process_cargo_query("-invalid").await.is_err());
        assert!(process_cargo_query("invalid-").await.is_err());
        assert!(process_cargo_query(&"a".repeat(65)).await.is_err());
    }

    #[tokio::test]
    async fn test_cargo_service_creation() {
        let result = process_cargo_query("nonexistent-crate-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Rust Crate"));
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1500000), "1.5M");
    }
}
