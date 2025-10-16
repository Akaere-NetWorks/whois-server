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
use tracing::{ debug, error, warn };

/// Steam API response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAppDetails {
    pub success: bool,
    pub data: Option<SteamAppData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAppData {
    pub name: String,
    pub steam_appid: u32,
    #[serde(rename = "type")]
    pub app_type: String,
    pub is_free: bool,
    pub detailed_description: Option<String>,
    pub short_description: Option<String>,
    pub supported_languages: Option<String>,
    pub developers: Option<Vec<String>>,
    pub publishers: Option<Vec<String>>,
    pub platforms: Option<SteamPlatforms>,
    pub categories: Option<Vec<SteamCategory>>,
    pub genres: Option<Vec<SteamGenre>>,
    pub release_date: Option<SteamReleaseDate>,
    pub price_overview: Option<SteamPriceOverview>,
    pub website: Option<String>,
    pub metacritic: Option<SteamMetacritic>,
    pub recommendations: Option<SteamRecommendations>,
    pub achievements: Option<SteamAchievements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamPlatforms {
    pub windows: bool,
    pub mac: bool,
    pub linux: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamCategory {
    pub id: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamGenre {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamReleaseDate {
    pub coming_soon: bool,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamPriceOverview {
    pub currency: String,
    pub initial: u32,
    pub r#final: u32,
    pub discount_percent: u32,
    pub initial_formatted: String,
    pub final_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamMetacritic {
    pub score: u32,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamRecommendations {
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAchievements {
    pub total: u32,
}

/// Steam user profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamUserResponse {
    pub response: SteamUserResponseData,
}

/// Steam search API response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SteamSearchResponse {
    pub success: bool,
    pub data: Option<SteamSearchData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SteamSearchData {
    pub query: String,
    pub results: Vec<SteamSearchResult>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SteamSearchResult {
    pub appid: u32,
    pub name: String,
    pub icon: String,
    pub logo: String,
    #[serde(rename = "type")]
    pub app_type: String,
    pub platforms: Option<SteamSearchPlatforms>,
    pub coming_soon: Option<bool>,
    pub price: Option<String>,
    pub metascore: Option<u32>,
    pub reviewstooltip: Option<String>,
    pub streamingvideo: Option<bool>,
    pub discount_block: Option<String>,
    pub early_access: Option<bool>,
    pub vr_support: Option<SteamVRSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SteamSearchPlatforms {
    pub windows: bool,
    pub mac: bool,
    pub linux: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SteamVRSupport {
    pub vrhmd: Option<bool>,
    pub vrhmd_only: Option<bool>,
}

/// Steam app list API response (for comprehensive search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAppListResponse {
    pub applist: SteamAppList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAppList {
    pub apps: Vec<SteamAppListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAppListItem {
    pub appid: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamUserResponseData {
    pub players: Vec<SteamUserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamUserProfile {
    pub steamid: String,
    pub communityvisibilitystate: u32,
    pub profilestate: u32,
    pub personaname: String,
    pub profileurl: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub personastate: u32,
    pub realname: Option<String>,
    pub primaryclanid: Option<String>,
    pub timecreated: Option<u64>,
    pub personastateflags: Option<u32>,
    pub loccountrycode: Option<String>,
    pub locstatecode: Option<String>,
    pub loccityid: Option<u32>,
}

/// Steam service for game and user information queries
///
/// To enable Steam user profile queries, set the STEAM_API_KEY environment variable
/// or add it to a .env file in the project root:
/// ```
/// STEAM_API_KEY=your_steam_api_key_here
/// ```
/// You can get an API key from: https://steamcommunity.com/dev/apikey
pub struct SteamService {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl Default for SteamService {
    fn default() -> Self {
        Self::new()
    }
}

impl SteamService {
    /// Create a new Steam service
    pub fn new() -> Self {
        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("WhoisServer/1.0 Steam API Client")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // Try to load .env file first (ignore errors if file doesn't exist)
        let _ = dotenv::dotenv();

        // Try to get API key from environment variable (including from .env file)
        let api_key = std::env::var("STEAM_API_KEY").ok();
        if api_key.is_none() {
            warn!(
                "STEAM_API_KEY not found in environment variables or .env file - user profile queries will be limited"
            );
        }

        Self { client, api_key }
    }

    /// Query Steam application information
    pub async fn query_app_info(&self, app_id: u32) -> Result<String> {
        debug!("Querying Steam app info for ID: {}", app_id);

        let url =
            format!("https://store.steampowered.com/api/appdetails?appids={}&l=english", app_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(
                format!(
                    "Steam App Query Failed for ID: {}\nHTTP Status: {}\n",
                    app_id,
                    response.status()
                )
            );
        }

        let text = response.text().await?;

        // Steam API returns a nested JSON structure with app ID as key
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&text);

        match parsed {
            Ok(json) => {
                if let Some(app_data) = json.get(&app_id.to_string()) {
                    let app_details: Result<SteamAppDetails, _> = serde_json::from_value(
                        app_data.clone()
                    );
                    match app_details {
                        Ok(details) => {
                            if details.success {
                                if let Some(data) = details.data {
                                    Ok(self.format_app_info(&data))
                                } else {
                                    Ok(
                                        format!("Steam App Not Found for ID: {}\nThe application may not exist or may be private.\n", app_id)
                                    )
                                }
                            } else {
                                Ok(
                                    format!("Steam App Query Failed for ID: {}\nApplication data not available.\n", app_id)
                                )
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse Steam app data for {}: {}", app_id, e);
                            Ok(
                                format!(
                                    "Steam App Query Failed for ID: {}\nData parsing error: {}\n",
                                    app_id,
                                    e
                                )
                            )
                        }
                    }
                } else {
                    Ok(
                        format!("Steam App Not Found for ID: {}\nNo data returned from Steam API.\n", app_id)
                    )
                }
            }
            Err(e) => {
                error!("Failed to parse Steam API response for app {}: {}", app_id, e);
                Ok(
                    format!(
                        "Steam App Query Failed for ID: {}\nAPI response parsing error: {}\n",
                        app_id,
                        e
                    )
                )
            }
        }
    }

    /// Query Steam user profile information
    pub async fn query_user_info(&self, steam_id: &str) -> Result<String> {
        debug!("Querying Steam user info for ID: {}", steam_id);

        if let Some(api_key) = &self.api_key {
            let url = format!(
                "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
                api_key,
                steam_id
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Ok(
                    format!(
                        "Steam User Query Failed for ID: {}\nHTTP Status: {}\n",
                        steam_id,
                        response.status()
                    )
                );
            }

            let user_response: Result<SteamUserResponse, _> = response.json().await;

            match user_response {
                Ok(response) => {
                    if let Some(profile) = response.response.players.first() {
                        Ok(self.format_user_info(profile))
                    } else {
                        Ok(
                            format!("Steam User Not Found for ID: {}\nProfile may not exist or may be private.\n", steam_id)
                        )
                    }
                }
                Err(e) => {
                    error!("Failed to parse Steam user data for {}: {}", steam_id, e);
                    Ok(
                        format!(
                            "Steam User Query Failed for ID: {}\nData parsing error: {}\n",
                            steam_id,
                            e
                        )
                    )
                }
            }
        } else {
            Ok(
                format!("Steam User Query Failed for ID: {}\nSteam API key not configured.\n\
                 To enable user profile queries, set the STEAM_API_KEY environment variable\n\
                 or add it to a .env file in the project root.\n\
                 You can get an API key from: https://steamcommunity.com/dev/apikey\n", steam_id)
            )
        }
    }

    /// Search Steam games by name (fuzzy search)
    pub async fn search_games(&self, query: &str, limit: usize) -> Result<String> {
        debug!("Searching Steam games for query: {}", query);

        // First try the Steam store search API (unofficial but works well)
        match self.search_games_via_store_api(query, limit).await {
            Ok(results) => Ok(results),
            Err(store_error) => {
                debug!("Store API search failed, trying app list fallback: {}", store_error);
                // Fallback to app list search
                self.search_games_via_app_list(query, limit).await
            }
        }
    }

    /// Search games using Steam store search API (more detailed results)
    async fn search_games_via_store_api(&self, query: &str, limit: usize) -> Result<String> {
        // Use Steam store search endpoint
        let url = format!(
            "https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US",
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("Steam store search API returned status: {}", response.status())
            );
        }

        let search_data: serde_json::Value = response.json().await?;

        // Parse the search results
        if let Some(items) = search_data.get("items").and_then(|v| v.as_array()) {
            let mut results = Vec::new();

            for item in items.iter().take(limit) {
                if
                    let (Some(id), Some(name)) = (
                        item.get("id").and_then(|v| v.as_u64()),
                        item.get("name").and_then(|v| v.as_str()),
                    )
                {
                    let app_type = item
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("game");
                    let price_info = self.extract_price_info_from_search(item);
                    let platforms = self.extract_platforms_from_search(item);
                    let coming_soon = item
                        .get("coming_soon")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    results.push((
                        id as u32,
                        name.to_string(),
                        app_type.to_string(),
                        price_info,
                        platforms,
                        coming_soon,
                    ));
                }
            }

            if results.is_empty() {
                Ok(format!("No Steam games found matching: {}\n", query))
            } else {
                Ok(self.format_search_results(query, &results))
            }
        } else {
            Err(anyhow::anyhow!("Invalid response format from Steam store search API"))
        }
    }

    /// Search games using Steam app list API (fallback method)
    async fn search_games_via_app_list(&self, query: &str, limit: usize) -> Result<String> {
        // Get the complete app list from Steam API
        let url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("Steam app list API returned status: {}", response.status())
            );
        }

        let app_list_response: SteamAppListResponse = response.json().await?;

        // Perform fuzzy search on app names
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        for app in &app_list_response.applist.apps {
            let name_lower = app.name.to_lowercase();

            // Simple fuzzy matching: exact match, starts with, or contains
            let score = if name_lower == query_lower {
                100 // Exact match
            } else if name_lower.starts_with(&query_lower) {
                50 // Starts with
            } else if name_lower.contains(&query_lower) {
                25 // Contains
            } else {
                0 // No match
            };

            if score > 0 {
                matches.push((app.appid, app.name.clone(), score));
            }
        }

        // Sort by score (descending) and take top results
        matches.sort_by(|a, b| b.2.cmp(&a.2));
        matches.truncate(limit);

        if matches.is_empty() {
            Ok(format!("No Steam games found matching: {}\n", query))
        } else {
            // Convert to the format expected by format_search_results
            let results: Vec<(u32, String, String, Option<String>, String, bool)> = matches
                .into_iter()
                .map(|(id, name, _score)| (
                    id,
                    name,
                    "app".to_string(),
                    None,
                    "N/A".to_string(),
                    false,
                ))
                .collect();

            Ok(self.format_search_results(query, &results))
        }
    }

    /// Extract price information from search result item
    fn extract_price_info_from_search(&self, item: &serde_json::Value) -> Option<String> {
        if let Some(price_obj) = item.get("price") {
            // Handle free games
            if let Some(currency) = price_obj.get("currency").and_then(|v| v.as_str()) {
                if currency == "USD" {
                    if let Some(final_price) = price_obj.get("final").and_then(|v| v.as_u64()) {
                        if final_price == 0 {
                            return Some("Free".to_string());
                        }
                    }
                }
            }

            // Handle priced games with potential discounts
            if
                let (
                    Some(final_formatted),
                    Some(initial),
                    Some(final_price),
                    Some(discount_percent),
                ) = (
                    price_obj.get("final_formatted").and_then(|v| v.as_str()),
                    price_obj.get("initial").and_then(|v| v.as_u64()),
                    price_obj.get("final").and_then(|v| v.as_u64()),
                    price_obj.get("discount_percent").and_then(|v| v.as_u64()),
                )
            {
                if discount_percent > 0 && initial > final_price {
                    // Has discount
                    return Some(format!("{} ({}%↓)", final_formatted, discount_percent));
                } else {
                    // No discount
                    return Some(final_formatted.to_string());
                }
            } else if
                let Some(final_formatted) = price_obj
                    .get("final_formatted")
                    .and_then(|v| v.as_str())
            {
                // Simple price without discount info
                return Some(final_formatted.to_string());
            }
        }

        None
    }

    /// Extract platform information from search result item
    fn extract_platforms_from_search(&self, item: &serde_json::Value) -> String {
        let mut platforms = Vec::new();

        if let Some(platform_info) = item.get("platforms") {
            if
                platform_info
                    .get("windows")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                platforms.push("Windows");
            }
            if
                platform_info
                    .get("mac")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                platforms.push("macOS");
            }
            if
                platform_info
                    .get("linux")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                platforms.push("Linux");
            }
        }

        if platforms.is_empty() {
            "N/A".to_string()
        } else {
            platforms.join(", ")
        }
    }

    /// Format search results for WHOIS display
    fn format_search_results(
        &self,
        query: &str,
        results: &[(u32, String, String, Option<String>, String, bool)]
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!("Steam Game Search Results for: {}\n", query));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');
        output.push_str(&format!("Found {} games:\n\n", results.len()));

        for (i, (app_id, name, app_type, price, platforms, coming_soon)) in results
            .iter()
            .enumerate() {
            output.push_str(&format!("{}. Game Information\n", i + 1));
            output.push_str("-".repeat(25).as_str());
            output.push('\n');

            output.push_str(&format!("app-id: {}\n", app_id));
            output.push_str(&format!("name: {}\n", name));
            output.push_str(&format!("type: {}\n", app_type));

            if let Some(price_str) = price {
                output.push_str(&format!("price: {}\n", price_str));
            }

            output.push_str(&format!("platforms: {}\n", platforms));

            if *coming_soon {
                output.push_str("status: Coming Soon\n");
            }

            output.push_str(
                &format!("steam-url: https://store.steampowered.com/app/{}/\n", app_id)
            );
            output.push('\n');
        }

        output.push_str(
            &format!(
                "% Use '{}-STEAM' to get detailed information for a specific game\n",
                results[0].0
            )
        );
        output.push_str("% Search limited to top 10 results\n");

        output
    }

    /// Format Steam application information for WHOIS display
    fn format_app_info(&self, app: &SteamAppData) -> String {
        let mut output = String::new();

        output.push_str(&format!("Steam Application Information for ID: {}\n", app.steam_appid));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("app-id: {}\n", app.steam_appid));
        output.push_str(&format!("name: {}\n", app.name));
        output.push_str(&format!("type: {}\n", app.app_type));
        output.push_str(&format!("is-free: {}\n", app.is_free));

        if let Some(developers) = &app.developers {
            if !developers.is_empty() {
                output.push_str(&format!("developers: {}\n", developers.join(", ")));
            }
        }

        if let Some(publishers) = &app.publishers {
            if !publishers.is_empty() {
                output.push_str(&format!("publishers: {}\n", publishers.join(", ")));
            }
        }

        if let Some(release_date) = &app.release_date {
            output.push_str(&format!("release-date: {}\n", release_date.date));
            output.push_str(&format!("coming-soon: {}\n", release_date.coming_soon));
        }

        if let Some(platforms) = &app.platforms {
            let mut platform_list = Vec::new();
            if platforms.windows {
                platform_list.push("Windows");
            }
            if platforms.mac {
                platform_list.push("macOS");
            }
            if platforms.linux {
                platform_list.push("Linux");
            }
            output.push_str(&format!("platforms: {}\n", platform_list.join(", ")));
        }

        if let Some(price) = &app.price_overview {
            if price.discount_percent > 0 {
                // Has discount - show discounted price with percentage
                output.push_str(
                    &format!("price: {} ({}%↓)\n", price.final_formatted, price.discount_percent)
                );
                output.push_str(&format!("original-price: {}\n", price.initial_formatted));
            } else {
                // No discount - show regular price
                output.push_str(&format!("price: {}\n", price.final_formatted));
            }
            output.push_str(&format!("currency: {}\n", price.currency));
        }

        if let Some(metacritic) = &app.metacritic {
            output.push_str(&format!("metacritic-score: {}\n", metacritic.score));
            output.push_str(&format!("metacritic-url: {}\n", metacritic.url));
        }

        if let Some(recommendations) = &app.recommendations {
            output.push_str(&format!("recommendations: {}\n", recommendations.total));
        }

        if let Some(achievements) = &app.achievements {
            output.push_str(&format!("achievements: {}\n", achievements.total));
        }

        if let Some(website) = &app.website {
            output.push_str(&format!("website: {}\n", website));
        }

        if let Some(genres) = &app.genres {
            if !genres.is_empty() {
                let genre_names: Vec<&str> = genres
                    .iter()
                    .map(|g| g.description.as_str())
                    .collect();
                output.push_str(&format!("genres: {}\n", genre_names.join(", ")));
            }
        }

        if let Some(categories) = &app.categories {
            if !categories.is_empty() {
                let category_names: Vec<&str> = categories
                    .iter()
                    .map(|c| c.description.as_str())
                    .collect();
                output.push_str(&format!("categories: {}\n", category_names.join(", ")));
            }
        }

        if let Some(languages) = &app.supported_languages {
            output.push_str(
                &format!(
                    "supported-languages: {}\n",
                    languages.replace("<br>", ", ").replace("<strong>", "").replace("</strong>", "")
                )
            );
        }

        if let Some(description) = &app.short_description {
            output.push_str(
                &format!("description: {}\n", description.replace("\r\n", " ").replace('\n', " "))
            );
        }

        output.push_str(
            &format!("steam-url: https://store.steampowered.com/app/{}/\n", app.steam_appid)
        );

        output
    }

    /// Format Steam user profile information for WHOIS display
    fn format_user_info(&self, profile: &SteamUserProfile) -> String {
        let mut output = String::new();

        output.push_str(&format!("Steam User Profile Information for ID: {}\n", profile.steamid));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("steamid: {}\n", profile.steamid));
        output.push_str(&format!("personaname: {}\n", profile.personaname));

        if let Some(realname) = &profile.realname {
            output.push_str(&format!("realname: {}\n", realname));
        }

        output.push_str(&format!("profileurl: {}\n", profile.profileurl));

        // Community visibility state
        let visibility = match profile.communityvisibilitystate {
            1 => "Private",
            3 => "Friends Only",
            _ => "Public",
        };
        output.push_str(&format!("visibility: {}\n", visibility));

        // Profile state
        let profile_state = match profile.profilestate {
            0 => "Not Configured",
            1 => "Configured",
            _ => "Unknown",
        };
        output.push_str(&format!("profile-state: {}\n", profile_state));

        // Persona state (online status)
        let persona_state = match profile.personastate {
            0 => "Offline",
            1 => "Online",
            2 => "Busy",
            3 => "Away",
            4 => "Snooze",
            5 => "Looking to trade",
            6 => "Looking to play",
            _ => "Unknown",
        };
        output.push_str(&format!("status: {}\n", persona_state));

        if let Some(created) = profile.timecreated {
            let datetime = chrono::DateTime::from_timestamp(created as i64, 0).unwrap_or_default();
            output.push_str(
                &format!("created: {} ({})\n", datetime.format("%Y-%m-%d %H:%M:%S UTC"), created)
            );
        }

        if let Some(country) = &profile.loccountrycode {
            output.push_str(&format!("country: {}\n", country));
        }

        if let Some(state) = &profile.locstatecode {
            output.push_str(&format!("state: {}\n", state));
        }

        if let Some(clan_id) = &profile.primaryclanid {
            output.push_str(&format!("primary-clan-id: {}\n", clan_id));
        }

        output.push_str(&format!("avatar: {}\n", profile.avatar));
        output.push_str(&format!("avatar-medium: {}\n", profile.avatarmedium));
        output.push_str(&format!("avatar-full: {}\n", profile.avatarfull));

        output
    }

    /// Check if a query string is a Steam query
    pub fn is_steam_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-STEAM")
    }

    /// Check if a query string is a Steam search query
    pub fn is_steam_search_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-STEAMSEARCH")
    }

    /// Parse Steam query to determine if it's an app ID or user ID
    pub fn parse_steam_query(query: &str) -> Option<String> {
        if !Self::is_steam_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 6]; // Remove "-STEAM"
        Some(clean_query.to_string())
    }

    /// Parse Steam search query
    pub fn parse_steam_search_query(query: &str) -> Option<String> {
        if !Self::is_steam_search_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 12]; // Remove "-STEAMSEARCH"
        Some(clean_query.to_string())
    }

    /// Determine if the query is likely an app ID (numeric) or user ID
    pub fn is_likely_app_id(query: &str) -> bool {
        // Steam App IDs are typically shorter numeric values (up to ~7 digits)
        // Steam User IDs are 17-digit numbers or custom URLs
        if let Ok(num) = query.parse::<u64>() {
            // App IDs are typically under 10 million
            num < 10_000_000
        } else {
            false
        }
    }
}

/// Process Steam query with -STEAM suffix
pub async fn process_steam_query(query: &str) -> Result<String> {
    let steam_service = SteamService::new();

    if let Some(steam_query) = SteamService::parse_steam_query(query) {
        debug!("Processing Steam query for: {}", steam_query);

        // Try to determine if this is an app ID or user ID
        if SteamService::is_likely_app_id(&steam_query) {
            // Try parsing as app ID first
            if let Ok(app_id) = steam_query.parse::<u32>() {
                debug!("Treating as Steam App ID: {}", app_id);
                return steam_service.query_app_info(app_id).await;
            }
        }

        // If not clearly an app ID, treat as user ID/username
        debug!("Treating as Steam User ID: {}", steam_query);

        // For custom URLs, we'd need to resolve them to Steam IDs first
        // For now, assume it's already a Steam ID
        steam_service.query_user_info(&steam_query).await
    } else {
        error!("Invalid Steam query format: {}", query);
        Ok(
            format!("Invalid Steam query format. Use: <app_id>-STEAM or <steam_id>-STEAM\nQuery: {}\n", query)
        )
    }
}

/// Process Steam search query with -STEAMSEARCH suffix
pub async fn process_steam_search_query(query: &str) -> Result<String> {
    let steam_service = SteamService::new();

    if let Some(search_query) = SteamService::parse_steam_search_query(query) {
        debug!("Processing Steam search query for: {}", search_query);

        if search_query.is_empty() {
            return Ok(
                format!(
                    "Invalid Steam search query. Please provide a search term.\nExample: Counter-Strike-STEAMSEARCH\n"
                )
            );
        }

        // Search for games with a limit of 10 results
        steam_service.search_games(&search_query, 10).await
    } else {
        error!("Invalid Steam search query format: {}", query);
        Ok(
            format!("Invalid Steam search query format. Use: <search_term>-STEAMSEARCH\nExample: Counter-Strike-STEAMSEARCH\nQuery: {}\n", query)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steam_query_detection() {
        assert!(SteamService::is_steam_query("730-STEAM"));
        assert!(SteamService::is_steam_query("76561198000000000-STEAM"));
        assert!(SteamService::is_steam_query("steam-STEAM"));

        assert!(!SteamService::is_steam_query("730"));
        assert!(!SteamService::is_steam_query("example.com-SSL"));
        assert!(!SteamService::is_steam_query("STEAM-730"));
    }

    #[test]
    fn test_steam_search_query_detection() {
        assert!(SteamService::is_steam_search_query("Counter-Strike-STEAMSEARCH"));
        assert!(SteamService::is_steam_search_query("dota-STEAMSEARCH"));
        assert!(SteamService::is_steam_search_query("Half Life-STEAMSEARCH"));

        assert!(!SteamService::is_steam_search_query("Counter-Strike"));
        assert!(!SteamService::is_steam_search_query("Counter-Strike-STEAM"));
        assert!(!SteamService::is_steam_search_query("STEAMSEARCH-Counter-Strike"));
    }

    #[test]
    fn test_steam_query_parsing() {
        assert_eq!(SteamService::parse_steam_query("730-STEAM"), Some("730".to_string()));

        assert_eq!(
            SteamService::parse_steam_query("76561198000000000-STEAM"),
            Some("76561198000000000".to_string())
        );

        assert_eq!(SteamService::parse_steam_query("730"), None);
    }

    #[test]
    fn test_steam_search_query_parsing() {
        assert_eq!(
            SteamService::parse_steam_search_query("Counter-Strike-STEAMSEARCH"),
            Some("Counter-Strike".to_string())
        );

        assert_eq!(
            SteamService::parse_steam_search_query("dota-STEAMSEARCH"),
            Some("dota".to_string())
        );

        assert_eq!(SteamService::parse_steam_search_query("Counter-Strike"), None);
    }

    #[test]
    fn test_app_id_detection() {
        assert!(SteamService::is_likely_app_id("730")); // CS2
        assert!(SteamService::is_likely_app_id("570")); // Dota 2
        assert!(SteamService::is_likely_app_id("1234567")); // Large but still app ID

        // Steam user IDs are 17 digits
        assert!(!SteamService::is_likely_app_id("76561198000000000"));
        assert!(!SteamService::is_likely_app_id("12345678901234567"));

        // Non-numeric should be treated as username/custom URL
        assert!(!SteamService::is_likely_app_id("username"));
    }

    #[tokio::test]
    async fn test_steam_service_creation() {
        let service = SteamService::new();
        // Just test that creation doesn't panic
        // The client is properly configured with timeout during creation
        assert!(service.api_key.is_none() || service.api_key.is_some());
    }
}
