use anyhow::Result;
use tracing::debug;

use crate::services::query_with_iana_referral;
use crate::dn42::process_dn42_query_managed;
use crate::core::{ analyze_query, QueryType };

/// Process description-only queries ending with -DESC
pub async fn process_desc_query(base_query: &str) -> Result<String> {
    debug!("Processing description query for: {}", base_query);

    // Determine what type of query this is without the -DESC suffix
    let query_type = analyze_query(base_query);
    
    // Get the raw WHOIS response based on query type
    let raw_response = match query_type {
        QueryType::Domain(_) | QueryType::IPv4(_) | QueryType::IPv6(_) | QueryType::ASN(_) => {
            // Try public WHOIS first
            match query_with_iana_referral(base_query).await {
                Ok(response) if !response.trim().is_empty() && 
                               !response.contains("No entries found") && 
                               !response.contains("Not found") => {
                    response
                }
                _ => {
                    // Fall back to DN42 if public query fails
                    debug!("Public query failed or returned no results, trying DN42 for: {}", base_query);
                    process_dn42_query_managed(base_query).await?
                }
            }
        }
        QueryType::Unknown(_) => {
            // For unknown types, try public first, then DN42
            match query_with_iana_referral(base_query).await {
                Ok(response) if !response.trim().is_empty() && 
                               !response.contains("No entries found") && 
                               !response.contains("Not found") => {
                    response
                }
                _ => {
                    // Fall back to DN42
                    debug!("Public query failed, trying DN42 for: {}", base_query);
                    process_dn42_query_managed(base_query).await?
                }
            }
        }
        _ => {
            // For other query types, just query as unknown
            query_with_iana_referral(base_query).await?
        }
    };

    debug!("Raw response length: {} chars", raw_response.len());

    // Extract descr and remarks fields from the response while preserving order
    let desc_remarks = extract_desc_and_remarks(&raw_response);
    
    // Format the response
    format_desc_response(base_query, &desc_remarks)
}

/// Represents a description or remarks field with its type and value
#[derive(Debug, Clone)]
struct DescField {
    field_type: String,
    value: String,
}

/// Extract description and remarks fields from WHOIS response while preserving order
fn extract_desc_and_remarks(response: &str) -> Vec<DescField> {
    let mut fields = Vec::new();

    for line in response.lines() {
        let line = line.trim();

        // Look for descr field (case-insensitive)
        if let Some(desc) = extract_field_value(line, "descr") {
            debug!("Found descr: {}", desc);
            fields.push(DescField {
                field_type: "descr".to_string(),
                value: desc,
            });
        }
        // Look for remarks field (case-insensitive)
        else if let Some(remark) = extract_field_value(line, "remarks") {
            debug!("Found remarks: {}", remark);
            fields.push(DescField {
                field_type: "remarks".to_string(),
                value: remark,
            });
        }
        // Also check for "description" field (some registries use this)
        else if let Some(desc) = extract_field_value(line, "description") {
            debug!("Found description: {}", desc);
            fields.push(DescField {
                field_type: "description".to_string(),
                value: desc,
            });
        }
    }

    fields
}

/// Extract value from a WHOIS field line (case-insensitive)
fn extract_field_value(line: &str, field_name: &str) -> Option<String> {
    let line_lower = line.to_lowercase();
    let field_lower = field_name.to_lowercase();
    
    if line_lower.starts_with(&field_lower) {
        // Find the colon
        if let Some(colon_pos) = line.find(':') {
            let value = line[colon_pos + 1..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Format description response
fn format_desc_response(query: &str, fields: &[DescField]) -> Result<String> {
    if fields.is_empty() {
        return Ok(format!(
            "% Description Query Results for: {}\n% No description or remarks fields found\n",
            query
        ));
    }

    let mut response = format!("% Description Query Results for: {}\n", query);
    
    // Count by field type
    let descr_count = fields.iter().filter(|f| f.field_type == "descr" || f.field_type == "description").count();
    let remarks_count = fields.iter().filter(|f| f.field_type == "remarks").count();
    
    if descr_count > 0 && remarks_count > 0 {
        response.push_str(&format!("% {} description(s) and {} remarks found\n\n", descr_count, remarks_count));
    } else if descr_count > 0 {
        response.push_str(&format!("% {} description(s) found\n\n", descr_count));
    } else {
        response.push_str(&format!("% {} remarks found\n\n", remarks_count));
    }

    // Add each field in original order, preserving the exact field name from the original response
    for field in fields {
        response.push_str(&format!("{}:             {}\n", field.field_type, field.value));
    }

    // Add a summary line
    response.push_str(&format!("\n% Total fields: {} (descriptions: {}, remarks: {})\n", 
                              fields.len(), descr_count, remarks_count));

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_desc_and_remarks() {
        let whois_data = r#"
aut-num:        AS64512
as-name:        TEST-AS
descr:          Test Autonomous System
descr:          Example AS for testing
remarks:        This is a test ASN
description:    Alternative description field
remarks:        Contact support for issues
admin-c:        TEST-DN42
tech-c:         TEST-DN42
mnt-by:         TEST-MNT
source:         DN42
        "#;

        let fields = extract_desc_and_remarks(whois_data);
        println!("Extracted fields: {:?}", fields);

        assert_eq!(fields.len(), 5);
        
        // Check order is preserved
        assert_eq!(fields[0].field_type, "descr");
        assert_eq!(fields[0].value, "Test Autonomous System");
        assert_eq!(fields[1].field_type, "descr");
        assert_eq!(fields[1].value, "Example AS for testing");
        assert_eq!(fields[2].field_type, "remarks");
        assert_eq!(fields[2].value, "This is a test ASN");
        assert_eq!(fields[3].field_type, "description");
        assert_eq!(fields[3].value, "Alternative description field");
        assert_eq!(fields[4].field_type, "remarks");
        assert_eq!(fields[4].value, "Contact support for issues");
    }

    #[test]
    fn test_extract_field_value() {
        assert_eq!(
            extract_field_value("descr:          Test Description", "descr"),
            Some("Test Description".to_string())
        );
        assert_eq!(
            extract_field_value("DESCR:          Test Description", "descr"),
            Some("Test Description".to_string())
        );
        assert_eq!(
            extract_field_value("description:    Alternative field", "description"),
            Some("Alternative field".to_string())
        );
        assert_eq!(
            extract_field_value("other-field:    Some value", "descr"),
            None
        );
    }

    #[test]
    fn test_format_desc_response() {
        let fields = vec![
            DescField { field_type: "descr".to_string(), value: "Test Autonomous System".to_string() },
            DescField { field_type: "remarks".to_string(), value: "This is a test ASN".to_string() },
            DescField { field_type: "descr".to_string(), value: "Example AS for testing".to_string() },
        ];

        let response = format_desc_response("AS64512", &fields).unwrap();
        println!("Formatted response:\n{}", response);

        assert!(response.contains("Description Query Results for: AS64512"));
        assert!(response.contains("2 description(s) and 1 remarks found"));
        assert!(response.contains("descr:             Test Autonomous System"));
        assert!(response.contains("remarks:             This is a test ASN"));
        assert!(response.contains("descr:             Example AS for testing"));
        assert!(response.contains("Total fields: 3 (descriptions: 2, remarks: 1)"));
    }

    #[test]
    fn test_format_single_desc_response() {
        let fields = vec![
            DescField { field_type: "descr".to_string(), value: "Single description".to_string() }
        ];

        let response = format_desc_response("example.com", &fields).unwrap();
        println!("Single description response:\n{}", response);

        assert!(response.contains("Description Query Results for: example.com"));
        assert!(response.contains("1 description(s) found"));
        assert!(response.contains("descr:             Single description"));
        assert!(response.contains("Total fields: 1 (descriptions: 1, remarks: 0)"));
    }

    #[test]
    fn test_format_empty_desc_response() {
        let fields = vec![];

        let response = format_desc_response("test.example", &fields).unwrap();
        println!("Empty description response:\n{}", response);

        assert!(response.contains("Description Query Results for: test.example"));
        assert!(response.contains("No description or remarks fields found"));
    }
}
