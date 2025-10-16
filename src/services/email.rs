use std::collections::HashSet;
use anyhow::Result;
use tracing::debug;

// Removed unused import
use crate::dn42::query_dn42_raw_managed;

/// Process email search queries ending with -EMAIL
pub async fn process_email_search(base_query: &str) -> Result<String> {
    debug!("Processing email search for: {}", base_query);

    // First, query the base object to get references
    let base_response = query_dn42_raw_managed(base_query).await?;
    debug!("Base response length: {} chars", base_response.len());

    // Start with emails from the base object itself
    let mut emails = HashSet::new();
    let base_emails = extract_emails(&base_response);
    debug!("Found {} emails in base object: {:?}", base_emails.len(), base_emails);
    emails.extend(base_emails);

    // Extract references from the base object
    let references = extract_references(&base_response);
    debug!("Found references: {:?}", references);

    // If no references found and no emails in base, try some common related queries
    if references.is_empty() && emails.is_empty() {
        debug!("No references or emails found, trying related queries");

        // Try querying with common suffixes if not already present
        let mut related_queries = vec![];

        if !base_query.to_uppercase().ends_with("-MNT") {
            related_queries.push(format!("{}-MNT", base_query));
        }
        if !base_query.to_uppercase().ends_with("-DN42") {
            related_queries.push(format!("{}-DN42", base_query));
        }

        for related_query in related_queries {
            debug!("Trying related query: {}", related_query);
            match query_dn42_raw_managed(&related_query).await {
                Ok(related_response) => {
                    let related_emails = extract_emails(&related_response);
                    debug!(
                        "Found {} emails in related query {}: {:?}",
                        related_emails.len(),
                        related_query,
                        related_emails
                    );
                    emails.extend(related_emails);

                    // Also extract references from related objects
                    let related_refs = extract_references(&related_response);
                    for ref_name in related_refs {
                        if !references.contains(&ref_name) {
                            debug!(
                                "Querying additional reference from {}: {}",
                                related_query,
                                ref_name
                            );
                            match query_dn42_raw_managed(&ref_name).await {
                                Ok(ref_response) => {
                                    let ref_emails = extract_emails(&ref_response);
                                    debug!(
                                        "Found {} emails in additional reference {}: {:?}",
                                        ref_emails.len(),
                                        ref_name,
                                        ref_emails
                                    );
                                    emails.extend(ref_emails);
                                }
                                Err(e) => {
                                    debug!(
                                        "Failed to query additional reference {}: {}",
                                        ref_name,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Related query {} failed: {}", related_query, e);
                }
            }
        }
    }

    // Query each reference to find email addresses
    for reference in references {
        debug!("Querying reference: {}", reference);
        match query_dn42_raw_managed(&reference).await {
            Ok(ref_response) => {
                let ref_emails = extract_emails(&ref_response);
                debug!("Found {} emails in {}: {:?}", ref_emails.len(), reference, ref_emails);
                emails.extend(ref_emails);
            }
            Err(e) => {
                debug!("Failed to query reference {}: {}", reference, e);
            }
        }
    }

    debug!("Total unique emails found: {}", emails.len());

    // Format response
    format_email_response(&emails)
}

/// Process email search queries ending with -EMAIL (blocking version)
fn extract_references(response: &str) -> Vec<String> {
    let mut references = Vec::new();

    for line in response.lines() {
        let line = line.trim();

        // Look for mnt-by, admin-c, and tech-c fields
        if let Some(value) = extract_field_value(line, "mnt-by") {
            references.push(value);
        } else if let Some(value) = extract_field_value(line, "admin-c") {
            references.push(value);
        } else if let Some(value) = extract_field_value(line, "tech-c") {
            references.push(value);
        }
    }

    // Remove duplicates while preserving order
    let mut unique_refs = Vec::new();
    let mut seen = HashSet::new();
    for ref_name in references {
        if seen.insert(ref_name.clone()) {
            unique_refs.push(ref_name);
        }
    }

    unique_refs
}

/// Extract email addresses from WHOIS response
fn extract_emails(response: &str) -> Vec<String> {
    let mut emails = Vec::new();

    for line in response.lines() {
        let line = line.trim();

        // Look for various email fields
        if let Some(email) = extract_field_value(line, "abuse-mailbox") {
            debug!("Found abuse-mailbox: {}", email);
            emails.push(email);
        } else if let Some(email) = extract_field_value(line, "e-mail") {
            debug!("Found e-mail: {}", email);
            emails.push(email);
        } else if let Some(email) = extract_field_value(line, "email") {
            debug!("Found email: {}", email);
            emails.push(email);
        } else if let Some(email) = extract_field_value(line, "abuse-c") {
            // Sometimes abuse-c contains email directly
            if email.contains("@") {
                debug!("Found email in abuse-c: {}", email);
                emails.push(email);
            }
        }
    }

    emails
}

/// Extract value from a WHOIS field line
fn extract_field_value(line: &str, field_name: &str) -> Option<String> {
    if line.starts_with(field_name) {
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

/// Format email search response
fn format_email_response(emails: &HashSet<String>) -> Result<String> {
    if emails.is_empty() {
        return Ok("% Email Search\n% No email addresses found\n".to_string());
    }

    let mut response = String::from("% Email Search\n");

    // Add each unique email address
    for email in emails {
        response.push_str(&format!("e-mail:             {}\n", email));
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_emails() {
        let whois_data =
            r#"
person:         Test Person
e-mail:         test@example.com
abuse-mailbox:  abuse@example.com
email:          another@example.com
tech-c:         TEST-DN42
admin-c:        TEST-DN42
        "#;

        let emails = extract_emails(whois_data);
        println!("Extracted emails: {:?}", emails);

        assert!(emails.contains(&"test@example.com".to_string()));
        assert!(emails.contains(&"abuse@example.com".to_string()));
        assert!(emails.contains(&"another@example.com".to_string()));
    }

    #[test]
    fn test_extract_references() {
        let whois_data =
            r#"
aut-num:        AS213605
mnt-by:         LiuHaoRan-MNT
admin-c:        PYSIO-DN42
tech-c:         PYSIO-DN42
source:         DN42
        "#;

        let refs = extract_references(whois_data);
        println!("Extracted references: {:?}", refs);

        assert!(refs.contains(&"LiuHaoRan-MNT".to_string()));
        assert!(refs.contains(&"PYSIO-DN42".to_string()));
    }
}
