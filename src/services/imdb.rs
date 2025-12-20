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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::{log_debug, log_error, log_warn};
/// IMDb API response structures for movie/TV show information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImdbResponse {
    #[serde(rename = "Response")]
    pub response: String,
    #[serde(rename = "Title")]
    pub title: Option<String>,
    #[serde(rename = "Year")]
    pub year: Option<String>,
    #[serde(rename = "Rated")]
    pub rated: Option<String>,
    #[serde(rename = "Released")]
    pub released: Option<String>,
    #[serde(rename = "Runtime")]
    pub runtime: Option<String>,
    #[serde(rename = "Genre")]
    pub genre: Option<String>,
    #[serde(rename = "Director")]
    pub director: Option<String>,
    #[serde(rename = "Writer")]
    pub writer: Option<String>,
    #[serde(rename = "Actors")]
    pub actors: Option<String>,
    #[serde(rename = "Plot")]
    pub plot: Option<String>,
    #[serde(rename = "Language")]
    pub language: Option<String>,
    #[serde(rename = "Country")]
    pub country: Option<String>,
    #[serde(rename = "Awards")]
    pub awards: Option<String>,
    #[serde(rename = "Poster")]
    pub poster: Option<String>,
    #[serde(rename = "Ratings")]
    pub ratings: Option<Vec<ImdbRating>>,
    #[serde(rename = "Metascore")]
    pub metascore: Option<String>,
    #[serde(rename = "imdbRating")]
    pub imdb_rating: Option<String>,
    #[serde(rename = "imdbVotes")]
    pub imdb_votes: Option<String>,
    #[serde(rename = "imdbID")]
    pub imdb_id: Option<String>,
    #[serde(rename = "Type")]
    pub content_type: Option<String>,
    #[serde(rename = "DVD")]
    pub dvd: Option<String>,
    #[serde(rename = "BoxOffice")]
    pub box_office: Option<String>,
    #[serde(rename = "Production")]
    pub production: Option<String>,
    #[serde(rename = "Website")]
    pub website: Option<String>,
    #[serde(rename = "Error")]
    pub error: Option<String>,
    #[serde(rename = "totalSeasons")]
    pub total_seasons: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImdbRating {
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Value")]
    pub value: String,
}

/// IMDb search response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImdbSearchResponse {
    #[serde(rename = "Search")]
    pub search: Option<Vec<ImdbSearchResult>>,
    #[serde(rename = "Response")]
    pub response: String,
    #[serde(rename = "Error")]
    pub error: Option<String>,
    #[serde(rename = "totalResults")]
    pub total_results: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImdbSearchResult {
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Year")]
    pub year: String,
    #[serde(rename = "imdbID")]
    pub imdb_id: String,
    #[serde(rename = "Type")]
    pub content_type: String,
    #[serde(rename = "Poster")]
    pub poster: String,
}

/// IMDb service for movie and TV show information queries
///
/// To enable IMDb queries, set the OMDB_API_KEY environment variable
/// or add it to a .env file in the project root:
/// ```
/// OMDB_API_KEY=your_omdb_api_key_here
/// ```
/// You can get a free API key from: http://www.omdbapi.com/apikey.aspx
pub struct ImdbService {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl Default for ImdbService {
    fn default() -> Self {
        Self::new()
    }
}

impl ImdbService {
    /// Create a new IMDb service
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("WhoisServer/1.0 IMDb API Client")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // Try to load .env file first (ignore errors if file doesn't exist)
        let _ = dotenv::dotenv();

        // Try to get API key from environment variable (including from .env file)
        let api_key = std::env::var("OMDB_API_KEY").ok();
        if api_key.is_none() {
            log_warn!(
                "OMDB_API_KEY not found in environment variables or .env file - IMDb queries will be limited"
            );
        }

        Self { client, api_key }
    }

    /// Query IMDb information by title or IMDb ID
    /// If the query is not an IMDb ID and title search fails, attempts a search
    pub async fn query_imdb_info(&self, query: &str) -> Result<String> {
        log_debug!("Querying IMDb info for: {}", query);

        if let Some(api_key) = &self.api_key {
            // First, try direct lookup (by IMDb ID or exact title)
            let search_param = if query.starts_with("tt") && query.len() >= 9 {
                format!("i={}", query) // IMDb ID format
            } else {
                format!("t={}", urlencoding::encode(query)) // Title search
            };

            let url = format!(
                "http://www.omdbapi.com/?{}&apikey={}&plot=full",
                search_param, api_key
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Ok(format!(
                    "IMDb Query Failed for: {}\nHTTP Status: {}\n",
                    query,
                    response.status()
                ));
            }

            let imdb_data: ImdbResponse = response.json().await?;

            if imdb_data.response == "True" {
                Ok(self.format_imdb_info(&imdb_data))
            } else {
                // If direct lookup failed and it's not an IMDb ID, try search
                if !query.starts_with("tt") {
                    log_debug!(
                        "Direct lookup failed for '{}', attempting fuzzy search",
                        query
                    );
                    match self.search_and_get_first_result(query).await {
                        Ok(result) => {
                            log_debug!("Search successful for '{}'", query);
                            Ok(result)
                        }
                        Err(search_err) => {
                            log_debug!("Search also failed for '{}': {}", query, search_err);
                            // Try alternative search approaches for non-English titles
                            Ok(format!(
                                "IMDb Information Not Found for: {}\n{}\n\
                                Note: For non-English titles, try using the English title or IMDb ID (e.g., tt1234567-IMDB)\n\
                                Use '<title>-IMDBSEARCH' for broader search results.\n",
                                query,
                                imdb_data
                                    .error
                                    .unwrap_or_else(|| "Movie not found!".to_string())
                            ))
                        }
                    }
                } else {
                    Ok(format!(
                        "IMDb Information Not Found for: {}\n{}\n",
                        query,
                        imdb_data
                            .error
                            .unwrap_or_else(|| "Movie not found!".to_string())
                    ))
                }
            }
        } else {
            Ok(format!(
                "IMDb Query Failed for: {}\nOMDB API key not configured.\n\
                 To enable IMDb queries, set the OMDB_API_KEY environment variable\n\
                 or add it to a .env file in the project root.\n\
                 You can get a free API key from: http://www.omdbapi.com/apikey.aspx\n",
                query
            ))
        }
    }

    /// Search IMDb and get detailed info for the first result
    async fn search_and_get_first_result(&self, query: &str) -> Result<String> {
        log_debug!("Searching IMDb for first result: {}", query);

        if let Some(api_key) = &self.api_key {
            let url = format!(
                "http://www.omdbapi.com/?s={}&apikey={}",
                urlencoding::encode(query),
                api_key
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Search request failed: {}",
                    response.status()
                ));
            }

            let search_data: ImdbSearchResponse = response.json().await?;

            if search_data.response == "True"
                && let Some(results) = search_data.search
                && let Some(first_result) = results.first()
            {
                // Get detailed info for the first search result using direct API call
                log_debug!(
                    "Found search result, getting details for: {}",
                    first_result.imdb_id
                );
                return self.get_movie_details_by_id(&first_result.imdb_id).await;
            }

            Err(anyhow::anyhow!("No search results found"))
        } else {
            Err(anyhow::anyhow!("No API key configured"))
        }
    }

    /// Get detailed movie information by IMDb ID (direct API call)
    async fn get_movie_details_by_id(&self, imdb_id: &str) -> Result<String> {
        log_debug!("Getting movie details for ID: {}", imdb_id);

        if let Some(api_key) = &self.api_key {
            let url = format!(
                "http://www.omdbapi.com/?i={}&apikey={}&plot=full",
                imdb_id, api_key
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!("Request failed: {}", response.status()));
            }

            let imdb_data: ImdbResponse = response.json().await?;

            if imdb_data.response == "True" {
                Ok(self.format_imdb_info(&imdb_data))
            } else {
                Err(anyhow::anyhow!(
                    "Movie details not found: {}",
                    imdb_data
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                ))
            }
        } else {
            Err(anyhow::anyhow!("No API key configured"))
        }
    }

    /// Search IMDb for movies/TV shows by title
    pub async fn search_imdb(&self, query: &str, limit: usize) -> Result<String> {
        log_debug!("Searching IMDb for: {}", query);

        if let Some(api_key) = &self.api_key {
            let url = format!(
                "http://www.omdbapi.com/?s={}&apikey={}",
                urlencoding::encode(query),
                api_key
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Ok(format!(
                    "IMDb Search Failed for: {}\nHTTP Status: {}\n",
                    query,
                    response.status()
                ));
            }

            let search_data: ImdbSearchResponse = response.json().await?;

            if search_data.response == "True" {
                if let Some(results) = search_data.search {
                    let limited_results: Vec<&ImdbSearchResult> =
                        results.iter().take(limit).collect();
                    Ok(self.format_search_results(query, &limited_results))
                } else {
                    Ok(format!("No IMDb search results found for: {}\n", query))
                }
            } else {
                Ok(format!(
                    "IMDb Search Failed for: {}\n{}\n",
                    query,
                    search_data
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                ))
            }
        } else {
            Ok(format!(
                "IMDb Search Failed for: {}\nOMDB API key not configured.\n\
                 To enable IMDb searches, set the OMDB_API_KEY environment variable\n\
                 or add it to a .env file in the project root.\n\
                 You can get a free API key from: http://www.omdbapi.com/apikey.aspx\n",
                query
            ))
        }
    }

    /// Format IMDb information for WHOIS display
    fn format_imdb_info(&self, imdb: &ImdbResponse) -> String {
        let mut output = String::new();

        if let Some(title) = &imdb.title {
            output.push_str(&format!("IMDb Information for: {}\n", title));
        } else {
            output.push_str("IMDb Information\n");
        }
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        if let Some(imdb_id) = &imdb.imdb_id {
            output.push_str(&format!("imdb-id: {}\n", imdb_id));
        }

        if let Some(title) = &imdb.title {
            output.push_str(&format!("title: {}\n", title));
        }

        if let Some(year) = &imdb.year {
            output.push_str(&format!("year: {}\n", year));
        }

        if let Some(content_type) = &imdb.content_type {
            output.push_str(&format!("type: {}\n", content_type));
        }

        if let Some(rated) = &imdb.rated {
            output.push_str(&format!("rated: {}\n", rated));
        }

        if let Some(runtime) = &imdb.runtime {
            output.push_str(&format!("runtime: {}\n", runtime));
        }

        if let Some(genre) = &imdb.genre {
            output.push_str(&format!("genre: {}\n", genre));
        }

        if let Some(director) = &imdb.director {
            output.push_str(&format!("director: {}\n", director));
        }

        if let Some(writer) = &imdb.writer {
            output.push_str(&format!("writer: {}\n", writer));
        }

        if let Some(actors) = &imdb.actors {
            output.push_str(&format!("actors: {}\n", actors));
        }

        if let Some(language) = &imdb.language {
            output.push_str(&format!("language: {}\n", language));
        }

        if let Some(country) = &imdb.country {
            output.push_str(&format!("country: {}\n", country));
        }

        if let Some(released) = &imdb.released {
            output.push_str(&format!("released: {}\n", released));
        }

        if let Some(imdb_rating) = &imdb.imdb_rating {
            output.push_str(&format!("imdb-rating: {}/10\n", imdb_rating));
        }

        if let Some(imdb_votes) = &imdb.imdb_votes {
            output.push_str(&format!("imdb-votes: {}\n", imdb_votes));
        }

        if let Some(metascore) = &imdb.metascore {
            output.push_str(&format!("metascore: {}/100\n", metascore));
        }

        if let Some(ratings) = &imdb.ratings {
            for rating in ratings {
                output.push_str(&format!(
                    "rating-{}: {}\n",
                    rating.source.to_lowercase().replace(' ', "-"),
                    rating.value
                ));
            }
        }

        if let Some(box_office) = &imdb.box_office {
            output.push_str(&format!("box-office: {}\n", box_office));
        }

        if let Some(awards) = &imdb.awards
            && awards != "N/A"
        {
            output.push_str(&format!("awards: {}\n", awards));
        }

        if let Some(production) = &imdb.production
            && production != "N/A"
        {
            output.push_str(&format!("production: {}\n", production));
        }

        if let Some(website) = &imdb.website
            && website != "N/A"
        {
            output.push_str(&format!("website: {}\n", website));
        }

        if let Some(total_seasons) = &imdb.total_seasons {
            output.push_str(&format!("total-seasons: {}\n", total_seasons));
        }

        if let Some(plot) = &imdb.plot
            && plot != "N/A"
        {
            output.push_str(&format!(
                "plot: {}\n",
                plot.replace("\r\n", " ").replace('\n', " ")
            ));
        }

        if let Some(imdb_id) = &imdb.imdb_id {
            output.push_str(&format!(
                "imdb-url: https://www.imdb.com/title/{}/\n",
                imdb_id
            ));
        }

        output
    }

    /// Format search results for WHOIS display
    fn format_search_results(&self, query: &str, results: &[&ImdbSearchResult]) -> String {
        let mut output = String::new();

        output.push_str(&format!("IMDb Search Results for: {}\n", query));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');
        output.push_str(&format!("Found {} titles:\n\n", results.len()));

        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("{}. Title Information\n", i + 1));
            output.push_str("-".repeat(25).as_str());
            output.push('\n');

            output.push_str(&format!("imdb-id: {}\n", result.imdb_id));
            output.push_str(&format!("title: {}\n", result.title));
            output.push_str(&format!("year: {}\n", result.year));
            output.push_str(&format!("type: {}\n", result.content_type));
            output.push_str(&format!(
                "imdb-url: https://www.imdb.com/title/{}/\n",
                result.imdb_id
            ));
            output.push('\n');
        }

        output.push_str(&format!(
            "% Use '{}-IMDB' to get detailed information for a specific title\n",
            results[0].imdb_id
        ));
        output.push_str("% Search limited to top 10 results\n");

        output
    }

    /// Check if a query string is an IMDb query
    pub fn is_imdb_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-IMDB")
    }

    /// Check if a query string is an IMDb search query
    pub fn is_imdb_search_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-IMDBSEARCH")
    }

    /// Parse IMDb query to extract the search term or IMDb ID
    pub fn parse_imdb_query(query: &str) -> Option<String> {
        if !Self::is_imdb_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 5]; // Remove "-IMDB"
        Some(clean_query.to_string())
    }

    /// Parse IMDb search query
    pub fn parse_imdb_search_query(query: &str) -> Option<String> {
        if !Self::is_imdb_search_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 11]; // Remove "-IMDBSEARCH"
        Some(clean_query.to_string())
    }
}

/// Process IMDb query with -IMDB suffix
pub async fn process_imdb_query(query: &str) -> Result<String> {
    let imdb_service = ImdbService::new();

    if let Some(imdb_query) = ImdbService::parse_imdb_query(query) {
        log_debug!("Processing IMDb query for: {}", imdb_query);
        imdb_service.query_imdb_info(&imdb_query).await
    } else {
        log_error!("Invalid IMDb query format: {}", query);
        Ok(format!(
            "Invalid IMDb query format. Use: <title_or_imdb_id>-IMDB\nExample: Inception-IMDB or tt1375666-IMDB\nQuery: {}\n",
            query
        ))
    }
}

/// Process IMDb search query with -IMDBSEARCH suffix
pub async fn process_imdb_search_query(query: &str) -> Result<String> {
    let imdb_service = ImdbService::new();

    if let Some(search_query) = ImdbService::parse_imdb_search_query(query) {
        log_debug!("Processing IMDb search query for: {}", search_query);

        if search_query.is_empty() {
            return Ok(
                "Invalid IMDb search query. Please provide a search term.\nExample: Batman-IMDBSEARCH\n".to_string()
            );
        }

        // Search for titles with a limit of 10 results
        imdb_service.search_imdb(&search_query, 10).await
    } else {
        log_error!("Invalid IMDb search query format: {}", query);
        Ok(format!(
            "Invalid IMDb search query format. Use: <search_term>-IMDBSEARCH\nExample: Batman-IMDBSEARCH\nQuery: {}\n",
            query
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imdb_query_detection() {
        assert!(ImdbService::is_imdb_query("Inception-IMDB"));
        assert!(ImdbService::is_imdb_query("tt1375666-IMDB"));
        assert!(ImdbService::is_imdb_query("The Matrix-IMDB"));

        assert!(!ImdbService::is_imdb_query("Inception"));
        assert!(!ImdbService::is_imdb_query("example.com-SSL"));
        assert!(!ImdbService::is_imdb_query("IMDB-Inception"));
    }

    #[test]
    fn test_imdb_search_query_detection() {
        assert!(ImdbService::is_imdb_search_query("Batman-IMDBSEARCH"));
        assert!(ImdbService::is_imdb_search_query("Star Wars-IMDBSEARCH"));
        assert!(ImdbService::is_imdb_search_query("Marvel-IMDBSEARCH"));

        assert!(!ImdbService::is_imdb_search_query("Batman"));
        assert!(!ImdbService::is_imdb_search_query("Batman-IMDB"));
        assert!(!ImdbService::is_imdb_search_query("IMDBSEARCH-Batman"));
    }

    #[test]
    fn test_imdb_query_parsing() {
        assert_eq!(
            ImdbService::parse_imdb_query("Inception-IMDB"),
            Some("Inception".to_string())
        );

        assert_eq!(
            ImdbService::parse_imdb_query("tt1375666-IMDB"),
            Some("tt1375666".to_string())
        );

        assert_eq!(ImdbService::parse_imdb_query("Inception"), None);
    }

    #[test]
    fn test_imdb_search_query_parsing() {
        assert_eq!(
            ImdbService::parse_imdb_search_query("Batman-IMDBSEARCH"),
            Some("Batman".to_string())
        );

        assert_eq!(
            ImdbService::parse_imdb_search_query("Star Wars-IMDBSEARCH"),
            Some("Star Wars".to_string())
        );

        assert_eq!(ImdbService::parse_imdb_search_query("Batman"), None);
    }

    #[tokio::test]
    async fn test_imdb_service_creation() {
        let service = ImdbService::new();
        // Just test that creation doesn't panic
        // The client is properly configured with timeout during creation
        assert!(service.api_key.is_none() || service.api_key.is_some());
    }
}
