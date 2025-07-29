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

use std::time::Duration;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

/// Luotianyi lyric API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricResponse {
    pub title: String,
    pub author: Vec<String>,
    pub year: u32,
    pub lines: Vec<String>,
}

/// Luotianyi lyric service for random lyrics
/// 
/// This service fetches random Luotianyi lyrics from lty.vc API
pub struct LyricService {
    client: reqwest::Client,
    base_url: String,
}

impl Default for LyricService {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricService {
    /// Create a new lyric service
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("WhoisServer/1.0 (https://github.com/akaere/whois-server)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let base_url = "https://lty.vc/lyric".to_string();

        Self { client, base_url }
    }

    /// Get random Luotianyi lyric
    pub async fn get_random_lyric(&self) -> Result<String> {
        debug!("Fetching random Luotianyi lyric from API");

        let params = [("format", "json")];

        let response = self.client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await?;

        let status = response.status();
        debug!("Lyric API response status: {}", status);
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
            debug!("Lyric API error response: {}", error_text);
            return Err(anyhow::anyhow!("Lyric request failed: {} - {}", status, error_text));
        }

        let response_text = response.text().await?;
        debug!("Lyric API response body: {}", &response_text[..std::cmp::min(200, response_text.len())]);
        
        let lyric_data: LyricResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse lyric response: {} - Response: {}", e, &response_text[..std::cmp::min(100, response_text.len())]))?;

        Ok(self.format_lyric_info(&lyric_data))
    }

    /// Format lyric information for WHOIS display
    fn format_lyric_info(&self, lyric: &LyricResponse) -> String {
        let mut output = String::new();

        output.push_str(&format!("Luotianyi Random Lyric: {}\n", lyric.title));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("song-name: {}\n", lyric.title));
        output.push_str(&format!("singer: 洛天依 (Luotianyi)\n"));
        
        if !lyric.author.is_empty() {
            output.push_str(&format!("author: {}\n", lyric.author.join(", ")));
        }

        output.push_str(&format!("year: {}\n", lyric.year));
        output.push_str(&format!("source: lty.vc\n"));

        // Add lyric content with proper formatting
        output.push_str("\n");
        output.push_str("lyric-content:\n");
        for line in &lyric.lines {
            output.push_str(&format!("{}\n", line));
        }

        output.push_str("\n");
        output.push_str("% Information retrieved from lty.vc API\n");
        output.push_str("% Query processed by WHOIS server\n");
        
        output
    }

    /// Check if a query string is a lyric query
    pub fn is_lyric_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-LYRIC")
    }

    /// Parse lyric query to extract any parameters (currently just returns empty string)
    pub fn parse_lyric_query(query: &str) -> Option<String> {
        if !Self::is_lyric_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 6]; // Remove "-LYRIC"
        Some(clean_query.to_string())
    }
}

/// Process lyric query with -LYRIC suffix
pub async fn process_lyric_query(query: &str) -> Result<String> {
    let lyric_service = LyricService::new();
    
    if LyricService::parse_lyric_query(query).is_some() {
        debug!("Processing Luotianyi lyric query");
        lyric_service.get_random_lyric().await
    } else {
        error!("Invalid lyric query format: {}", query);
        Ok(format!("Invalid lyric query format. Use: <any_text>-LYRIC or just -LYRIC\nExample: random-LYRIC\nQuery: {}\n", query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lyric_query_detection() {
        assert!(LyricService::is_lyric_query("random-LYRIC"));
        assert!(LyricService::is_lyric_query("luotianyi-LYRIC"));
        assert!(LyricService::is_lyric_query("-LYRIC"));
        assert!(LyricService::is_lyric_query("test-lyric"));
        
        assert!(!LyricService::is_lyric_query("random"));
        assert!(!LyricService::is_lyric_query("example.com-SSL"));
        assert!(!LyricService::is_lyric_query("LYRIC-random"));
    }

    #[test]
    fn test_lyric_query_parsing() {
        assert_eq!(
            LyricService::parse_lyric_query("random-LYRIC"),
            Some("random".to_string())
        );
        
        assert_eq!(
            LyricService::parse_lyric_query("-LYRIC"),
            Some("".to_string())
        );
        
        assert_eq!(LyricService::parse_lyric_query("random"), None);
    }

    #[tokio::test]
    async fn test_lyric_service_creation() {
        let service = LyricService::new();
        // Just test that creation doesn't panic
        assert_eq!(service.base_url, "https://lty.vc/lyric");
    }
}