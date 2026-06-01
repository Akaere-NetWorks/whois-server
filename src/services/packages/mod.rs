// Package management services for various Linux distributions
// Copyright (C) 2024 Akaere Networks
//
// This file is part of the WHOIS server.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#[cfg(feature = "pkg-alma")]
pub mod alma;
#[cfg(feature = "pkg-aosc")]
pub mod aosc;
#[cfg(feature = "pkg-aur")]
pub mod aur;
#[cfg(feature = "pkg-cargo")]
pub mod cargo;
#[cfg(feature = "pkg-curseforge")]
pub mod curseforge;
#[cfg(feature = "pkg-debian")]
pub mod debian;
#[cfg(feature = "pkg-epel")]
pub mod epel;
#[cfg(feature = "pkg-modrinth")]
pub mod modrinth;
#[cfg(feature = "pkg-nixos")]
pub mod nixos;
#[cfg(feature = "pkg-npm")]
pub mod npm;
#[cfg(feature = "pkg-opensuse")]
pub mod opensuse;
#[cfg(feature = "pkg-openwrt")]
pub mod openwrt;
#[cfg(feature = "pkg-pypi")]
pub mod pypi;
#[cfg(feature = "pkg-ubuntu")]
pub mod ubuntu;

// Re-export package services
#[cfg(feature = "pkg-alma")]
pub use alma::process_alma_query;
#[cfg(feature = "pkg-aosc")]
pub use aosc::process_aosc_query;
#[cfg(feature = "pkg-aur")]
pub use aur::process_aur_query;
#[cfg(feature = "pkg-cargo")]
pub use cargo::process_cargo_query;
#[cfg(feature = "pkg-curseforge")]
pub use curseforge::query_curseforge;
#[cfg(feature = "pkg-debian")]
pub use debian::process_debian_query;
#[cfg(feature = "pkg-epel")]
pub use epel::process_epel_query;
#[cfg(feature = "pkg-modrinth")]
pub use modrinth::query_modrinth;
#[cfg(feature = "pkg-nixos")]
pub use nixos::process_nixos_query;
#[cfg(feature = "pkg-npm")]
pub use npm::process_npm_query;
#[cfg(feature = "pkg-opensuse")]
pub use opensuse::process_opensuse_query;
#[cfg(feature = "pkg-openwrt")]
pub use openwrt::process_openwrt_query;
#[cfg(feature = "pkg-pypi")]
pub use pypi::process_pypi_query;
#[cfg(feature = "pkg-ubuntu")]
pub use ubuntu::process_ubuntu_query;
