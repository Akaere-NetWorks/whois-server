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

use crate::core::query_processor::process_query;
use crate::core::{StatsState, analyze_query, get_stats_response};
use crate::web::json_formatter::{JsonFormatter, WhoisApiResponse};
use axum::{
    Router,
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use serde::Deserialize;
use std::time::Instant;
use tower_http::cors::CorsLayer;

#[derive(Debug, Deserialize)]
struct ApiQuery {
    q: String,
}

pub async fn run_web_server(
    stats: StatsState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(dashboard))
        .route("/docs", get(api_docs))
        .route("/api/openapi.json", get(openapi_spec))
        .route("/api/stats", get(get_stats_api))
        .route("/api/whois", get(whois_api_get))
        .route("/api/whois", post(whois_api_post))
        .route("/raw/:query", get(raw_whois_query))
        .layer(CorsLayer::permissive())
        .with_state(stats);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn dashboard() -> impl IntoResponse {
    // 读取 HTML 模板文件
    let html = include_str!("dashboard_template.html");
    Html(html)
}

async fn get_stats_api(State(stats): State<StatsState>) -> impl IntoResponse {
    match get_stats_response(&stats).await {
        response => Json(response),
    }
}

// GET /api/whois?q=query
async fn whois_api_get(
    State(stats): State<StatsState>,
    Query(params): Query<ApiQuery>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let query = params.q.trim();

    if query.is_empty() {
        let formatter = JsonFormatter::new();
        return Json(formatter.format_error(
            query,
            "Query parameter 'q' is required and cannot be empty",
            "unknown",
            start_time.elapsed().as_millis() as u64,
        ));
    }

    process_whois_query(query, stats, start_time).await
}

// POST /api/whois with JSON body: {"q": "query"}
async fn whois_api_post(
    State(stats): State<StatsState>,
    Json(query_data): Json<ApiQuery>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let query = query_data.q.trim();

    if query.is_empty() {
        let formatter = JsonFormatter::new();
        return Json(formatter.format_error(
            query,
            "Query field 'q' is required and cannot be empty",
            "unknown",
            start_time.elapsed().as_millis() as u64,
        ));
    }

    process_whois_query(query, stats, start_time).await
}

async fn process_whois_query(
    query: &str,
    stats: StatsState,
    start_time: Instant,
) -> Json<WhoisApiResponse> {
    let formatter = JsonFormatter::new();

    // 检测查询类型
    let query_type_str = detect_query_type(query);
    let query_type = analyze_query(query);

    // 处理查询
    match process_query(query, &query_type, None).await {
        Ok(result) => {
            // 更新统计信息
            {
                let mut stats_guard = stats.stats.write().await;
                stats_guard.total_requests += 1;
            }

            Json(formatter.format_response(
                query,
                result,
                &query_type_str,
                start_time.elapsed().as_millis() as u64,
            ))
        }
        Err(e) => Json(formatter.format_error(
            query,
            &format!("Query processing failed: {}", e),
            &query_type_str,
            start_time.elapsed().as_millis() as u64,
        )),
    }
}

fn detect_query_type(query: &str) -> String {
    let query_lower = query.to_lowercase();
    let query_trimmed = query.trim();

    // 域名检测
    if query_trimmed.contains('.') && query_trimmed.parse::<std::net::IpAddr>().is_err() {
        if query_lower.ends_with("-geo") {
            return "domain-geo".to_string();
        }
        return "domain".to_string();
    }

    // IP地址检测
    if query_trimmed.parse::<std::net::IpAddr>().is_ok() {
        if query_lower.ends_with("-geo") {
            return "ip-geo".to_string();
        }
        return "ip".to_string();
    }

    // CIDR检测
    if query_trimmed.contains('/') {
        return "cidr".to_string();
    }

    // ASN检测
    if query_lower.starts_with("as")
        && query_trimmed.len() > 2
        && query_trimmed[2..].parse::<u32>().is_ok()
    {
        return "asn".to_string();
    }

    // DN42相关检测
    if query_lower.ends_with("-dn42") || query_lower.ends_with("-mnt") {
        return "dn42".to_string();
    }

    // 邮箱检测
    if query_trimmed.contains('@') {
        return "email".to_string();
    }

    // 特殊服务检测
    if query_lower.starts_with("steam:") {
        return "steam".to_string();
    }

    if query_lower.starts_with("github:") {
        return "github".to_string();
    }

    if query_lower.starts_with("package:") {
        return "package".to_string();
    }

    if query_lower.starts_with("minecraft:") {
        return "minecraft".to_string();
    }

    // 其他
    "generic".to_string()
}

// API文档页面
async fn api_docs() -> impl IntoResponse {
    let html = include_str!("docs_template.html");
    Html(html)
}

// OpenAPI规范JSON
async fn openapi_spec() -> impl IntoResponse {
    let spec = include_str!("openapi.json");
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        spec,
    )
}

// GET /raw/:query - 返回原始WHOIS结果，不做任何JSON处理
async fn raw_whois_query(
    Path(query_param): Path<String>,
    State(stats): State<StatsState>,
) -> impl IntoResponse {
    let query = urlencoding::decode(&query_param)
        .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&query_param))
        .to_string();

    let query = query.trim();

    if query.is_empty() {
        return (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            "Error: Query parameter is required and cannot be empty".to_string(),
        );
    }

    // 检测查询类型
    let query_type = analyze_query(query);

    // 处理查询
    match process_query(query, &query_type, None).await {
        Ok(result) => {
            // 更新统计信息
            {
                let mut stats_guard = stats.stats.write().await;
                stats_guard.total_requests += 1;
            }

            (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                result,
            )
        }
        Err(e) => (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            format!("Error: Query processing failed: {}", e),
        ),
    }
}
