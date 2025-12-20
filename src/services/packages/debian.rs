// Debian/Ubuntu package lookup service
// Copyright (C) 2024 Akaere Networks
//
// This file is part of the WHOIS server.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use anyhow::{Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use crate::{log_debug, log_error};
const DEBIAN_API_BASE: &str = "https://sources.debian.org/api/src";
const DEBIAN_PACKAGES_BASE: &str = "https://packages.debian.org";
const UBUNTU_PACKAGES_BASE: &str = "https://packages.ubuntu.com";

#[derive(Debug, Deserialize, Serialize)]
struct DebianPackageResponse {
    package: String,
    versions: Vec<DebianVersion>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DebianVersion {
    version: String,
    suites: Vec<String>,
    area: Option<String>,
    binaries: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct DebianPackageInfo {
    package: String,
    version: String,
    architecture: String,
    maintainer: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    section: Option<String>,
    priority: Option<String>,
    depends: Option<String>,
    recommends: Option<String>,
    suggests: Option<String>,
    conflicts: Option<String>,
    replaces: Option<String>,
    provides: Option<String>,
    size: Option<u64>,
    installed_size: Option<u64>,
}

pub async fn process_debian_query(package_name: &str) -> Result<String> {
    log_debug!("Processing Debian query for package: {}", package_name);

    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }

    // Validate package name (Debian package names should follow specific rules)
    if package_name.len() > 100
        || package_name.contains(' ')
        || !package_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c))
    {
        return Err(anyhow::anyhow!("Invalid Debian package name format"));
    }

    match query_debian_api(package_name).await {
        Ok(package_info) => Ok(format_debian_response(&package_info, package_name)),
        Err(e) => {
            log_error!("Debian API query failed for {}: {}", package_name, e);
            Ok(format_debian_not_found(package_name))
        }
    }
}

async fn query_debian_api(package_name: &str) -> Result<DebianPackageResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("whois-server/1.0 (Debian package lookup)")
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!("{}/{}/", DEBIAN_API_BASE, package_name);
    log_debug!("Querying Debian API: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send Debian API request")?;

    if !response.status().is_success() {
        if response.status() == 404 {
            return Err(anyhow::anyhow!("Package not found in Debian repository"));
        }
        return Err(anyhow::anyhow!(
            "Debian API returned status: {}",
            response.status()
        ));
    }

    let package_response: DebianPackageResponse = response
        .json()
        .await
        .context("Failed to parse Debian API response")?;

    log_debug!(
        "Debian API response: {} versions for {}",
        package_response.versions.len(),
        package_name
    );

    if package_response.versions.is_empty() {
        return Err(anyhow::anyhow!("No versions found for package"));
    }

    Ok(package_response)
}

fn format_debian_response(package: &DebianPackageResponse, _query: &str) -> String {
    let mut response = String::new();

    // Get the latest version (first in the list is usually the newest)
    let latest_version = &package.versions[0];

    // Format as WHOIS-style response
    response.push_str(&format!("package: {}\n", package.package));
    response.push_str(&format!("version: {}\n", latest_version.version));

    // Suites (Debian releases)
    if !latest_version.suites.is_empty() {
        response.push_str(&format!("suites: {}\n", latest_version.suites.join(", ")));
    }

    // Area (main, contrib, non-free, etc.)
    if let Some(area) = &latest_version.area {
        response.push_str(&format!("area: {}\n", area));
    }

    // Binary packages built from this source
    if let Some(binaries) = &latest_version.binaries
        && !binaries.is_empty()
    {
        response.push_str(&format!("binary-packages: {}\n", binaries.join(", ")));
    }

    // Package repository information
    response.push_str("repository: Debian Source Repository\n");
    response.push_str("package-format: deb\n");

    // All available versions
    response.push_str("\n% Available Versions:\n");
    for (index, version) in package.versions.iter().enumerate() {
        if index >= 5 {
            // Limit to first 5 versions to avoid too much output
            response.push_str(&format!(
                "% ... and {} more versions\n",
                package.versions.len() - 5
            ));
            break;
        }
        response.push_str(&format!(
            "% {}: {} ({})\n",
            version.version,
            version.suites.join(", "),
            version.area.as_deref().unwrap_or("unknown")
        ));
    }

    // Installation instructions
    response.push_str("\n% Installation Instructions:\n");
    response.push_str(&format!("% apt install {}\n", package.package));
    response.push_str(&format!("% apt-get install {}\n", package.package));

    // Package information URLs
    response.push_str("\n% Additional Information:\n");
    response.push_str(&format!(
        "% Debian Package: {}/search?keywords={}\n",
        DEBIAN_PACKAGES_BASE, package.package
    ));
    response.push_str(&format!(
        "% Ubuntu Package: {}/search?keywords={}\n",
        UBUNTU_PACKAGES_BASE, package.package
    ));
    response.push_str(&format!(
        "% Source Code: https://sources.debian.org/src/{}/\n",
        package.package
    ));
    response.push_str(&format!(
        "% Bug Reports: https://bugs.debian.org/{}\n",
        package.package
    ));
    response.push_str(&format!(
        "% Package Tracker: https://tracker.debian.org/pkg/{}\n",
        package.package
    ));

    response
}

fn format_debian_not_found(package_name: &str) -> String {
    format!(
        "% Package '{}' not found in Debian repository\n\
         % \n\
         % Search suggestions:\n\
         % - Check package name spelling\n\
         % - Try searching on: {}/search?keywords={}\n\
         % - Try Ubuntu packages: {}/search?keywords={}\n\
         % - Package might be named differently\n\
         % - Package might be in a PPA or third-party repository\n\
         % \n\
         % Common Debian package commands:\n\
         % apt search {}\n\
         % apt-cache search {}\n\
         % apt show {}\n\
         % \n\
         % Repository Information:\n\
         % Debian Packages: {}\n\
         % Ubuntu Packages: {}\n\
         % Source Browser: https://sources.debian.org/\n\
         ",
        package_name,
        DEBIAN_PACKAGES_BASE,
        package_name,
        UBUNTU_PACKAGES_BASE,
        package_name,
        package_name,
        package_name,
        package_name,
        DEBIAN_PACKAGES_BASE,
        UBUNTU_PACKAGES_BASE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_not_found() {
        let result = format_debian_not_found("nonexistent-package");
        assert!(result.contains("not found"));
        assert!(result.contains("nonexistent-package"));
        assert!(result.contains("Debian Packages"));
        assert!(result.contains("apt search"));
    }
}
