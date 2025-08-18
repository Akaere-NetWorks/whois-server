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
use serde::{ Deserialize, Serialize };
use tracing::{ debug, error };
use regex::Regex;

/// Wikipedia API response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaResponse {
    pub batchcomplete: Option<String>,
    pub query: Option<WikipediaQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaQuery {
    pub pages: Option<std::collections::HashMap<String, WikipediaPage>>,
    pub search: Option<Vec<WikipediaSearchResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaPage {
    pub pageid: Option<u64>,
    pub ns: Option<i32>,
    pub title: String,
    pub extract: Option<String>,
    pub revisions: Option<Vec<WikipediaRevision>>,
    pub fullurl: Option<String>,
    pub editurl: Option<String>,
    pub canonicalurl: Option<String>,
    pub length: Option<u64>,
    pub touched: Option<String>,
    pub categories: Option<Vec<WikipediaCategory>>,
    pub langlinks: Option<Vec<WikipediaLangLink>>,
    pub pageviews: Option<std::collections::HashMap<String, Option<u64>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaRevision {
    #[serde(rename = "*")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaSearchResult {
    pub title: String,
    pub pageid: u64,
    pub size: u64,
    pub wordcount: u64,
    pub snippet: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaCategory {
    pub title: Option<String>,
    #[serde(rename = "*")]
    pub sortkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaLangLink {
    pub lang: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
}

/// Wikipedia service for article information
///
/// This service fetches article information from Wikipedia using MediaWiki API
pub struct WikipediaService {
    client: reqwest::Client,
    base_url: String,
}

impl Default for WikipediaService {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaService {
    /// Create a new Wikipedia service
    pub fn new() -> Self {
        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"
            )
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let base_url = "https://en.wikipedia.org/w/api.php".to_string();

        Self { client, base_url }
    }

    /// Query Wikipedia article information by title
    pub async fn query_article_info(&self, query: &str) -> Result<String> {
        debug!("Querying Wikipedia article info for: {}", query);

        // First, try to search for the article
        match self.search_article(query).await {
            Ok(search_results) => {
                if !search_results.is_empty() {
                    // Get detailed info for the first search result
                    let first_result = &search_results[0];
                    debug!("Found article, getting details for: {}", first_result.title);
                    self.get_article_details(&first_result.title).await
                } else {
                    Ok(
                        format!("Wikipedia Article Not Found: {}\nNo matching articles found on Wikipedia.\n", query)
                    )
                }
            }
            Err(e) => {
                error!("Wikipedia search failed for '{}': {}", query, e);
                Ok(format!("Wikipedia Query Failed for: {}\nError: {}\n", query, e))
            }
        }
    }

    /// Search for articles by title
    async fn search_article(&self, query: &str) -> Result<Vec<WikipediaSearchResult>> {
        debug!("Searching Wikipedia for: {}", query);

        let params = [
            ("action", "query"),
            ("format", "json"),
            ("list", "search"),
            ("srsearch", query),
            ("srlimit", "5"),
            ("srnamespace", "0"), // Main namespace
            ("srprop", "size|wordcount|timestamp|snippet"),
            ("utf8", "1"),
        ];

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        let status = response.status();
        debug!("Wikipedia search response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text().await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            debug!("Wikipedia search error response: {}", error_text);
            return Err(anyhow::anyhow!("Search request failed: {} - {}", status, error_text));
        }

        let response_text = response.text().await?;
        debug!(
            "Wikipedia search response body: {}",
            &response_text[..std::cmp::min(500, response_text.len())]
        );

        let wiki_data: WikipediaResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                anyhow::anyhow!(
                    "Failed to parse Wikipedia search response: {} - Response: {}",
                    e,
                    &response_text[..std::cmp::min(200, response_text.len())]
                )
            )?;

        if let Some(query_data) = wiki_data.query {
            if let Some(search_results) = query_data.search {
                Ok(search_results)
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    }

    /// Get detailed article information by page title
    async fn get_article_details(&self, title: &str) -> Result<String> {
        debug!("Getting article details for: {}", title);

        let params = [
            ("action", "query"),
            ("format", "json"),
            ("titles", title),
            ("prop", "extracts|info|categories|langlinks"),
            ("exintro", "1"),
            ("explaintext", "1"),
            ("exsectionformat", "plain"),
            ("exlimit", "1"),
            ("inprop", "url|length|touched"),
            ("cllimit", "10"), // Limit categories to 10
            ("lllimit", "10"), // Limit language links to 10
            ("utf8", "1"),
        ];

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        let status = response.status();
        debug!("Wikipedia details response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text().await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            debug!("Wikipedia details error response: {}", error_text);
            return Err(anyhow::anyhow!("Details request failed: {} - {}", status, error_text));
        }

        let response_text = response.text().await?;
        debug!(
            "Wikipedia details response body: {}",
            &response_text[..std::cmp::min(500, response_text.len())]
        );

        let wiki_data: WikipediaResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                anyhow::anyhow!(
                    "Failed to parse Wikipedia details response: {} - Response: {}",
                    e,
                    &response_text[..std::cmp::min(200, response_text.len())]
                )
            )?;

        if let Some(query_data) = wiki_data.query {
            if let Some(pages) = query_data.pages {
                for (_, page) in pages {
                    if page.pageid.is_some() {
                        return Ok(self.format_article_info(&page));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("No article details found"))
    }

    /// Format article information for WHOIS display
    fn format_article_info(&self, page: &WikipediaPage) -> String {
        let mut output = String::new();

        output.push_str(&format!("Wikipedia Article Information: {}\n", page.title));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        if let Some(pageid) = page.pageid {
            output.push_str(&format!("page-id: {}\n", pageid));
        }

        output.push_str(&format!("title: {}\n", page.title));
        output.push_str(&format!("source: Wikipedia (English)\n"));

        // Add article length and last modified date
        if let Some(length) = page.length {
            output.push_str(&format!("article-length: {} bytes\n", length));
        }

        if let Some(touched) = &page.touched {
            // Parse and format the timestamp (try multiple formats)
            if
                let Ok(parsed_time) = chrono::DateTime::parse_from_str(
                    touched,
                    "%Y-%m-%dT%H:%M:%SZ"
                )
            {
                output.push_str(
                    &format!("last-modified: {}\n", parsed_time.format("%Y-%m-%d %H:%M:%S UTC"))
                );
            } else if
                let Ok(parsed_time) = chrono::NaiveDateTime::parse_from_str(touched, "%Y%m%d%H%M%S")
            {
                let utc_time = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    parsed_time,
                    chrono::Utc
                );
                output.push_str(
                    &format!("last-modified: {}\n", utc_time.format("%Y-%m-%d %H:%M:%S UTC"))
                );
            } else {
                output.push_str(&format!("last-modified: {}\n", touched));
            }
        }

        // Add article extract/summary
        if let Some(extract) = &page.extract {
            if !extract.is_empty() {
                let cleaned_extract = self.clean_wiki_text(extract);
                if !cleaned_extract.is_empty() {
                    // Limit extract to reasonable length
                    let truncated_extract = if cleaned_extract.len() > 800 {
                        format!("{}...", &cleaned_extract[..800])
                    } else {
                        cleaned_extract
                    };
                    output.push_str(&format!("summary: {}\n", truncated_extract));
                }
            }
        }

        // Add categories
        if let Some(categories) = &page.categories {
            if !categories.is_empty() {
                let category_names: Vec<String> = categories
                    .iter()
                    .filter_map(|cat| cat.title.as_ref())
                    .map(|title| title.replace("Category:", ""))
                    .take(8) // Limit to 8 categories for readability
                    .collect();

                if !category_names.is_empty() {
                    output.push_str(&format!("categories: {}\n", category_names.join(", ")));
                }
            }
        }

        // Add language links
        if let Some(langlinks) = &page.langlinks {
            if !langlinks.is_empty() {
                let lang_info: Vec<String> = langlinks
                    .iter()
                    .filter_map(|link| {
                        if let (Some(lang), Some(title)) = (&link.lang, &link.title) {
                            Some(format!("{} ({})", lang, title))
                        } else {
                            None
                        }
                    })
                    .take(8) // Limit to 8 languages for readability
                    .collect();

                if !lang_info.is_empty() {
                    output.push_str(&format!("languages: {}\n", lang_info.join(", ")));
                }
            }
        }

        // Add URLs
        if let Some(url) = &page.canonicalurl {
            output.push_str(&format!("wikipedia-url: {}\n", url));
        } else if let Some(url) = &page.fullurl {
            output.push_str(&format!("wikipedia-url: {}\n", url));
        } else {
            // Construct URL from title
            let encoded_title = urlencoding::encode(&page.title);
            output.push_str(
                &format!("wikipedia-url: https://en.wikipedia.org/wiki/{}\n", encoded_title)
            );
        }

        if let Some(edit_url) = &page.editurl {
            output.push_str(&format!("edit-url: {}\n", edit_url));
        }

        output.push_str("% Information retrieved from Wikipedia via MediaWiki API\n");
        output.push_str("% Query processed by WHOIS server\n");

        output
    }

    /// Clean wiki markup from text
    fn clean_wiki_text(&self, text: &str) -> String {
        let mut text = text.trim().to_string();

        // Early return for clearly invalid content
        if text.is_empty() {
            return String::new();
        }

        // Remove MediaWiki templates and references
        if let Ok(re) = Regex::new(r"\{\{[^}]*\}\}") {
            text = re.replace_all(&text, "").to_string();
        }

        // Remove wiki links and keep only the display text
        // Handle [[link|display]] -> display
        if let Ok(re) = Regex::new(r"\[\[([^|\]]*\|)?([^\]]*)\]\]") {
            text = re.replace_all(&text, "$2").to_string();
        }

        // Remove incomplete wiki links
        if let Ok(re) = Regex::new(r"\[\[[^\]]*$") {
            text = re.replace_all(&text, "").to_string();
        }

        // Remove bold and italic formatting
        text = text.replace("'''", "").replace("''", "");

        // Remove HTML tags
        if let Ok(re) = Regex::new(r"<[^>]*>") {
            text = re.replace_all(&text, "").to_string();
        }

        // Remove ref tags content
        if let Ok(re) = Regex::new(r"<ref[^>]*>.*?</ref>") {
            text = re.replace_all(&text, "").to_string();
        }

        // Clean up multiple spaces and newlines
        if let Ok(re) = Regex::new(r"\s+") {
            text = re.replace_all(&text, " ").to_string();
        }

        // Remove common HTML entities
        text = text.replace("&nbsp;", " ");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&amp;", "&");
        text = text.replace("&quot;", "\"");
        text = text.replace("&#39;", "'");

        text.trim().to_string()
    }

    /// Check if a query string is a Wikipedia query
    pub fn is_wikipedia_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-WIKIPEDIA")
    }

    /// Parse Wikipedia query to extract the article name
    pub fn parse_wikipedia_query(query: &str) -> Option<String> {
        if !Self::is_wikipedia_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 10]; // Remove "-WIKIPEDIA"
        Some(clean_query.to_string())
    }
}

/// Process Wikipedia query with -WIKIPEDIA suffix
pub async fn process_wikipedia_query(query: &str) -> Result<String> {
    let wikipedia_service = WikipediaService::new();

    if let Some(article_query) = WikipediaService::parse_wikipedia_query(query) {
        debug!("Processing Wikipedia query for: {}", article_query);

        if article_query.is_empty() {
            return Ok(
                format!(
                    "Invalid Wikipedia query. Please provide an article name.\nExample: Rust-WIKIPEDIA\n"
                )
            );
        }

        wikipedia_service.query_article_info(&article_query).await
    } else {
        error!("Invalid Wikipedia query format: {}", query);
        Ok(
            format!("Invalid Wikipedia query format. Use: <article_name>-WIKIPEDIA\nExample: Rust-WIKIPEDIA\nQuery: {}\n", query)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wikipedia_query_detection() {
        assert!(WikipediaService::is_wikipedia_query("Rust-WIKIPEDIA"));
        assert!(WikipediaService::is_wikipedia_query("Python-WIKIPEDIA"));
        assert!(WikipediaService::is_wikipedia_query("Linux-wikipedia"));

        assert!(!WikipediaService::is_wikipedia_query("Rust"));
        assert!(!WikipediaService::is_wikipedia_query("example.com-SSL"));
        assert!(!WikipediaService::is_wikipedia_query("WIKIPEDIA-Rust"));
    }

    #[test]
    fn test_wikipedia_query_parsing() {
        assert_eq!(
            WikipediaService::parse_wikipedia_query("Rust-WIKIPEDIA"),
            Some("Rust".to_string())
        );

        assert_eq!(
            WikipediaService::parse_wikipedia_query("Machine Learning-WIKIPEDIA"),
            Some("Machine Learning".to_string())
        );

        assert_eq!(WikipediaService::parse_wikipedia_query("Rust"), None);
    }

    #[test]
    fn test_clean_wiki_text() {
        let service = WikipediaService::new();

        assert_eq!(service.clean_wiki_text("'''Bold text'''"), "Bold text");

        assert_eq!(service.clean_wiki_text("[[Link|Display text]]"), "Display text");

        assert_eq!(
            service.clean_wiki_text("Normal text with [[link]] in it"),
            "Normal text with link in it"
        );
    }

    #[tokio::test]
    async fn test_wikipedia_service_creation() {
        let service = WikipediaService::new();
        // Just test that creation doesn't panic
        assert_eq!(service.base_url, "https://en.wikipedia.org/w/api.php");
    }
}
