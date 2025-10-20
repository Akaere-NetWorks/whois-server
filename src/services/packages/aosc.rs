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
use regex::Regex;

const AOSC_PACKAGES_URL: &str = "https://packages.aosc.io/packages/";
const AOSC_SEARCH_URL: &str = "https://packages.aosc.io/search?q=";

#[derive(Debug, Deserialize, Serialize)]
struct AOSCPackage {
    name: String,
    version: String,
    description: String,
    section: Option<String>,
    depends: Vec<String>,
    replaces: Vec<String>,
    breaks: Vec<String>,
    provides: Vec<String>,
    suggests: Vec<String>,
    upstream_url: Option<String>,
    upstream_version: Option<String>,
    architectures: Vec<AOSCArchitecture>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AOSCArchitecture {
    name: String,
    version: String,
    size: String,
    download_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AOSCSearchResponse {
    packages: Vec<AOSCPackage>,
}

pub async fn process_aosc_query(package_name: &str) -> Result<String> {
    debug!("Processing AOSC query for package: {}", package_name);

    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }

    // Validate package name
    if
        package_name.len() > 100 ||
        package_name.contains(' ') ||
        !package_name.chars().all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c))
    {
        return Err(anyhow::anyhow!("Invalid AOSC package name format"));
    }

    match query_aosc_packages(package_name).await {
        Ok(search_result) => {
            if !search_result.packages.is_empty() {
                Ok(format_aosc_response(&search_result.packages, package_name))
            } else {
                Ok(format_aosc_not_found(package_name))
            }
        }
        Err(e) => {
            error!("AOSC packages query failed for {}: {}", package_name, e);
            Ok(format_aosc_not_found(package_name))
        }
    }
}

async fn query_aosc_packages(package_name: &str) -> Result<AOSCSearchResponse> {
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"
        )
        .build()
        .context("Failed to create HTTP client")?;

    // Use AOSC packages web page
    let package_url = format!("{}{}", AOSC_PACKAGES_URL, urlencoding::encode(package_name));

    debug!("Querying AOSC packages page: {}", package_url);

    let response = client
        .get(&package_url)
        .send().await
        .context("Failed to send request to AOSC packages page")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("AOSC packages page returned status: {}", response.status()));
    }

    let html_content = response
        .text().await
        .context("Failed to get HTML content from AOSC packages page")?;

    parse_aosc_html(&html_content, package_name)
}

fn parse_aosc_html(html: &str, query: &str) -> Result<AOSCSearchResponse> {
    // Parse the HTML to extract package information
    let mut packages = Vec::new();

    // Extract package version from header
    let version_regex = Regex::new(r#"<span class="pkg-version">([^<]+)</span>"#).unwrap();
    let version = version_regex
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map_or("unknown".to_string(), |m| m.as_str().to_string());

    // Extract description
    let desc_regex = Regex::new(r#"<p class="description pkg-description">([^<]+)</p>"#).unwrap();
    let description = desc_regex
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map_or(format!("AOSC package: {}", query), |m| m.as_str().to_string());

    // Extract section
    let section_regex = Regex::new(r#"<b class="pkg-field">Section</b>:\s*([^<]+)"#).unwrap();
    let section = section_regex
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string());

    // Extract dependencies - only runtime dependencies, not build or library
    let depends_regex = Regex::new(
        r#"<b class="pkg-field pkg-dep-rel">Depends</b>\s*:\s*\n\s*((?:<span class="pkg-dep"><a href="[^"]+">([^<]+)</a></span>,?\s*)+)"#
    ).unwrap();
    let mut depends = Vec::new();
    if let Some(cap) = depends_regex.captures(html) {
        let deps_html = cap.get(1).map_or("", |m| m.as_str());
        let dep_name_regex = Regex::new(r#"<a href="([^"]+)">([^<]+)</a>"#).unwrap();
        for dep_cap in dep_name_regex.captures_iter(deps_html) {
            if let Some(dep_name) = dep_cap.get(1) {
                // Only include actual package names, skip URLs and paths
                let dep_str = dep_name.as_str();
                if
                    !dep_str.starts_with("http") &&
                    !dep_str.starts_with("/") &&
                    !dep_str.contains("github.com")
                {
                    depends.push(dep_str.to_string());
                }
            }
        }
    }

    // Extract upstream URL - try multiple patterns
    let upstream_url = {
        // Try the source link first
        let upstream_regex1 = Regex::new(
            r#"<b class="pkg-field"[^>]*>Upstream</b>:\s*<a href="([^"]+)">source</a>"#
        ).unwrap();
        let url1 = upstream_regex1
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .filter(|url| url.starts_with("http") && url.len() > 8);

        if url1.is_some() {
            url1
        } else {
            // Try the tarball link as fallback
            let upstream_regex2 = Regex::new(
                r#"<a href="([^"]+)"\s*>\(tarball\)[^<]*</a>"#
            ).unwrap();
            upstream_regex2
                .captures(html)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().to_string())
                .filter(|url| url.starts_with("http"))
        }
    };

    // Extract upstream version
    let upstream_ver_regex = Regex::new(r#"<a href="[^"]+"\s*\(git\)\s*([^<]+)</a>"#).unwrap();
    let upstream_version = upstream_ver_regex
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string());

    // Extract architecture information from the package page
    let mut architectures = Vec::new();

    // Pattern 1: Extract architecture and size from text patterns like "amd64: 19.8 MiB"
    let arch_size_regex = Regex::new(r#"([a-z0-9]+):\s+(\d+\.\d+\s+[KMGT]?iB)"#).unwrap();
    for cap in arch_size_regex.captures_iter(html) {
        if let (Some(arch_name), Some(size)) = (cap.get(1), cap.get(2)) {
            let arch = arch_name.as_str().trim();
            let pkg_size = size.as_str().trim();

            // Only include known AOSC architectures
            if
                matches!(
                    arch,
                    "amd64" |
                        "arm64" |
                        "loongarch64" |
                        "loongson3" |
                        "mips64r6el" |
                        "ppc64el" |
                        "riscv64"
                )
            {
                architectures.push(AOSCArchitecture {
                    name: arch.to_string(),
                    version: version.clone(),
                    size: pkg_size.to_string(),
                    download_url: format!(
                        "https://packages.aosc.io/files/{}/stable/{}/{}",
                        arch,
                        query,
                        version
                    ),
                });
            }
        }
    }

    // Pattern 2: Extract from download links with size information
    if architectures.is_empty() {
        let download_regex = Regex::new(
            r#"<a[^>]+href="/files/([a-z0-9]+)/[^"]*"[^>]*>\s*([0-9.]+\s*[KMGT]?iB)\s*</a>"#
        ).unwrap();
        for cap in download_regex.captures_iter(html) {
            if let (Some(arch_name), Some(size)) = (cap.get(1), cap.get(2)) {
                let arch = arch_name.as_str().trim();
                let pkg_size = size.as_str().trim();

                if
                    matches!(
                        arch,
                        "amd64" |
                            "arm64" |
                            "loongarch64" |
                            "loongson3" |
                            "mips64r6el" |
                            "ppc64el" |
                            "riscv64"
                    )
                {
                    architectures.push(AOSCArchitecture {
                        name: arch.to_string(),
                        version: version.clone(),
                        size: pkg_size.to_string(),
                        download_url: format!(
                            "https://packages.aosc.io/files/{}/stable/{}/{}",
                            arch,
                            query,
                            version
                        ),
                    });
                }
            }
        }
    }

    // Pattern 3: Extract architecture names and find corresponding sizes in nearby text
    if architectures.is_empty() {
        let known_archs = [
            "amd64",
            "arm64",
            "loongarch64",
            "loongson3",
            "mips64r6el",
            "ppc64el",
            "riscv64",
        ];
        let mut size_map = std::collections::HashMap::new();

        // Look for size patterns in the HTML text
        let size_context_regex = Regex::new(
            r#"(amd64|arm64|loongarch64|loongson3|mips64r6el|ppc64el|riscv64)[^0-9]*?(\d+\.\d+\s*[KMGT]?iB)"#
        ).unwrap();
        for cap in size_context_regex.captures_iter(html) {
            if let (Some(arch_name), Some(size)) = (cap.get(1), cap.get(2)) {
                let arch = arch_name.as_str().trim();
                let pkg_size = size.as_str().trim();
                size_map.insert(arch.to_string(), pkg_size.to_string());
            }
        }

        // Create architecture entries for each known arch
        for &arch in &known_archs {
            if html.contains(arch) {
                let size = size_map
                    .get(arch)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                architectures.push(AOSCArchitecture {
                    name: arch.to_string(),
                    version: version.clone(),
                    size,
                    download_url: format!(
                        "https://packages.aosc.io/files/{}/stable/{}/{}",
                        arch,
                        query,
                        version
                    ),
                });
            }
        }
    }

    if !architectures.is_empty() || version != "unknown" {
        let package = AOSCPackage {
            name: query.to_string(),
            version,
            description,
            section,
            depends,
            replaces: Vec::new(), // Could be extracted similarly if needed
            breaks: Vec::new(),
            provides: Vec::new(),
            suggests: Vec::new(),
            upstream_url,
            upstream_version,
            architectures,
        };
        packages.push(package);
    }

    Ok(AOSCSearchResponse { packages })
}

fn format_aosc_response(packages: &[AOSCPackage], query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("AOSC Package Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    for (i, package) in packages.iter().enumerate().take(3) {
        if i > 0 {
            output.push('\n');
        }

        output.push_str(&format!("package-name: {}\n", package.name));
        output.push_str(&format!("version: {}\n", package.version));
        output.push_str(&format!("description: {}\n", package.description));

        if let Some(section) = &package.section {
            output.push_str(&format!("section: {}\n", section));
        }

        if !package.depends.is_empty() {
            output.push_str(&format!("depends: {}\n", package.depends.join(", ")));
        }

        if let Some(upstream_url) = &package.upstream_url {
            output.push_str(&format!("upstream: {}\n", upstream_url));
        }

        if let Some(upstream_ver) = &package.upstream_version {
            output.push_str(&format!("upstream-version: {}\n", upstream_ver));
        }

        // Display architectures as a simple list
        if !package.architectures.is_empty() {
            let arch_names: Vec<String> = package.architectures
                .iter()
                .map(|arch| arch.name.clone())
                .collect();
            output.push_str(&format!("architectures: {}\n", arch_names.join(", ")));

            // Display individual architecture details
            for arch in &package.architectures {
                output.push_str(&format!("  {}: {} ({})\n", arch.name, arch.version, arch.size));
            }
        }

        output.push_str(
            &format!("aosc-url: {}{}\n", AOSC_PACKAGES_URL, urlencoding::encode(&package.name))
        );
    }

    output.push_str("repository: AOSC OS\n");
    output.push_str("source: AOSC Packages\n");
    output.push('\n');
    output.push_str("% Information retrieved from AOSC packages\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_aosc_not_found(package_name: &str) -> String {
    format!(
        "AOSC Package Not Found: {}\n\
        No package with this name was found in AOSC repositories.\n\
        \n\
        You can search manually at: {}{}\n\
        \n\
        % Package not found in AOSC repositories\n\
        % Query processed by WHOIS server\n",
        package_name,
        AOSC_SEARCH_URL,
        urlencoding::encode(package_name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aosc_package_name_validation() {
        // Valid package names
        assert!(process_aosc_query("vim").await.is_ok());
        assert!(process_aosc_query("python-3").await.is_ok());
        assert!(process_aosc_query("lib64-dev").await.is_ok());
        assert!(process_aosc_query("package+name").await.is_ok());

        // Invalid package names
        assert!(process_aosc_query("").await.is_err());
        assert!(process_aosc_query("package with spaces").await.is_err());
        assert!(process_aosc_query(&"a".repeat(101)).await.is_err());
    }

    #[tokio::test]
    async fn test_aosc_service_creation() {
        // Test that we can create the service without panicing
        let result = process_aosc_query("nonexistent-package-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("AOSC Package"));
    }
}
