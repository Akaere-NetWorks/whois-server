/// Extract IP address from network prefix for IPinfo API queries
pub fn extract_ip_from_prefix(prefix: &str) -> String {
    // Handle IPv6 prefixes like "2a14:67c1:a024::/48"
    if prefix.contains("::") && prefix.contains("/") {
        let ip_part = prefix.split("/").next().unwrap_or(prefix);
        
        // For IPv6 prefixes ending with "::", append a zero to get a valid address
        if ip_part.ends_with("::") {
            return ip_part.to_string();  // IPinfo accepts "::" format
        } else {
            return ip_part.to_string();
        }
    }
    
    // Handle IPv4 prefixes like "192.168.1.0/24"
    if prefix.contains("/") {
        if let Some(ip_part) = prefix.split("/").next() {
            return ip_part.to_string();
        }
    }
    
    // Return as-is if no special handling needed
    prefix.to_string()
}

/// Truncate string to specified length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("very_long_string", 10), "very_lo...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }
} 