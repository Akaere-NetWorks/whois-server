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

#[cfg(test)]
mod tests {
    use crate::core::color::{ColorScheme, ColorProtocol, Colorizer};
use crate::core::QueryType;

    #[test]
    fn test_color_scheme_parsing() {
        assert_eq!(ColorScheme::from_string("ripe"), Some(ColorScheme::Ripe));
        assert_eq!(ColorScheme::from_string("RIPE"), Some(ColorScheme::Ripe));
        assert_eq!(ColorScheme::from_string("bgptools"), Some(ColorScheme::BgpTools));
        assert_eq!(ColorScheme::from_string("ripe-dark"), Some(ColorScheme::RipeDark));
        assert_eq!(ColorScheme::from_string("dark-ripe"), Some(ColorScheme::RipeDark));
        assert_eq!(ColorScheme::from_string("bgptools-dark"), Some(ColorScheme::BgpToolsDark));
        assert_eq!(ColorScheme::from_string("dark-bgptools"), Some(ColorScheme::BgpToolsDark));
        assert_eq!(ColorScheme::from_string("invalid"), None);
    }

  
    #[test]
    fn test_protocol_header_parsing() {
        let mut protocol = ColorProtocol::new();

        // Test capability probe
        let probe_request = "X-WHOIS-COLOR-PROBE: 1\r\nexample.com\r\n";
        assert!(protocol.parse_headers(probe_request));
        assert!(protocol.client_supports_color);

        // Test color scheme request
        let mut protocol2 = ColorProtocol::new();
        let color_request = "X-WHOIS-COLOR: ripe\r\nexample.com\r\n";
        assert!(!protocol2.parse_headers(color_request));
        assert!(protocol2.client_supports_color);
        assert_eq!(protocol2.scheme, Some(ColorScheme::Ripe));

        // Test dark color scheme request
        let mut protocol3 = ColorProtocol::new();
        let dark_request = "X-WHOIS-COLOR: ripe-dark\r\nexample.com\r\n";
        assert!(!protocol3.parse_headers(dark_request));
        assert!(protocol3.client_supports_color);
        assert_eq!(protocol3.scheme, Some(ColorScheme::RipeDark));

        // Test scheme format
        let mut protocol4 = ColorProtocol::new();
        let scheme_request = "X-WHOIS-COLOR: scheme=bgptools-dark\r\nexample.com\r\n";
        assert!(!protocol4.parse_headers(scheme_request));
        assert!(protocol4.client_supports_color);
        assert_eq!(protocol4.scheme, Some(ColorScheme::BgpToolsDark));
    }

    #[test]
    fn test_capability_response() {
        let protocol = ColorProtocol::new();
        let response = protocol.get_capability_response();
        assert!(response.contains("ripe"));
        assert!(response.contains("ripe-dark"));
        assert!(response.contains("bgptools"));
        assert!(response.contains("bgptools-dark"));
    }

    #[test]
    fn test_dark_vs_light_coloring() {
        let sample = "% Test\ninetnum: 192.0.2.0 - 192.0.2.255\nnetname: EXAMPLE-NET\ndescr: Example network";

        let ripe_colorizer = Colorizer::new(ColorScheme::Ripe);
        let ripe_dark_colorizer = Colorizer::new(ColorScheme::RipeDark);

        let light_output = ripe_colorizer.colorize_response(sample, &QueryType::IPv4("192.0.2.0".parse().unwrap()));
        let dark_output = ripe_dark_colorizer.colorize_response(sample, &QueryType::IPv4("192.0.2.0".parse().unwrap()));

        // Colors should be different between light and dark modes
        assert_ne!(light_output, dark_output);

        // Dark mode should use dim white for comments, light mode should use bright black
        assert!(light_output.contains("\x1b[90m")); // Bright black
        assert!(dark_output.contains("\x1b[37m")); // Dim white
    }

    #[test]
    fn test_bgptools_dark_coloring() {
        let sample = "% Test\norigin: AS64544\nroute: 192.0.2.0/24";

        let bgptools_colorizer = Colorizer::new(ColorScheme::BgpTools);
        let bgptools_dark_colorizer = Colorizer::new(ColorScheme::BgpToolsDark);

        let light_output = bgptools_colorizer.colorize_response(sample, &QueryType::BGPTool("192.0.2.0/24".to_string()));
        let dark_output = bgptools_dark_colorizer.colorize_response(sample, &QueryType::BGPTool("192.0.2.0/24".to_string()));

        // Colors should be different between light and dark modes
        assert_ne!(light_output, dark_output);
    }
}