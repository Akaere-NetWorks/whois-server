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

// Re-export package services
pub use aur::process_aur_query;
pub use debian::process_debian_query;
