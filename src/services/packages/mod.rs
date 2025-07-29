// Package management services for various Linux distributions
// Copyright (C) 2024 Akaere Networks
// 
// This file is part of the WHOIS server.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

pub mod aur;
pub mod debian;

use anyhow::Result;
use tracing::debug;

// Re-export package services
pub use aur::process_aur_query;
pub use debian::process_debian_query;

/// Package manager types supported by the system
#[derive(Debug, Clone)]
pub enum PackageManager {
    Aur,        // Arch User Repository
    Debian,     // Debian/Ubuntu APT packages
    // Future additions:
    // Fedora,     // Fedora DNF/YUM packages
    // Alpine,     // Alpine APK packages
    // Gentoo,     // Gentoo Portage
    // Nixpkgs,    // NixOS packages
    // Homebrew,   // macOS Homebrew
    // Chocolatey, // Windows Chocolatey
}

impl PackageManager {
    /// Get package manager from query suffix
    pub fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix.to_uppercase().as_str() {
            "AUR" => Some(PackageManager::Aur),
            "DEBIAN" => Some(PackageManager::Debian),
            // Future additions will go here
            _ => None,
        }
    }
    
    /// Get the display name for the package manager
    pub fn display_name(&self) -> &'static str {
        match self {
            PackageManager::Aur => "Arch User Repository (AUR)",
            PackageManager::Debian => "Debian Package Repository",
        }
    }
    
    /// Get the official website for the package manager
    pub fn website(&self) -> &'static str {
        match self {
            PackageManager::Aur => "https://aur.archlinux.org/",
            PackageManager::Debian => "https://packages.debian.org/",
        }
    }
}

/// Process package query based on package manager type
pub async fn process_package_query(package_name: &str, pm: PackageManager) -> Result<String> {
    debug!("Processing {} package query for: {}", pm.display_name(), package_name);
    
    match pm {
        PackageManager::Aur => process_aur_query(package_name).await,
        PackageManager::Debian => process_debian_query(package_name).await,
    }
}

/// Unified package not found response
pub fn format_package_not_found(package_name: &str, pm: &PackageManager) -> String {
    format!(
        "% Package '{}' not found in {}\n\
         % \n\
         % Search suggestions:\n\
         % - Check package name spelling\n\
         % - Try searching on: {}\n\
         % - Package might be in another repository\n\
         % \n\
         % Package Manager Information:\n\
         % Name: {}\n\
         % Website: {}\n\
         ",
        package_name, 
        pm.display_name(), 
        pm.website(),
        pm.display_name(),
        pm.website()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_from_suffix() {
        assert!(matches!(PackageManager::from_suffix("AUR"), Some(PackageManager::Aur)));
        assert!(matches!(PackageManager::from_suffix("aur"), Some(PackageManager::Aur)));
        assert!(matches!(PackageManager::from_suffix("DEBIAN"), Some(PackageManager::Debian)));
        assert!(matches!(PackageManager::from_suffix("debian"), Some(PackageManager::Debian)));
        assert!(PackageManager::from_suffix("UNKNOWN").is_none());
    }

    #[test]
    fn test_package_manager_display_names() {
        assert_eq!(PackageManager::Aur.display_name(), "Arch User Repository (AUR)");
        assert_eq!(PackageManager::Debian.display_name(), "Debian Package Repository");
    }

    #[test]
    fn test_package_manager_websites() {
        assert_eq!(PackageManager::Aur.website(), "https://aur.archlinux.org/");
        assert_eq!(PackageManager::Debian.website(), "https://packages.debian.org/");
    }
}