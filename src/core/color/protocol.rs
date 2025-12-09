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

use crate::core::color::scheme::ColorScheme;

#[derive(Debug, Clone)]
pub struct ColorProtocol {
    pub enabled: bool,
    pub scheme: Option<ColorScheme>,
    pub client_supports_color: bool,
}

impl Default for ColorProtocol {
    fn default() -> Self {
        Self {
            enabled: true,
            scheme: None,
            client_supports_color: false,
        }
    }
}

impl ColorProtocol {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_headers(&mut self, request: &str) -> bool {
        let lines: Vec<&str> = request.lines().collect();

        for line in &lines {
            let line = line.trim();

            // Check for capability probe
            if line.to_uppercase().starts_with("X-WHOIS-COLOR-PROBE:") {
                self.client_supports_color = true;
                return true; // This is a capability probe request
            }

            // Check for color scheme request
            if line.to_uppercase().starts_with("X-WHOIS-COLOR:")
                && let Some(value_part) = line.split(':').nth(1)
            {
                let value_part = value_part.trim();

                // Support both formats: "ripe", "ripe-dark", "scheme=ripe", etc.
                let scheme_str = if value_part.starts_with("scheme=") {
                    &value_part[7..] // Remove "scheme=" prefix
                } else {
                    value_part
                };

                if let Some(scheme) = ColorScheme::from_string(scheme_str) {
                    self.scheme = Some(scheme);
                    self.client_supports_color = true;
                }
            }
        }

        false // Not a capability probe
    }

    pub fn should_colorize(&self) -> bool {
        self.enabled && self.client_supports_color && self.scheme.is_some()
    }

    pub fn get_capability_response(&self) -> String {
        if self.enabled {
            "X-WHOIS-COLOR-SUPPORT: 1.0 schemes=ripe,ripe-dark,bgptools,bgptools-dark\r\n\r\n".to_string()
        } else {
            "X-WHOIS-COLOR-SUPPORT: no\r\n\r\n".to_string()
        }
    }
}