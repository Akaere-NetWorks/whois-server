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
use regex::Regex;

/// MediaWiki API response structures for page information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWikiResponse {
    pub batchcomplete: Option<String>,
    pub query: Option<MediaWikiQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWikiQuery {
    pub pages: Option<std::collections::HashMap<String, MediaWikiPage>>,
    pub search: Option<Vec<MediaWikiSearchResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWikiPage {
    pub pageid: Option<u64>,
    pub ns: Option<i32>,
    pub title: String,
    pub extract: Option<String>,
    pub revisions: Option<Vec<MediaWikiRevision>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWikiRevision {
    #[serde(rename = "*")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWikiSearchResult {
    pub title: String,
    pub pageid: u64,
    pub size: u64,
    pub wordcount: u64,
    pub snippet: String,
    pub timestamp: String,
}

/// ACGC (Anime/Comic/Game Character) service for character information from Moegirl Wiki
/// 
/// This service fetches character information from zh.moegirl.org.cn using MediaWiki API
pub struct AcgcService {
    client: reqwest::Client,
    base_url: String,
}

impl Default for AcgcService {
    fn default() -> Self {
        Self::new()
    }
}

impl AcgcService {
    /// Create a new ACGC service
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let base_url = "https://zh.moegirl.org.cn/api.php".to_string();

        Self { client, base_url }
    }

    /// Search for a character by name and return detailed information
    pub async fn query_character_info(&self, query: &str) -> Result<String> {
        debug!("Querying ACGC character info for: {}", query);

        // First, try to search for the character
        match self.search_character(query).await {
            Ok(search_results) => {
                if !search_results.is_empty() {
                    // Get detailed info for the first search result
                    let first_result = &search_results[0];
                    debug!("Found character, getting details for: {}", first_result.title);
                    self.get_character_details(&first_result.title).await
                } else {
                    Ok(format!("ACGC Character Not Found: {}\nNo matching characters found on Moegirl Wiki.\n", query))
                }
            }
            Err(e) => {
                error!("ACGC search failed for '{}': {}", query, e);
                Ok(format!("ACGC Query Failed for: {}\nError: {}\n", query, e))
            }
        }
    }

    /// Search for characters by name
    async fn search_character(&self, query: &str) -> Result<Vec<MediaWikiSearchResult>> {
        debug!("Searching Moegirl Wiki for: {}", query);

        let params = [
            ("action", "query"),
            ("format", "json"),
            ("list", "search"),
            ("srsearch", query),
            ("srlimit", "5"),
            ("srnamespace", "0"), // Main namespace
        ];

        let response = self.client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Search request failed: {}", response.status()));
        }

        let wiki_data: MediaWikiResponse = response.json().await?;
        
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

    /// Get detailed character information by page title
    async fn get_character_details(&self, title: &str) -> Result<String> {
        debug!("Getting character details for: {}", title);

        let params = [
            ("action", "query"),
            ("format", "json"),
            ("titles", title),
            ("prop", "extracts|revisions"),
            ("exintro", "1"), 
            ("explaintext", "1"),
            ("exsectionformat", "plain"),
            ("rvprop", "content"),
            ("rvlimit", "1"),
            ("exlimit", "1"),
        ];

        let response = self.client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Details request failed: {}", response.status()));
        }

        let wiki_data: MediaWikiResponse = response.json().await?;
        
        if let Some(query_data) = wiki_data.query {
            if let Some(pages) = query_data.pages {
                for (_, page) in pages {
                    if page.pageid.is_some() {
                        return Ok(self.format_character_info(&page));
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("No character details found"))
    }

    /// Format character information for WHOIS display
    fn format_character_info(&self, page: &MediaWikiPage) -> String {
        let mut output = String::new();

        output.push_str(&format!("ACGC Character Information: {}\n", page.title));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        if let Some(pageid) = page.pageid {
            output.push_str(&format!("page-id: {}\n", pageid));
        }

        output.push_str(&format!("character-name: {}\n", page.title));
        output.push_str(&format!("source: Moegirl Wiki (萌娘百科)\n"));

        // Add character description from extract
        if let Some(extract) = &page.extract {
            if !extract.is_empty() {
                let cleaned_extract = self.clean_wiki_text(extract);
                if !cleaned_extract.is_empty() {
                    output.push_str(&format!("description: {}\n", cleaned_extract));
                }
            }
        }

        // Try to extract additional information from the page content
        if let Some(revisions) = &page.revisions {
            if let Some(revision) = revisions.first() {
                if let Some(content) = &revision.content {
                    let info = self.extract_character_info(content);
                    output.push_str(&info);
                }
            }
        }

        // Add wiki URL
        let encoded_title = urlencoding::encode(&page.title);
        output.push_str(&format!("moegirl-url: https://zh.moegirl.org.cn/{}\n", encoded_title));

        output
    }

    /// Extract character information from wiki content
    fn extract_character_info(&self, content: &str) -> String {
        let mut info = String::new();

        // Extract information from character template patterns
        let template_patterns = [
            // 角色模板信息
            (r"角色\s*\|\s*[^=]*=\s*([^|\n\}]+)", "character-template"),
            (r"作品\s*=\s*([^|\n\}]+)", "source-work"),
            (r"系列\s*=\s*([^|\n\}]+)", "series"),
            
            // 声优/配音信息 (多种格式)
            (r"声优\s*[：=:|]\s*([^|\n\}]+)", "voice-actor"),
            (r"配音\s*[：=:|]\s*([^|\n\}]+)", "voice-actor"), 
            (r"CV\s*[：=:|]\s*([^|\n\}]+)", "voice-actor"),
            (r"日配\s*[：=:|]\s*([^|\n\}]+)", "voice-actor-jp"),
            (r"中配\s*[：=:|]\s*([^|\n\}]+)", "voice-actor-cn"),
            
            // 基本信息
            (r"年龄\s*[：=:|]\s*([^|\n\}]+)", "age"),
            (r"生日\s*[：=:|]\s*([^|\n\}]+)", "birthday"),
            (r"身高\s*[：=:|]\s*([^|\n\}]+)", "height"),
            (r"体重\s*[：=:|]\s*([^|\n\}]+)", "weight"),
            (r"性别\s*[：=:|]\s*([^|\n\}]+)", "gender"),
            (r"种族\s*[：=:|]\s*([^|\n\}]+)", "species"),
            (r"血型\s*[：=:|]\s*([^|\n\}]+)", "blood-type"),
            
            // 外观特征
            (r"发色\s*[：=:|]\s*([^|\n\}]+)", "hair-color"),
            (r"瞳色\s*[：=:|]\s*([^|\n\}]+)", "eye-color"),
            (r"眼色\s*[：=:|]\s*([^|\n\}]+)", "eye-color"),
            (r"头发颜色\s*[：=:|]\s*([^|\n\}]+)", "hair-color"),
            (r"服装\s*[：=:|]\s*([^|\n\}]+)", "clothing"),
            (r"装扮\s*[：=:|]\s*([^|\n\}]+)", "appearance"),
            
            // 身份和角色信息
            (r"出身\s*[：=:|]\s*([^|\n\}]+)", "origin"),
            (r"职业\s*[：=:|]\s*([^|\n\}]+)", "occupation"),
            (r"职务\s*[：=:|]\s*([^|\n\}]+)", "position"),
            (r"身份\s*[：=:|]\s*([^|\n\}]+)", "identity"),
            (r"等级\s*[：=:|]\s*([^|\n\}]+)", "level"),
            (r"阶级\s*[：=:|]\s*([^|\n\}]+)", "class"),
            
            // 性格和特征
            (r"性格\s*[：=:|]\s*([^|\n\}]+)", "personality"),
            (r"萌点\s*[：=:|]\s*([^|\n\}]+)", "moe-points"),
            (r"属性\s*[：=:|]\s*([^|\n\}]+)", "attributes"),
            (r"特征\s*[：=:|]\s*([^|\n\}]+)", "traits"),
            
            // 能力和技能
            (r"喜好\s*[：=:|]\s*([^|\n\}]+)", "hobby"),
            (r"爱好\s*[：=:|]\s*([^|\n\}]+)", "hobby"),
            (r"特技\s*[：=:|]\s*([^|\n\}]+)", "special-skill"),
            (r"能力\s*[：=:|]\s*([^|\n\}]+)", "ability"),
            (r"技能\s*[：=:|]\s*([^|\n\}]+)", "skill"),
            (r"武器\s*[：=:|]\s*([^|\n\}]+)", "weapon"),
            (r"装备\s*[：=:|]\s*([^|\n\}]+)", "equipment"),
            
            // 称号和别名
            (r"称号\s*[：=:|]\s*([^|\n\}]+)", "title"),
            (r"别名\s*[：=:|]\s*([^|\n\}]+)", "alias"),
            (r"外号\s*[：=:|]\s*([^|\n\}]+)", "nickname"),
            (r"绰号\s*[：=:|]\s*([^|\n\}]+)", "nickname"),
            
            // 关系信息
            (r"亲属\s*[：=:|]\s*([^|\n\}]+)", "family"),
            (r"朋友\s*[：=:|]\s*([^|\n\}]+)", "friends"),
            (r"恋人\s*[：=:|]\s*([^|\n\}]+)", "lover"),
            (r"主人\s*[：=:|]\s*([^|\n\}]+)", "master"),
            (r"从属\s*[：=:|]\s*([^|\n\}]+)", "subordinate"),
        ];

        // Extract using enhanced patterns with deduplication
        let mut extracted_info: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
        
        for (pattern, field_name) in template_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for captures in re.captures_iter(content) {
                    if let Some(value) = captures.get(1) {
                        let cleaned_value = self.clean_wiki_text(value.as_str());
                        
                        // Filter out invalid/meaningless content
                        if !cleaned_value.is_empty() && 
                           cleaned_value.len() < 300 && 
                           cleaned_value.len() > 1 &&
                           !cleaned_value.starts_with("Category:") &&
                           !cleaned_value.contains("内容=") &&
                           cleaned_value != "Race" &&
                           cleaned_value != "Skill" &&
                           cleaned_value != "Ultimate Skill" &&
                           !cleaned_value.contains("{{") &&
                           !cleaned_value.contains("}}") {
                            
                            let entry = extracted_info.entry(field_name.to_string()).or_insert_with(std::collections::HashSet::new);
                            entry.insert(cleaned_value);
                        }
                    }
                }
            }
        }
        
        // Format deduplicated information
        for (field, values) in extracted_info {
            if !values.is_empty() {
                let combined_values: Vec<String> = values.into_iter().collect();
                if combined_values.len() == 1 {
                    info.push_str(&format!("{}: {}\n", field, combined_values[0]));
                } else {
                    info.push_str(&format!("{}: {}\n", field, combined_values.join(", ")));
                }
            }
        }

        // Extract萌娘百科分类信息
        if let Ok(re) = Regex::new(r"\[\[Category:([^\]]+)\]\]") {
            let mut categories = Vec::new();
            for captures in re.captures_iter(content) {
                if let Some(category) = captures.get(1) {
                    let cat = category.as_str();
                    // 只保留角色相关的分类
                    if cat.contains("角色") || cat.contains("人物") || cat.contains("萌点") || 
                       cat.contains("属性") || cat.contains("声优") || cat.contains("CV") {
                        categories.push(cat);
                    }
                }
            }
            if !categories.is_empty() && categories.len() <= 10 {
                info.push_str(&format!("categories: {}\n", categories.join(", ")));
            }
        }

        info
    }

    /// Clean wiki markup from text
    fn clean_wiki_text(&self, text: &str) -> String {
        let mut text = text.trim().to_string();
        
        // Early return for clearly invalid content
        if text.is_empty() || text == "=" || text.starts_with("内容=") {
            return String::new();
        }
        
        // Remove incomplete MediaWiki templates ({{...}} and incomplete ones)
        if let Ok(re) = Regex::new(r"\{\{[^}]*(\}\})?") {
            text = re.replace_all(&text, "").to_string();
        }
        
        // Remove wiki links and keep only the display text
        // Handle [[link|display]] -> display
        if let Ok(re) = Regex::new(r"\[\[([^|\]]*\|)?([^\]]*)\]\]") { 
            text = re.replace_all(&text, "$2").to_string();
        }
        
        // Remove incomplete wiki links like [[text without closing
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
        
        // Remove wiki table markup and excess pipes
        text = text.replace("|-", "");
        if let Ok(re) = Regex::new(r"\|+") {
            text = re.replace_all(&text, " ").to_string();
        }
        
        // Clean up multiple spaces and newlines
        if let Ok(re) = Regex::new(r"\s+") {
            text = re.replace_all(&text, " ").to_string();
        }
        
        // Remove common wiki formatting remnants
        text = text.replace("&nbsp;", " ");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&amp;", "&");
        
        // Remove trailing incomplete content that might cause issues
        if let Ok(re) = Regex::new(r"[{<[].*$") {
            if text.len() > 20 && re.is_match(&text) {
                if let Some(pos) = text.find(|c| c == '{' || c == '<' || c == '[') {
                    if pos > 10 { // Keep some content before the incomplete markup
                        text = text[..pos].to_string();
                    }
                }
            }
        }
        
        // Remove trailing commas and unnecessary punctuation
        text = text.trim_end_matches(',').trim_end_matches('、').to_string();
        
        let result = text.trim().to_string();
        
        // Final validation - reject meaningless or too short content
        if result.len() < 2 || result == "=" || result.contains("内容=") {
            String::new()
        } else {
            result
        }
    }

    /// Check if a query string is an ACGC query
    pub fn is_acgc_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-ACGC")
    }

    /// Parse ACGC query to extract the character name
    pub fn parse_acgc_query(query: &str) -> Option<String> {
        if !Self::is_acgc_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 5]; // Remove "-ACGC"
        Some(clean_query.to_string())
    }
}

/// Process ACGC query with -ACGC suffix
pub async fn process_acgc_query(query: &str) -> Result<String> {
    let acgc_service = AcgcService::new();
    
    if let Some(character_query) = AcgcService::parse_acgc_query(query) {
        debug!("Processing ACGC query for: {}", character_query);
        
        if character_query.is_empty() {
            return Ok(format!("Invalid ACGC query. Please provide a character name.\nExample: 利姆鲁-ACGC\n"));
        }
        
        acgc_service.query_character_info(&character_query).await
    } else {
        error!("Invalid ACGC query format: {}", query);
        Ok(format!("Invalid ACGC query format. Use: <character_name>-ACGC\nExample: 利姆鲁-ACGC\nQuery: {}\n", query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acgc_query_detection() {
        assert!(AcgcService::is_acgc_query("利姆鲁-ACGC"));
        assert!(AcgcService::is_acgc_query("Rimuru-ACGC"));
        assert!(AcgcService::is_acgc_query("炭治郎-ACGC"));
        
        assert!(!AcgcService::is_acgc_query("利姆鲁"));
        assert!(!AcgcService::is_acgc_query("example.com-SSL"));
        assert!(!AcgcService::is_acgc_query("ACGC-利姆鲁"));
    }

    #[test]
    fn test_acgc_query_parsing() {
        assert_eq!(
            AcgcService::parse_acgc_query("利姆鲁-ACGC"),
            Some("利姆鲁".to_string())
        );
        
        assert_eq!(
            AcgcService::parse_acgc_query("Rimuru Tempest-ACGC"),
            Some("Rimuru Tempest".to_string())
        );
        
        assert_eq!(AcgcService::parse_acgc_query("利姆鲁"), None);
    }

    #[test]
    fn test_clean_wiki_text() {
        let service = AcgcService::new();
        
        assert_eq!(
            service.clean_wiki_text("{{角色|利姆鲁}}"),
            "角色|利姆鲁"
        );
        
        assert_eq!(
            service.clean_wiki_text("[[转生史莱姆]]的主角"),
            "转生史莱姆的主角"
        );
        
        assert_eq!(
            service.clean_wiki_text("'''史莱姆'''"),
            "史莱姆"
        );
    }

    #[tokio::test]
    async fn test_acgc_service_creation() {
        let service = AcgcService::new();
        // Just test that creation doesn't panic
        assert_eq!(service.base_url, "https://zh.moegirl.org.cn/api.php");
    }
}