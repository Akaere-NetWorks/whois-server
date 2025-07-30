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
use tracing::{debug, error};

const UBUNTU_PACKAGES_API: &str = "https://api.launchpad.net/1.0/ubuntu/+archive/primary";
const UBUNTU_PACKAGES_SEARCH: &str = "https://packages.ubuntu.com";

#[derive(Debug, Deserialize, Serialize)]
struct UbuntuPackageInfo {
    binary_package_name: String,
    binary_package_version: Option<String>,
    component_name: Option<String>,
    source_package_name: Option<String>,
    source_package_version: Option<String>,
    architecture_specific: Option<bool>,
    section_name: Option<String>,
    priority_name: Option<String>,
    status: Option<String>,
    date_published: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UbuntuSearchResult {
    entries: Option<Vec<UbuntuPackageInfo>>,
    total_size: Option<u32>,
}

pub async fn process_ubuntu_query(package_name: &str) -> Result<String> {
    debug!("Processing Ubuntu query for package: {}", package_name);
    
    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }
    
    // Validate package name (Ubuntu follows Debian naming conventions)
    if package_name.len() > 100 || package_name.contains(' ') || 
       !package_name.chars().all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c)) {
        return Err(anyhow::anyhow!("Invalid Ubuntu package name format"));
    }
    
    match query_ubuntu_packages(package_name).await {
        Ok(search_result) => {
            if let Some(packages) = search_result.entries {
                if !packages.is_empty() {
                    Ok(format_ubuntu_response(&packages, package_name))
                } else {
                    Ok(format_ubuntu_not_found(package_name))
                }
            } else {
                Ok(format_ubuntu_not_found(package_name))
            }
        },
        Err(e) => {
            error!("Ubuntu packages query failed for {}: {}", package_name, e);
            Ok(format_ubuntu_not_found(package_name))
        }
    }
}

async fn query_ubuntu_packages(package_name: &str) -> Result<UbuntuSearchResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
        .build()
        .context("Failed to create HTTP client")?;

    // Use Launchpad API to search for packages
    let search_url = format!("{}?ws.op=getPublishedBinaries&binary_name={}&ws.size=5", 
                           UBUNTU_PACKAGES_API, package_name);
    
    debug!("Querying Ubuntu packages API: {}", search_url);
    
    let response = client
        .get(&search_url)
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to send request to Ubuntu packages API")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Ubuntu packages API returned status: {}", response.status()));
    }

    let search_result: UbuntuSearchResult = response
        .json()
        .await
        .context("Failed to parse Ubuntu packages API response")?;

    Ok(search_result)
}

fn format_ubuntu_response(packages: &[UbuntuPackageInfo], query: &str) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Ubuntu Package Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    for (i, package) in packages.iter().enumerate().take(3) {
        if i > 0 {
            output.push('\n');
        }
        
        output.push_str(&format!("package-name: {}\n", package.binary_package_name));
        
        if let Some(version) = &package.binary_package_version {
            output.push_str(&format!("version: {}\n", version));
        }
        
        if let Some(component) = &package.component_name {
            output.push_str(&format!("component: {}\n", component));
        }
        
        if let Some(source_name) = &package.source_package_name {
            output.push_str(&format!("source-package: {}\n", source_name));
        }
        
        if let Some(source_version) = &package.source_package_version {
            output.push_str(&format!("source-version: {}\n", source_version));
        }
        
        if let Some(section) = &package.section_name {
            output.push_str(&format!("section: {}\n", section));
        }
        
        if let Some(priority) = &package.priority_name {
            output.push_str(&format!("priority: {}\n", priority));
        }
        
        if let Some(arch_specific) = package.architecture_specific {
            output.push_str(&format!("architecture-specific: {}\n", 
                           if arch_specific { "yes" } else { "no" }));
        }
        
        if let Some(status) = &package.status {
            output.push_str(&format!("status: {}\n", status));
        }
        
        if let Some(date) = &package.date_published {
            output.push_str(&format!("date-published: {}\n", date));
        }
        
        output.push_str(&format!("ubuntu-url: {}/search?keywords={}\n", 
                               UBUNTU_PACKAGES_SEARCH, package.binary_package_name));
    }

    output.push_str(&format!("repository: Ubuntu\n"));
    output.push_str(&format!("source: Launchpad API\n"));
    output.push('\n');
    output.push_str("% Information retrieved from Ubuntu packages\n");
    output.push_str("% Query processed by WHOIS server\n");
    
    output
}

fn format_ubuntu_not_found(package_name: &str) -> String {
    format!(
        "Ubuntu Package Not Found: {}\n\
        No package with this name was found in Ubuntu repositories.\n\
        \n\
        You can search manually at: {}/search?keywords={}\n\
        \n\
        % Package not found in Ubuntu repositories\n\
        % Query processed by WHOIS server\n",
        package_name, UBUNTU_PACKAGES_SEARCH, package_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubuntu_package_name_validation() {
        // Valid package names
        assert!(process_ubuntu_query("vim").is_ok());
        assert!(process_ubuntu_query("python3-pip").is_ok()); 
        assert!(process_ubuntu_query("lib64-dev").is_ok());
        assert!(process_ubuntu_query("package+name").is_ok());
        
        // Invalid package names
        assert!(process_ubuntu_query("").is_err());
        assert!(process_ubuntu_query("package with spaces").is_err());
        assert!(process_ubuntu_query(&"a".repeat(101)).is_err());
    }

    #[tokio::test]
    async fn test_ubuntu_service_creation() {
        // Test that we can create the service without panicing
        let result = process_ubuntu_query("nonexistent-package-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Ubuntu Package"));
    }
}