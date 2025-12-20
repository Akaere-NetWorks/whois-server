// AUR (Arch User Repository) package lookup service
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
const AUR_API_BASE: &str = "https://aur.archlinux.org/rpc/v5/info";
const AUR_PACKAGE_BASE: &str = "https://aur.archlinux.org/packages";

#[derive(Debug, Deserialize, Serialize)]
struct AurResponse {
    version: u32,
    #[serde(rename = "type")]
    response_type: String,
    resultcount: u32,
    results: Vec<AurPackage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AurPackage {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "PackageBaseID")]
    package_base_id: Option<u32>,
    #[serde(rename = "PackageBase")]
    package_base: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "NumVotes")]
    num_votes: u32,
    #[serde(rename = "Popularity")]
    popularity: f64,
    #[serde(rename = "OutOfDate")]
    out_of_date: Option<u64>,
    #[serde(rename = "Maintainer")]
    maintainer: Option<String>,
    #[serde(rename = "FirstSubmitted")]
    first_submitted: u64,
    #[serde(rename = "LastModified")]
    last_modified: u64,
    #[serde(rename = "URLPath")]
    url_path: String,
    #[serde(rename = "Depends")]
    depends: Option<Vec<String>>,
    #[serde(rename = "MakeDepends")]
    make_depends: Option<Vec<String>>,
    #[serde(rename = "OptDepends")]
    opt_depends: Option<Vec<String>>,
    #[serde(rename = "CheckDepends")]
    check_depends: Option<Vec<String>>,
    #[serde(rename = "Conflicts")]
    conflicts: Option<Vec<String>>,
    #[serde(rename = "Provides")]
    provides: Option<Vec<String>>,
    #[serde(rename = "Replaces")]
    replaces: Option<Vec<String>>,
    #[serde(rename = "Groups")]
    groups: Option<Vec<String>>,
    #[serde(rename = "License")]
    license: Option<Vec<String>>,
    #[serde(rename = "Keywords")]
    keywords: Option<Vec<String>>,
}

pub async fn process_aur_query(package_name: &str) -> Result<String> {
    log_debug!("Processing AUR query for package: {}", package_name);

    if package_name.is_empty() {
        return Err(anyhow::anyhow!("Package name cannot be empty"));
    }

    // Validate package name (AUR package names should be reasonable)
    if package_name.len() > 100 || package_name.contains(' ') {
        return Err(anyhow::anyhow!("Invalid package name format"));
    }

    match query_aur_api(package_name).await {
        Ok(package) => Ok(format_aur_response(&package, package_name)),
        Err(e) => {
            log_error!("AUR API query failed for {}: {}", package_name, e);
            Ok(format_aur_not_found(package_name))
        }
    }
}

async fn query_aur_api(package_name: &str) -> Result<AurPackage> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("whois-server/1.0 (AUR package lookup)")
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!("{}?arg={}", AUR_API_BASE, package_name);
    log_debug!("Querying AUR API: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send AUR API request")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "AUR API returned status: {}",
            response.status()
        ));
    }

    let aur_response: AurResponse = response
        .json()
        .await
        .context("Failed to parse AUR API response")?;

    log_debug!(
        "AUR API response: {} results for {}",
        aur_response.resultcount, package_name
    );

    if aur_response.resultcount == 0 {
        return Err(anyhow::anyhow!("Package not found in AUR"));
    }

    // Return the first result (should be exact match)
    aur_response
        .results
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No package data in AUR response"))
}

fn format_aur_response(package: &AurPackage, _query: &str) -> String {
    let mut response = String::new();

    // Format as WHOIS-style response
    response.push_str(&format!("package: {}\n", package.name));
    response.push_str(&format!("package-base: {}\n", package.package_base));
    response.push_str(&format!("version: {}\n", package.version));

    if let Some(description) = &package.description {
        response.push_str(&format!("description: {}\n", description));
    }

    if let Some(url) = &package.url {
        response.push_str(&format!("upstream-url: {}\n", url));
    }

    // AUR-specific information
    response.push_str(&format!("aur-url: {}/{}\n", AUR_PACKAGE_BASE, package.name));
    response.push_str(&format!("aur-id: {}\n", package.id));
    response.push_str(&format!("votes: {}\n", package.num_votes));
    response.push_str(&format!("popularity: {:.6}\n", package.popularity));

    if let Some(maintainer) = &package.maintainer {
        response.push_str(&format!("maintainer: {}\n", maintainer));
    } else {
        response.push_str("maintainer: orphaned\n");
    }

    // Dates
    let first_submitted = format_timestamp(package.first_submitted);
    let last_modified = format_timestamp(package.last_modified);
    response.push_str(&format!("first-submitted: {}\n", first_submitted));
    response.push_str(&format!("last-modified: {}\n", last_modified));

    // Out of date status
    if let Some(ood_timestamp) = package.out_of_date {
        let ood_date = format_timestamp(ood_timestamp);
        response.push_str(&format!("out-of-date: {}\n", ood_date));
    } else {
        response.push_str("out-of-date: no\n");
    }

    // Dependencies
    if let Some(depends) = &package.depends
        && !depends.is_empty()
    {
        response.push_str(&format!("depends: {}\n", depends.join(", ")));
    }

    if let Some(make_depends) = &package.make_depends
        && !make_depends.is_empty()
    {
        response.push_str(&format!("makedepends: {}\n", make_depends.join(", ")));
    }

    if let Some(opt_depends) = &package.opt_depends
        && !opt_depends.is_empty()
    {
        response.push_str(&format!("optdepends: {}\n", opt_depends.join(", ")));
    }

    if let Some(check_depends) = &package.check_depends
        && !check_depends.is_empty()
    {
        response.push_str(&format!("checkdepends: {}\n", check_depends.join(", ")));
    }

    // Conflicts, provides, replaces
    if let Some(conflicts) = &package.conflicts
        && !conflicts.is_empty()
    {
        response.push_str(&format!("conflicts: {}\n", conflicts.join(", ")));
    }

    if let Some(provides) = &package.provides
        && !provides.is_empty()
    {
        response.push_str(&format!("provides: {}\n", provides.join(", ")));
    }

    if let Some(replaces) = &package.replaces
        && !replaces.is_empty()
    {
        response.push_str(&format!("replaces: {}\n", replaces.join(", ")));
    }

    // Groups and licenses
    if let Some(groups) = &package.groups
        && !groups.is_empty()
    {
        response.push_str(&format!("groups: {}\n", groups.join(", ")));
    }

    if let Some(license) = &package.license
        && !license.is_empty()
    {
        response.push_str(&format!("license: {}\n", license.join(", ")));
    }

    // Keywords
    if let Some(keywords) = &package.keywords
        && !keywords.is_empty()
    {
        response.push_str(&format!("keywords: {}\n", keywords.join(", ")));
    }

    // Source and additional info
    response.push('\n');
    response.push_str("% Additional Information:\n");
    response.push_str(&format!(
        "% AUR Package URL: {}/{}\n",
        AUR_PACKAGE_BASE, package.name
    ));
    response.push_str(&format!(
        "% AUR Git Clone: https://aur.archlinux.org/{}.git\n",
        package.name
    ));
    response.push_str(&format!(
        "% Install with: yay -S {} or paru -S {}\n",
        package.name, package.name
    ));
    response.push_str("% Source: Arch User Repository (AUR)\n");

    response
}

fn format_aur_not_found(package_name: &str) -> String {
    format!(
        "% Package '{}' not found in AUR\n\
         % \n\
         % Search suggestions:\n\
         % - Check package name spelling\n\
         % - Try searching on: https://aur.archlinux.org/packages/?K={}\n\
         % - Package might be in official Arch repositories\n\
         % \n\
         % AUR Information:\n\
         % AUR URL: https://aur.archlinux.org/\n\
         % AUR Guidelines: https://wiki.archlinux.org/title/AUR\n\
         ",
        package_name, package_name
    )
}

fn format_timestamp(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(_now) => {
            let target_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp);
            match target_time.duration_since(UNIX_EPOCH) {
                Ok(duration) => {
                    // Simple format: YYYY-MM-DD HH:MM:SS UTC
                    let secs = duration.as_secs();
                    let days_since_epoch = secs / 86400;
                    let remaining_secs = secs % 86400;
                    let hours = remaining_secs / 3600;
                    let minutes = (remaining_secs % 3600) / 60;
                    let seconds = remaining_secs % 60;

                    // Calculate approximate date (simplified)
                    let year = 1970 + days_since_epoch / 365;
                    let day_of_year = days_since_epoch % 365;
                    let month = day_of_year / 30 + 1;
                    let day = (day_of_year % 30) + 1;

                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
                        year,
                        month.min(12),
                        day.min(31),
                        hours,
                        minutes,
                        seconds
                    )
                }
                Err(_) => format!("{} (timestamp)", timestamp),
            }
        }
        Err(_) => format!("{} (timestamp)", timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        // Test with a known timestamp (2024-01-01 00:00:00 UTC = 1704067200)
        let formatted = format_timestamp(1704067200);
        assert!(formatted.contains("2024"));
        assert!(formatted.contains("UTC"));
    }

    #[test]
    fn test_format_not_found() {
        let result = format_aur_not_found("nonexistent-package");
        assert!(result.contains("not found"));
        assert!(result.contains("nonexistent-package"));
        assert!(result.contains("AUR URL"));
    }
}
