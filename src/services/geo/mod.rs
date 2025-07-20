// Sub-modules
pub mod types;
pub mod constants;
pub mod utils;
pub mod ripe_api;
pub mod ipinfo_api;
pub mod formatters;
pub mod services;

// Re-export public API
pub use services::{
    process_geo_query,
    process_geo_query_blocking,
    process_rir_geo_query,
    process_rir_geo_query_blocking,
    process_prefixes_query,
    process_prefixes_query_blocking,
};

#[cfg(test)]
mod tests {
    use super::formatters::format_rir_geo_response;
    use super::types::{RirGeoResponse, RirGeoData, RirGeoResource, RirGeoParameters};

    #[test]
    fn test_format_rir_geo_response_empty() {
        let response = RirGeoResponse {
            data: None,
            status: "ok".to_string(),
            messages: None,
            see_also: None,
            version: "1.0".to_string(),
            data_call_name: "rir-geo".to_string(),
            data_call_status: "supported".to_string(),
            cached: false,
            query_id: "test".to_string(),
            process_time: 41,
            server_id: "test".to_string(),
            build_version: "test".to_string(),
            status_code: 200,
            time: "2025-06-08T18:05:15.809098".to_string(),
        };
        
        let formatted = format_rir_geo_response("2001:67c:2e8::/48", &response).unwrap();
        assert!(formatted.contains("% RIPE NCC STAT RIR Geographic Query"));
        assert!(formatted.contains("% Query: 2001:67c:2e8::/48"));
        assert!(formatted.contains("% No RIR geographic data available"));
    }
    
    #[test]
    fn test_format_rir_geo_response_with_data() {
        let response = RirGeoResponse {
            data: Some(RirGeoData {
                located_resources: Some(vec![
                    RirGeoResource {
                        resource: "2001:67c:2e8::/48".to_string(),
                        location: "NL".to_string(),
                    }
                ]),
                result_time: "2025-06-07T00:00:00".to_string(),
                parameters: RirGeoParameters {
                    resource: "2001:67c:2e8::/48".to_string(),
                    query_time: "2025-06-07T00:00:00".to_string(),
                    cache: None,
                },
                earliest_time: "2005-02-18T00:00:00".to_string(),
                latest_time: "2025-06-07T00:00:00".to_string(),
            }),
            status: "ok".to_string(),
            messages: None,
            see_also: None,
            version: "1.0".to_string(),
            data_call_name: "rir-geo".to_string(),
            data_call_status: "supported".to_string(),
            cached: false,
            query_id: "test".to_string(),
            process_time: 41,
            server_id: "test".to_string(),
            build_version: "test".to_string(),
            status_code: 200,
            time: "2025-06-08T18:05:15.809098".to_string(),
        };
        
        let formatted = format_rir_geo_response("2001:67c:2e8::/48", &response).unwrap();
        assert!(formatted.contains("% RIPE NCC STAT RIR Geographic Query"));
        assert!(formatted.contains("% Query: 2001:67c:2e8::/48"));
        assert!(formatted.contains("RIR Geographic Location Results"));
        assert!(formatted.contains("2001:67c:2e8::/48"));
        assert!(formatted.contains("NL"));
        assert!(formatted.contains("% Total located resources: 1"));
    }
} 