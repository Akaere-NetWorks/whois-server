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

use anyhow::{Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error};

const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/";

#[derive(Debug, Deserialize, Serialize)]
struct NPMPackageResponse {
    name: String,
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<NPMRepository>,
    author: Option<NPMAuthor>,
    maintainers: Option<Vec<NPMAuthor>>,
    license: Option<String>,
    keywords: Option<Vec<String>>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<HashMap<String, String>>,
    time: Option<HashMap<String, String>>,
    versions: HashMap<String, NPMVersion>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NPMPackage {
    name: String,
    description: Option<String>,
    version: String,
    homepage: Option<String>,
    repository: Option<NPMRepository>,
    author: Option<NPMAuthor>,
    maintainers: Option<Vec<NPMAuthor>>,
    license: Option<String>,
    keywords: Option<Vec<String>>,
    dependencies: Option<HashMap<String, String>>,
    dev_dependencies: Option<HashMap<String, String>>,
    dist: Option<NPMDist>,
    engines: Option<HashMap<String, String>>,
    dist_tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NPMVersion {
    name: String,
    version: String,
    description: Option<String>,
    main: Option<String>,
    scripts: Option<HashMap<String, String>>,
    author: Option<NPMAuthor>,
    license: Option<String>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    keywords: Option<Vec<String>>,
    repository: Option<NPMRepository>,
    homepage: Option<String>,
    dist: Option<NPMDist>,
    engines: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum NPMAuthor {
    String(String),
    Object {
        name: Option<String>,
        email: Option<String>,
        url: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum NPMRepository {
    String(String),
    Object {
        #[serde(rename = "type")]
        repo_type: Option<String>,
        url: Option<String>,
        directory: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct NPMDist {
    tarball: Option<String>,
    shasum: Option<String>,
    integrity: Option<String>,
    #[serde(rename = "unpackedSize")]
    unpacked_size: Option<u64>,
    #[serde(rename = "fileCount")]
    file_count: Option<u64>,
}

pub async fn process_npm_query(package_name: &str) -> Result<String> {
    debug!("Processing NPM query for package: {}", package_name);

    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }

    // Validate NPM package name format (including scoped packages like @types/node)
    if package_name.len() > 214
        || package_name.contains(' ')
        || package_name.to_lowercase() != package_name
    {
        return Err(anyhow::anyhow!("Invalid NPM package name format"));
    }

    // Handle scoped packages (starting with @)
    if package_name.starts_with('@') {
        // Scoped package format: @scope/name
        if !package_name.contains('/') {
            return Err(anyhow::anyhow!(
                "Invalid scoped NPM package format. Use @scope/name"
            ));
        }
        let parts: Vec<&str> = package_name.splitn(2, '/').collect();
        if parts.len() != 2 || parts[0].len() <= 1 || parts[1].is_empty() {
            return Err(anyhow::anyhow!("Invalid scoped NPM package format"));
        }
        // Validate scope and package name parts
        let scope = &parts[0][1..]; // Remove @ prefix
        let name = parts[1];
        if scope.is_empty()
            || name.is_empty()
            || scope.starts_with('.')
            || scope.starts_with('_')
            || name.starts_with('.')
            || name.starts_with('_')
        {
            return Err(anyhow::anyhow!("Invalid scoped NPM package format"));
        }
    } else {
        // Regular package validation
        if package_name.starts_with('.') || package_name.starts_with('_') {
            return Err(anyhow::anyhow!("Invalid NPM package name format"));
        }
    }

    match query_npm_package(package_name).await {
        Ok(package) => Ok(format_npm_response(&package, package_name)),
        Err(e) => {
            error!("NPM package query failed for {}: {}", package_name, e);
            Ok(format_npm_not_found(package_name))
        }
    }
}

async fn query_npm_package(package_name: &str) -> Result<NPMPackage> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WHOIS-Server/1.0)")
        .build()
        .context("Failed to create HTTP client")?;

    // Handle scoped packages - NPM Registry expects %2F for / in scoped packages
    let encoded_name = if package_name.starts_with('@') {
        // For scoped packages, encode @ as %40 and / as %2F
        package_name.replace("@", "%40").replace("/", "%2F")
    } else {
        urlencoding::encode(package_name).to_string()
    };
    let package_url = format!("{}{}", NPM_REGISTRY_URL, encoded_name);

    debug!("Querying NPM registry: {}", package_url);

    let response = client
        .get(&package_url)
        .send()
        .await
        .context("Failed to send request to NPM registry")?;

    if response.status() == 404 {
        return Err(anyhow::anyhow!("NPM package not found"));
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "NPM registry returned status: {}",
            response.status()
        ));
    }

    let package_response: NPMPackageResponse = response
        .json()
        .await
        .context("Failed to parse NPM package data")?;

    // Get the latest version from dist-tags
    let latest_version = package_response
        .dist_tags
        .as_ref()
        .and_then(|tags| tags.get("latest"))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // Get version-specific data from versions object
    let version_data = package_response.versions.get(&latest_version);

    // Construct the final package object
    let package = NPMPackage {
        name: package_response.name.clone(),
        description: package_response
            .description
            .or_else(|| version_data.and_then(|v| v.description.clone())),
        version: latest_version.clone(),
        homepage: package_response
            .homepage
            .or_else(|| version_data.and_then(|v| v.homepage.clone())),
        repository: package_response
            .repository
            .or_else(|| version_data.and_then(|v| v.repository.clone())),
        author: package_response
            .author
            .or_else(|| version_data.and_then(|v| v.author.clone())),
        maintainers: package_response.maintainers,
        license: package_response
            .license
            .or_else(|| version_data.and_then(|v| v.license.clone())),
        keywords: package_response
            .keywords
            .or_else(|| version_data.and_then(|v| v.keywords.clone())),
        dependencies: version_data.and_then(|v| v.dependencies.clone()),
        dev_dependencies: version_data.and_then(|v| v.dev_dependencies.clone()),
        dist: version_data.and_then(|v| v.dist.clone()),
        engines: version_data.and_then(|v| v.engines.clone()),
        dist_tags: package_response.dist_tags,
    };

    Ok(package)
}

fn format_npm_response(package: &NPMPackage, query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("NPM Package Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    output.push_str(&format!("package-name: {}\n", package.name));
    output.push_str(&format!("version: {}\n", package.version));

    if let Some(description) = &package.description {
        output.push_str(&format!("description: {}\n", description));
    }

    // Author information
    if let Some(author) = &package.author {
        let author_str = match author {
            NPMAuthor::String(s) => s.clone(),
            NPMAuthor::Object { name, email, .. } => {
                if let Some(name) = name {
                    if let Some(email) = email {
                        format!("{} <{}>", name, email)
                    } else {
                        name.clone()
                    }
                } else if let Some(email) = email {
                    email.clone()
                } else {
                    "Unknown".to_string()
                }
            }
        };
        output.push_str(&format!("author: {}\n", author_str));
    }

    // License
    if let Some(license) = &package.license {
        output.push_str(&format!("license: {}\n", license));
    }

    // Homepage
    if let Some(homepage) = &package.homepage {
        output.push_str(&format!("homepage: {}\n", homepage));
    }

    // Repository
    if let Some(repository) = &package.repository {
        let repo_url = match repository {
            NPMRepository::String(s) => s.clone(),
            NPMRepository::Object { url: Some(url), .. } => url.clone(),
            _ => "Unknown".to_string(),
        };
        if repo_url != "Unknown" {
            output.push_str(&format!("repository: {}\n", repo_url));
        }
    }

    // Keywords
    if let Some(keywords) = &package.keywords
        && !keywords.is_empty()
    {
        output.push_str(&format!("keywords: {}\n", keywords.join(", ")));
    }

    // Dependencies
    if let Some(dependencies) = &package.dependencies
        && !dependencies.is_empty()
    {
        let deps: Vec<String> = dependencies.keys().take(10).cloned().collect();
        output.push_str(&format!("dependencies: {}\n", deps.join(", ")));
        if dependencies.len() > 10 {
            output.push_str(&format!(
                "... and {} more dependencies\n",
                dependencies.len() - 10
            ));
        }
    }

    // Maintainers
    if let Some(maintainers) = &package.maintainers
        && !maintainers.is_empty()
    {
        let maintainer_names: Vec<String> = maintainers
            .iter()
            .take(5)
            .map(|m| match m {
                NPMAuthor::String(s) => s.clone(),
                NPMAuthor::Object {
                    name: Some(name), ..
                } => name.clone(),
                NPMAuthor::Object {
                    email: Some(email), ..
                } => email.clone(),
                _ => "Unknown".to_string(),
            })
            .collect();
        output.push_str(&format!("maintainers: {}\n", maintainer_names.join(", ")));
    }

    // Distribution info
    if let Some(dist) = &package.dist {
        if let Some(unpacked_size) = dist.unpacked_size {
            let size_mb = (unpacked_size as f64) / 1024.0 / 1024.0;
            output.push_str(&format!("unpacked-size: {:.2} MB\n", size_mb));
        }
        if let Some(file_count) = dist.file_count {
            output.push_str(&format!("file-count: {}\n", file_count));
        }
    }

    // Latest versions
    if let Some(dist_tags) = &package.dist_tags {
        if let Some(latest) = dist_tags.get("latest") {
            output.push_str(&format!("latest-version: {}\n", latest));
        }
        if let Some(beta) = dist_tags.get("beta") {
            output.push_str(&format!("beta-version: {}\n", beta));
        }
    }

    // Handle scoped packages in URLs
    let encoded_for_web = if package.name.starts_with('@') {
        package.name.clone() // NPM web interface doesn't need encoding for @
    } else {
        urlencoding::encode(&package.name).to_string()
    };
    let encoded_for_api = if package.name.starts_with('@') {
        package.name.replace("@", "%40").replace("/", "%2F")
    } else {
        urlencoding::encode(&package.name).to_string()
    };

    output.push_str(&format!(
        "npm-url: https://www.npmjs.com/package/{}\n",
        encoded_for_web
    ));
    output.push_str(&format!(
        "registry-url: {}{}\n",
        NPM_REGISTRY_URL, encoded_for_api
    ));
    output.push_str("repository: NPM Registry\n");
    output.push_str("source: NPM Registry API\n");
    output.push('\n');
    output.push_str("% Information retrieved from NPM registry\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_npm_not_found(package_name: &str) -> String {
    // Handle scoped packages in search URL
    let encoded_search = urlencoding::encode(package_name);

    format!(
        "NPM Package Not Found: {}\n\
        No package with this name was found in NPM registry.\n\
        \n\
        You can search manually at: https://www.npmjs.com/search?q={}\n\
        \n\
        % Package not found in NPM registry\n\
        % Query processed by WHOIS server\n",
        package_name, encoded_search
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_npm_package_name_validation() {
        // Valid package names
        assert!(process_npm_query("react").await.is_ok());
        assert!(process_npm_query("@types/node").await.is_ok());
        assert!(process_npm_query("@angular/core").await.is_ok());
        assert!(process_npm_query("lodash").await.is_ok());

        // Invalid package names
        assert!(process_npm_query("").await.is_err());
        assert!(process_npm_query("Package With Spaces").await.is_err());
        assert!(process_npm_query(".hidden").await.is_err());
        assert!(process_npm_query("_private").await.is_err());
        assert!(process_npm_query("UPPERCASE").await.is_err());
        assert!(process_npm_query("@").await.is_err());
        assert!(process_npm_query("@scope").await.is_err());
        assert!(process_npm_query("@scope/").await.is_err());
        assert!(process_npm_query("@/.hidden").await.is_err());
    }

    #[tokio::test]
    async fn test_npm_service_creation() {
        let result = process_npm_query("nonexistent-package-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("NPM Package"));
    }
}
