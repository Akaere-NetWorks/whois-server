//! Data models for Pixiv API responses

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Pixiv API response wrapper
#[derive(Debug, Deserialize)]
pub struct PixivResponse<T> {
    pub error: Option<bool>,
    pub message: Option<String>,
    pub body: Option<T>,
}

/// Artwork information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artwork {
    pub id: i64,
    pub title: String,
    #[serde(rename = "type")]
    pub artwork_type: String, // "illust" or "manga"
    pub image_urls: ImageUrls,
    #[serde(rename = "caption")]
    pub description: Option<String>,
    pub restrict: i32,
    pub user: User,
    pub tags: Vec<Tag>,
    #[serde(rename = "tools")]
    pub tools: Option<Vec<String>>,
    #[serde(rename = "create_date")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "page_count")]
    pub page_count: i32,
    #[serde(rename = "width")]
    pub width: i32,
    #[serde(rename = "height")]
    pub height: i32,
    #[serde(rename = "sanity_level")]
    pub sanity_level: i32,
    #[serde(rename = "x_restrict")]
    pub x_restrict: i32,
    pub series: Option<Series>,
    #[serde(rename = "meta_single_page")]
    pub meta_single_page: MetaSinglePage,
    #[serde(rename = "meta_pages")]
    pub meta_pages: Vec<MetaPage>,
    #[serde(rename = "total_view")]
    pub total_view: Option<i32>,
    #[serde(rename = "total_bookmarks")]
    pub total_bookmarks: Option<i32>,
    #[serde(rename = "total_comments")]
    pub total_comments: Option<i32>,
    #[serde(rename = "is_bookmarked")]
    pub is_bookmarked: bool,
    #[serde(rename = "is_muted")]
    pub is_muted: bool,
    #[serde(rename = "visible")]
    pub visible: bool,
    #[serde(rename = "is_manga")]
    pub is_manga: bool,
}

/// Image URLs for different sizes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageUrls {
    #[serde(rename = "square_medium")]
    pub square_medium: String,
    pub medium: String,
    pub large: String,
}

/// Meta page information for manga/multi-page works
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaPage {
    #[serde(rename = "image_urls")]
    pub image_urls: ImageUrls,
    pub width: i32,
    pub height: i32,
}

/// Meta single page information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaSinglePage {
    #[serde(rename = "original_image_url")]
    pub original_image_url: Option<String>,
}

/// Tag information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    #[serde(rename = "translated_name")]
    pub translated_name: Option<String>,
}

/// User information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub account: String,
    #[serde(rename = "profile_image_urls")]
    pub profile_image_urls: ProfileImageUrls,
    #[serde(rename = "is_followed")]
    pub is_followed: bool,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
}

/// User profile details (from user/detail endpoint)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserProfile {
    pub user: User,
    #[serde(rename = "profile")]
    pub profile: Profile,
    #[serde(rename = "profile_publicity")]
    pub profile_publicity: ProfilePublicity,
    #[serde(rename = "workspace")]
    pub workspace: Workspace,
}

/// User profile
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    #[serde(rename = "webpage")]
    pub webpage: Option<String>,
    pub gender: Option<String>,
    pub birth: Option<String>,
    #[serde(rename = "birth_day")]
    pub birth_day: Option<i32>,
    #[serde(rename = "birth_month")]
    pub birth_month: Option<i32>,
    #[serde(rename = "birth_year")]
    pub birth_year: Option<i32>,
    #[serde(rename = "region")]
    pub region: Option<String>,
    #[serde(rename = "region_name")]
    pub region_name: Option<String>,
    #[serde(rename = "country_code")]
    pub country_code: Option<String>,
    #[serde(rename = "job")]
    pub job: Option<String>,
    #[serde(rename = "job_name")]
    pub job_name: Option<String>,
    #[serde(rename = "total_follow_users")]
    pub total_follow_users: Option<i32>,
    #[serde(rename = "total_mypixiv_users")]
    pub total_mypixiv_users: Option<i32>,
    #[serde(rename = "total_illusts")]
    pub total_illusts: Option<i32>,
    #[serde(rename = "total_manga")]
    pub total_manga: Option<i32>,
    #[serde(rename = "total_novels")]
    pub total_novels: Option<i32>,
    #[serde(rename = "total_bookmark_tags")]
    pub total_bookmark_tags: Option<i32>,
    #[serde(rename = "total_illust_bookmarks")]
    pub total_illust_bookmarks: Option<i32>,
    #[serde(rename = "total_illust_series")]
    pub total_illust_series: Option<i32>,
    #[serde(rename = "background_image_url")]
    pub background_image_url: Option<String>,
    #[serde(rename = "twitter_account")]
    pub twitter_account: Option<String>,
    #[serde(rename = "twitter_url")]
    pub twitter_url: Option<String>,
    #[serde(rename = "pawoo_url")]
    pub pawoo_url: Option<String>,
    #[serde(rename = "is_premium")]
    pub is_premium: bool,
    #[serde(rename = "is_using_custom_profile_image")]
    pub is_using_custom_profile_image: bool,
}

/// Profile publicity settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfilePublicity {
    #[serde(rename = "gender")]
    pub gender: bool,
    #[serde(rename = "region")]
    pub region: bool,
    #[serde(rename = "birth_day")]
    pub birth_day: bool,
    #[serde(rename = "birth_year")]
    pub birth_year: bool,
    #[serde(rename = "job")]
    pub job: bool,
    #[serde(rename = "pawoo")]
    pub pawoo: bool,
}

/// Workspace information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workspace {
    pub pc: Option<String>,
    pub monitor: Option<String>,
    pub tool: Option<String>,
    #[serde(rename = "scanner")]
    pub scanner: Option<String>,
    pub mouse: Option<String>,
    pub printer: Option<String>,
    pub desktop: Option<String>,
    pub music: Option<String>,
    pub desk: Option<String>,
    pub chair: Option<String>,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
    #[serde(rename = "tablet")]
    pub tablet: Option<String>,
}

/// Profile image URLs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileImageUrls {
    pub medium: String,
}

/// Series information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Series {
    pub id: i64,
    pub title: String,
}

/// Search results
#[derive(Debug, Deserialize)]
pub struct SearchResults {
    #[serde(rename = "illusts")]
    pub artworks: Vec<Artwork>,
    #[serde(rename = "manga")]
    pub manga: Vec<Artwork>,
    #[serde(rename = "novels")]
    pub novels: Option<Vec<serde_json::Value>>, // Novel support not implemented
    #[serde(rename = "next_url")]
    pub next_url: Option<String>,
    #[serde(rename = "search_span_limit")]
    pub search_span_limit: Option<i32>,
}

/// Ranking information
#[derive(Debug, Deserialize)]
pub struct RankingResults {
    #[serde(rename = "contents")]
    pub contents: Vec<RankingItem>,
    #[serde(rename = "mode")]
    pub mode: String,
    #[serde(rename = "next")]
    pub next: Option<RankingNext>,
}

/// Ranking item
#[derive(Debug, Clone, Deserialize)]
pub struct RankingItem {
    #[serde(rename = "illust_id")]
    pub illust_id: i64,
    pub title: String,
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "user_id")]
    pub user_id: i64,
    pub user_name: String,
    #[serde(rename = "profile_img")]
    pub profile_img: String,
    #[serde(rename = "width")]
    pub width: i32,
    #[serde(rename = "height")]
    pub height: i32,
    #[serde(rename = "tags")]
    pub tags: Vec<Tag>,
    #[serde(rename = "illust_upload_timestamp")]
    pub illust_upload_timestamp: i64,
    #[serde(rename = "date")]
    pub date: String,
    #[serde(rename = "ranking_date")]
    pub ranking_date: String,
}

/// Next page information for ranking
#[derive(Debug, Deserialize)]
pub struct RankingNext {
    pub rank: i32,
    pub content_ids: Option<Vec<String>>,
}

/// User artworks collection
#[derive(Debug, Deserialize)]
pub struct UserArtworks {
    #[serde(rename = "illusts")]
    pub artworks: Vec<Artwork>,
    #[serde(rename = "manga")]
    pub manga: Vec<Artwork>,
    #[serde(rename = "next_url")]
    pub next_url: Option<String>,
}

/// API request parameters
#[derive(Debug, Serialize)]
pub struct SearchParams {
    pub word: String,
    #[serde(rename = "search_target")]
    pub search_target: Option<String>, // "partial_match_for_tags", "exact_match_for_tags", etc.
    pub sort: Option<String>, // "date_desc", "date_asc", "popular_desc"
    pub filter: Option<String>, // "for_ios", "safe"
    pub offset: Option<i32>,
    pub include_translated_tag_results: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RankingParams {
    pub mode: String, // "daily", "weekly", "monthly", etc.
    pub filter: Option<String>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UserIllustParams {
    #[serde(rename = "user_id")]
    pub user_id: i64,
    #[serde(rename = "filter")]
    pub filter: Option<String>,
    #[serde(rename = "offset")]
    pub offset: Option<i32>,
    #[serde(rename = "type")]
    pub artwork_type: Option<String>, // "illust", "manga"
}