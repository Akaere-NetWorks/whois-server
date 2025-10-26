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

const PYPI_API_URL: &str = "https://pypi.org/pypi/";

#[derive(Debug, Deserialize, Serialize)]
struct PyPIResponse {
    info: PyPIPackageInfo,
    urls: Option<Vec<PyPIUrl>>,
    releases: Option<HashMap<String, Vec<PyPIRelease>>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PyPIPackageInfo {
    name: String,
    version: String,
    summary: Option<String>,
    description: Option<String>,
    home_page: Option<String>,
    download_url: Option<String>,
    author: Option<String>,
    author_email: Option<String>,
    maintainer: Option<String>,
    maintainer_email: Option<String>,
    license: Option<String>,
    keywords: Option<String>,
    classifiers: Option<Vec<String>>,
    requires_dist: Option<Vec<String>>,
    requires_python: Option<String>,
    project_urls: Option<HashMap<String, String>>,
    platform: Option<String>,
    package_url: Option<String>,
    project_url: Option<String>,
    release_url: Option<String>,
    docs_url: Option<String>,
    bugtrack_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PyPIUrl {
    filename: String,
    python_version: String,
    size: Option<u64>,
    upload_time: Option<String>,
    upload_time_iso_8601: Option<String>,
    url: String,
    md5_digest: Option<String>,
    sha256_digest: Option<String>,
    packagetype: String,
    requires_python: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PyPIRelease {
    filename: String,
    python_version: String,
    size: Option<u64>,
    upload_time: Option<String>,
    upload_time_iso_8601: Option<String>,
    url: String,
    md5_digest: Option<String>,
    sha256_digest: Option<String>,
    packagetype: String,
    requires_python: Option<String>,
}

pub async fn process_pypi_query(package_name: &str) -> Result<String> {
    debug!("Processing PyPI query for package: {}", package_name);

    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }

    // Validate PyPI package name format
    if package_name.len() > 214
        || !package_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    {
        return Err(anyhow::anyhow!("Invalid PyPI package name format"));
    }

    match query_pypi_package(package_name).await {
        Ok(package) => Ok(format_pypi_response(&package, package_name)),
        Err(e) => {
            error!("PyPI package query failed for {}: {}", package_name, e);
            Ok(format_pypi_not_found(package_name))
        }
    }
}

async fn query_pypi_package(package_name: &str) -> Result<PyPIResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WHOIS-Server/1.0)")
        .build()
        .context("Failed to create HTTP client")?;

    let package_url = format!("{}{}/json", PYPI_API_URL, urlencoding::encode(package_name));

    debug!("Querying PyPI API: {}", package_url);

    let response = client
        .get(&package_url)
        .send()
        .await
        .context("Failed to send request to PyPI API")?;

    if response.status() == 404 {
        return Err(anyhow::anyhow!("PyPI package not found"));
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "PyPI API returned status: {}",
            response.status()
        ));
    }

    let package_data: PyPIResponse = response
        .json()
        .await
        .context("Failed to parse PyPI package data")?;

    Ok(package_data)
}

fn format_pypi_response(package: &PyPIResponse, query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("PyPI Package Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    let info = &package.info;

    output.push_str(&format!("package-name: {}\n", info.name));
    output.push_str(&format!("version: {}\n", info.version));

    if let Some(summary) = &info.summary {
        output.push_str(&format!("summary: {}\n", summary));
    }

    // Author information
    if let Some(author) = &info.author {
        if let Some(email) = &info.author_email {
            output.push_str(&format!("author: {} <{}>\n", author, email));
        } else {
            output.push_str(&format!("author: {}\n", author));
        }
    } else if let Some(email) = &info.author_email {
        output.push_str(&format!("author: {}\n", email));
    }

    // Maintainer information
    if let Some(maintainer) = &info.maintainer {
        if let Some(email) = &info.maintainer_email {
            output.push_str(&format!("maintainer: {} <{}>\n", maintainer, email));
        } else {
            output.push_str(&format!("maintainer: {}\n", maintainer));
        }
    } else if let Some(email) = &info.maintainer_email {
        output.push_str(&format!("maintainer: {}\n", email));
    }

    // License
    if let Some(license) = &info.license
        && !license.is_empty()
    {
        output.push_str(&format!("license: {}\n", license));
    }

    // Homepage
    if let Some(homepage) = &info.home_page
        && !homepage.is_empty()
    {
        output.push_str(&format!("homepage: {}\n", homepage));
    }

    // Project URLs
    if let Some(project_urls) = &info.project_urls {
        for (key, url) in project_urls.iter().take(5) {
            output.push_str(&format!(
                "{}: {}\n",
                key.to_lowercase().replace(' ', "-"),
                url
            ));
        }
    }

    // Python version requirement
    if let Some(requires_python) = &info.requires_python {
        output.push_str(&format!("requires-python: {}\n", requires_python));
    }

    // Keywords
    if let Some(keywords) = &info.keywords
        && !keywords.is_empty()
    {
        output.push_str(&format!("keywords: {}\n", keywords));
    }

    // Dependencies
    if let Some(requires_dist) = &info.requires_dist
        && !requires_dist.is_empty()
    {
        let deps: Vec<String> = requires_dist
            .iter()
            .take(10)
            .map(|dep| {
                // Extract just the package name from dependency specification
                dep.split_whitespace().next().unwrap_or(dep).to_string()
            })
            .collect();
        output.push_str(&format!("dependencies: {}\n", deps.join(", ")));
        if requires_dist.len() > 10 {
            output.push_str(&format!(
                "... and {} more dependencies\n",
                requires_dist.len() - 10
            ));
        }
    }

    // Classifiers (programming language, license, etc.)
    if let Some(classifiers) = &info.classifiers {
        let lang_classifiers: Vec<&String> = classifiers
            .iter()
            .filter(|c| c.starts_with("Programming Language"))
            .take(3)
            .collect();
        if !lang_classifiers.is_empty() {
            let langs: Vec<String> = lang_classifiers
                .iter()
                .filter_map(|c| c.split("::").last())
                .map(|s| s.trim().to_string())
                .collect();
            output.push_str(&format!("programming-languages: {}\n", langs.join(", ")));
        }

        let status_classifiers: Vec<&String> = classifiers
            .iter()
            .filter(|c| c.starts_with("Development Status"))
            .take(1)
            .collect();
        if !status_classifiers.is_empty()
            && let Some(status) = status_classifiers
                .first()
                .and_then(|c| c.split("::").last())
        {
            output.push_str(&format!("development-status: {}\n", status.trim()));
        }
    }

    // File information from current release
    if let Some(urls) = &package.urls {
        let total_size: u64 = urls.iter().filter_map(|u| u.size).sum();
        if total_size > 0 {
            let size_mb = (total_size as f64) / 1024.0 / 1024.0;
            output.push_str(&format!("total-size: {:.2} MB\n", size_mb));
        }

        let wheel_count = urls
            .iter()
            .filter(|u| u.packagetype == "bdist_wheel")
            .count();
        let source_count = urls.iter().filter(|u| u.packagetype == "sdist").count();

        if wheel_count > 0 {
            output.push_str(&format!("wheel-files: {}\n", wheel_count));
        }
        if source_count > 0 {
            output.push_str(&format!("source-files: {}\n", source_count));
        }
    }

    // Platform
    if let Some(platform) = &info.platform
        && !platform.is_empty()
        && platform != "UNKNOWN"
    {
        output.push_str(&format!("platform: {}\n", platform));
    }

    output.push_str(&format!(
        "pypi-url: https://pypi.org/project/{}/\n",
        urlencoding::encode(&info.name)
    ));
    output.push_str(&format!(
        "api-url: {}{}/json\n",
        PYPI_API_URL,
        urlencoding::encode(&info.name)
    ));
    output.push_str("repository: Python Package Index (PyPI)\n");
    output.push_str("source: PyPI API\n");
    output.push('\n');
    output.push_str("% Information retrieved from PyPI\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_pypi_not_found(package_name: &str) -> String {
    format!(
        "PyPI Package Not Found: {}\n\
        No package with this name was found in PyPI.\n\
        \n\
        You can search manually at: https://pypi.org/search/?q={}\n\
        \n\
        % Package not found in PyPI\n\
        % Query processed by WHOIS server\n",
        package_name,
        urlencoding::encode(package_name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pypi_package_name_validation() {
        // Valid package names
        assert!(process_pypi_query("numpy").await.is_ok());
        assert!(process_pypi_query("requests").await.is_ok());
        assert!(process_pypi_query("django-rest-framework").await.is_ok());

        // Invalid package names
        assert!(process_pypi_query("").await.is_err());
        assert!(process_pypi_query(&"a".repeat(215)).await.is_err());
    }

    #[tokio::test]
    async fn test_pypi_service_creation() {
        let result = process_pypi_query("nonexistent-package-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("PyPI Package"));
    }
}
