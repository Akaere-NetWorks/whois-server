/*
 * WHOIS Server with DN42 Support - Pixiv Image Proxy
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

use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use reqwest::Client;
use tracing::{debug, error, warn};

/// Pixiv image reverse proxy handler
/// Proxies requests to i.pximg.net with proper headers to bypass restrictions
pub async fn proxy_pixiv_image(Path(path): Path<String>) -> Response {
    debug!("Pixiv proxy request for path: {}", path);

    // Construct the original Pixiv URL
    let pixiv_url = format!("https://i.pximg.net/{}", path);
    debug!("Proxying to: {}", pixiv_url);

    // Create HTTP client
    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create HTTP client: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create HTTP client",
            )
                .into_response();
        }
    };

    // Make request to Pixiv with proper User-Agent
    let response = match client
        .get(&pixiv_url)
        .header(
            reqwest::header::USER_AGENT,
            "TelegramBot (like TwitterBot)",
        )
        .header(reqwest::header::REFERER, "https://www.pixiv.net/")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Failed to fetch image from Pixiv: {}", e);
            return (StatusCode::BAD_GATEWAY, "Failed to fetch image from Pixiv")
                .into_response();
        }
    };

    let status = response.status();
    debug!("Pixiv response status: {}", status);

    // Check if the request was successful
    if !status.is_success() {
        warn!("Pixiv returned non-success status: {}", status);
        return (
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            format!("Pixiv returned status: {}", status),
        )
            .into_response();
    }

    // Extract content type from reqwest response
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    debug!("Content-Type: {}", content_type);

    // Get the image bytes
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read response body: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read response body",
            )
                .into_response();
        }
    };

    debug!("Successfully proxied {} bytes", bytes.len());

    // Build response headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        content_type.parse().unwrap_or_else(|_| {
            header::HeaderValue::from_static("application/octet-stream")
        }),
    );
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, max-age=86400"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_static("*"),
    );

    (headers, bytes).into_response()
}

/// Health check endpoint for Pixiv proxy
pub async fn proxy_health() -> Response {
    (StatusCode::OK, "Pixiv Proxy OK").into_response()
}
