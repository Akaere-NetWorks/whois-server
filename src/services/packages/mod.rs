// Package management services for various Linux distributions
// Copyright (C) 2024 Akaere Networks
// 
// This file is part of the WHOIS server.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

pub mod aosc;
pub mod aur;
pub mod debian;
pub mod nixos;
pub mod npm;
pub mod opensuse;
pub mod pypi;
pub mod ubuntu;

// Re-export package services
pub use aosc::process_aosc_query;
pub use aur::process_aur_query;
pub use debian::process_debian_query;
pub use nixos::process_nixos_query;
pub use npm::process_npm_query;
pub use opensuse::process_opensuse_query;
pub use pypi::process_pypi_query;
pub use ubuntu::process_ubuntu_query;
