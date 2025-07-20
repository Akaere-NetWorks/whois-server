use anyhow::Result;
use tracing::debug;

use crate::config::DEFAULT_WHOIS_PORT;
use super::whois::{query_whois, blocking_query_whois};

// BGP Tools WHOIS server
const BGPTOOLS_WHOIS_SERVER: &str = "bgp.tools";

/// Process BGP Tools queries ending with -BGPTOOL
pub async fn process_bgptool_query(base_query: &str) -> Result<String> {
    debug!("Processing BGP Tools query for: {}", base_query);
    
    // Format query for BGP Tools (add -v flag as expected by bgp.tools)
    let formatted_query = format!(" -v {}", base_query);
    debug!("Formatted BGP Tools query: {}", formatted_query);
    
    // Query BGP Tools WHOIS server directly
    let response = query_whois(&formatted_query, BGPTOOLS_WHOIS_SERVER, DEFAULT_WHOIS_PORT).await?;
    
    // Format response with BGP Tools header
    format_bgptool_response(&response)
}

/// Process BGP Tools queries ending with -BGPTOOL (blocking version)
pub fn process_bgptool_query_blocking(base_query: &str, timeout: std::time::Duration) -> Result<String> {
    debug!("Processing BGP Tools query (blocking) for: {}", base_query);
    
    // Format query for BGP Tools (add -v flag as expected by bgp.tools)
    let formatted_query = format!(" -v {}", base_query);
    debug!("Formatted BGP Tools query (blocking): {}", formatted_query);
    
    // Query BGP Tools WHOIS server directly
    let response = blocking_query_whois(&formatted_query, BGPTOOLS_WHOIS_SERVER, DEFAULT_WHOIS_PORT, timeout)?;
    
    // Format response with BGP Tools header
    format_bgptool_response(&response)
}

/// Format BGP Tools response with appropriate header
fn format_bgptool_response(response: &str) -> Result<String> {
    let mut formatted = String::from("% BGP Tools Query\n");
    formatted.push_str("% Data from bgp.tools\n");
    formatted.push_str("\n");
    
    // Add the response content
    formatted.push_str(response);
    
    // Ensure response ends properly
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bgptool_response() {
        let sample_response = "AS213605\nDescription: Test AS\nCountry: US";
        let formatted = format_bgptool_response(sample_response).unwrap();
        
        assert!(formatted.contains("% BGP Tools Query"));
        assert!(formatted.contains("% Data from bgp.tools"));
        assert!(formatted.contains("AS213605"));
        assert!(formatted.contains("Description: Test AS"));
    }
} 