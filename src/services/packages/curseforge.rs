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
use reqwest::Client;
use serde::Deserialize;
use std::env;

use crate::{log_debug, log_error};
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CurseForgeProject {
    id: u64,
    name: String,
    slug: String,
    summary: String,
    #[serde(default)]
    links: Links,
    authors: Vec<Author>,
    logo: Logo,
    screenshots: Vec<Screenshot>,
    #[serde(rename = "downloadCount")]
    download_count: u64,
    #[serde(rename = "dateCreated")]
    date_created: String,
    #[serde(rename = "dateModified")]
    date_modified: String,
    #[serde(rename = "dateReleased")]
    date_released: String,
    #[serde(rename = "gameId")]
    game_id: u32,
    #[serde(default)]
    categories: Vec<Category>,
    #[serde(rename = "latestFiles")]
    #[serde(default)]
    latest_files: Vec<LatestFile>,
    #[serde(rename = "latestFilesIndexes")]
    #[serde(default)]
    latest_files_indexes: Vec<FileIndex>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
struct Links {
    #[serde(rename = "websiteUrl")]
    website_url: Option<String>,
    #[serde(rename = "wikiUrl")]
    wiki_url: Option<String>,
    #[serde(rename = "issuesUrl")]
    issues_url: Option<String>,
    #[serde(rename = "sourceUrl")]
    source_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Author {
    id: u64,
    name: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Logo {
    id: u64,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Screenshot {
    id: u64,
    title: String,
    description: String,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Category {
    id: u32,
    name: String,
    slug: String,
    url: String,
    #[serde(rename = "iconUrl")]
    icon_url: String,
    #[serde(rename = "dateModified")]
    date_modified: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LatestFile {
    id: u64,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "fileDate")]
    file_date: String,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(rename = "gameVersions")]
    game_versions: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FileIndex {
    #[serde(rename = "gameVersion")]
    game_version: String,
    #[serde(rename = "fileId")]
    file_id: u64,
    filename: String,
    #[serde(rename = "releaseType")]
    release_type: u32,
}

#[derive(Debug, Deserialize)]
struct CurseForgeResponse {
    data: CurseForgeProject,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<CurseForgeProject>,
}

pub async fn query_curseforge(query: &str) -> Result<String> {
    let api_key = match env::var("CURSEFORGE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return Ok("% CurseForge API key not configured\n% Set CURSEFORGE_API_KEY environment variable to enable CurseForge queries\n% Get your API key from: https://console.curseforge.com/".to_string());
        }
    };

    let client = Client::builder()
        .user_agent("Akaere WHois/0.2.0 (contact: team@akae.re)")
        .build()?;

    // 尝试将查询解析为项目ID (纯数字)
    if let Ok(project_id) = query.parse::<u64>() {
        return get_project_by_id(&client, &api_key, project_id).await;
    }

    // 否则进行搜索
    search_curseforge(&client, &api_key, query).await
}

async fn get_project_by_id(client: &Client, api_key: &str, project_id: u64) -> Result<String> {
    let url = format!("https://api.curseforge.com/v1/mods/{}", project_id);

    log_debug!("CurseForge request URL: {}", url);
    log_debug!(
        "CurseForge API key (first 10 chars): {}",
        &api_key[..10.min(api_key.len())]
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("x-api-key", api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error body".to_string());
        log_error!(
            "CurseForge API error - Status: {}, Body: {}",
            status,
            error_body
        );
        return Ok(format!(
            "% CurseForge API error: {}\n% Project ID {} not found or API quota exceeded\n% Error details: {}",
            status, project_id, error_body
        ));
    }

    let curse_response: CurseForgeResponse = response.json().await?;
    Ok(format_project_info(&curse_response.data))
}

async fn search_curseforge(client: &Client, api_key: &str, query: &str) -> Result<String> {
    let url = format!(
        "https://api.curseforge.com/v1/mods/search?gameId=432&searchFilter={}&pageSize=5",
        urlencoding::encode(query)
    );

    log_debug!("CurseForge search URL: {}", url);
    log_debug!(
        "CurseForge API key (first 10 chars): {}",
        &api_key[..10.min(api_key.len())]
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("x-api-key", api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error body".to_string());
        log_error!(
            "CurseForge search API error - Status: {}, Body: {}",
            status,
            error_body
        );
        return Ok(format!(
            "% CurseForge API error: {}\n% Error details: {}",
            status, error_body
        ));
    }

    let search_response: SearchResponse = response.json().await?;

    if search_response.data.is_empty() {
        return Ok(format!("% No CurseForge mods found for: {}", query));
    }

    let mut output = String::new();
    output.push_str("% ======================================================================\n");
    output.push_str(&format!("% CurseForge Search Results: {}\n", query));
    output.push_str("% ======================================================================\n");
    output.push_str("% \n");

    for (i, project) in search_response.data.iter().enumerate() {
        output.push_str(&format!("% --- Result {} ---\n", i + 1));
        output.push_str(&format!("project-id:           {}\n", project.id));
        output.push_str(&format!("project-name:         {}\n", project.name));
        output.push_str(&format!("project-slug:         {}\n", project.slug));
        output.push_str(&format!("summary:              {}\n", project.summary));

        if !project.authors.is_empty() {
            let authors: Vec<String> = project.authors.iter().map(|a| a.name.clone()).collect();
            output.push_str(&format!("authors:              {}\n", authors.join(", ")));
        }

        output.push_str(&format!(
            "downloads:            {:>12}\n",
            format_number(project.download_count)
        ));

        if !project.categories.is_empty() {
            let cats: Vec<String> = project.categories.iter().map(|c| c.name.clone()).collect();
            output.push_str(&format!("categories:           {}\n", cats.join(", ")));
        }

        output.push_str(&format!(
            "curseforge-url:       https://www.curseforge.com/minecraft/mc-mods/{}\n",
            project.slug
        ));

        if i < search_response.data.len() - 1 {
            output.push_str("% \n");
        }
    }

    output.push_str("% \n");
    output.push_str("% ======================================================================\n");
    output.push_str("% Use project ID for detailed info: <project-id>-CURSEFORGE\n");
    output.push_str("% ======================================================================\n");

    Ok(output)
}

fn format_project_info(project: &CurseForgeProject) -> String {
    let mut output = String::new();

    // 标题
    output.push_str("% ======================================================================\n");
    output.push_str(&format!("% CurseForge: {}\n", project.name));
    output.push_str("% ======================================================================\n");
    output.push_str("% \n");

    output.push_str(&format!("project-id:           {}\n", project.id));
    output.push_str(&format!("project-name:         {}\n", project.name));
    output.push_str(&format!("project-slug:         {}\n", project.slug));
    output.push_str(&format!("summary:              {}\n", project.summary));

    // 作者信息
    if !project.authors.is_empty() {
        output.push_str("% \n");
        output.push_str("% --- Authors ---\n");
        for author in &project.authors {
            output.push_str(&format!(
                "author:               {} ({})\n",
                author.name, author.url
            ));
        }
    }

    // 统计信息
    output.push_str("% \n");
    output.push_str("% --- Statistics ---\n");
    output.push_str(&format!(
        "total-downloads:      {:>12}\n",
        format_number(project.download_count)
    ));

    // 分类
    if !project.categories.is_empty() {
        output.push_str("% \n");
        output.push_str("% --- Categories ---\n");
        let cats: Vec<String> = project.categories.iter().map(|c| c.name.clone()).collect();
        output.push_str(&format!("categories:           {}\n", cats.join(", ")));
    }

    // 时间信息
    output.push_str("% \n");
    output.push_str("% --- Timeline ---\n");
    output.push_str(&format!(
        "created:              {}\n",
        format_date(&project.date_created)
    ));
    output.push_str(&format!(
        "last-modified:        {}\n",
        format_date(&project.date_modified)
    ));
    output.push_str(&format!(
        "last-release:         {}\n",
        format_date(&project.date_released)
    ));

    // 链接
    output.push_str("% \n");
    output.push_str("% --- Links ---\n");
    output.push_str(&format!(
        "curseforge-url:       https://www.curseforge.com/minecraft/mc-mods/{}\n",
        project.slug
    ));

    if let Some(website) = &project.links.website_url {
        output.push_str(&format!("website:              {}\n", website));
    }
    if let Some(source) = &project.links.source_url {
        output.push_str(&format!("source-code:          {}\n", source));
    }
    if let Some(issues) = &project.links.issues_url {
        output.push_str(&format!("issue-tracker:        {}\n", issues));
    }
    if let Some(wiki) = &project.links.wiki_url {
        output.push_str(&format!("wiki:                 {}\n", wiki));
    }

    // Logo
    output.push_str("% \n");
    output.push_str("% --- Media ---\n");
    output.push_str(&format!("logo:                 {}\n", project.logo.url));

    if !project.screenshots.is_empty() {
        output.push_str(&format!("% Screenshots: {}\n", project.screenshots.len()));
        for (i, screenshot) in project.screenshots.iter().take(3).enumerate() {
            output.push_str(&format!(
                "screenshot-{}:         {} ({})\n",
                i + 1,
                screenshot.title,
                screenshot.url
            ));
        }
        if project.screenshots.len() > 3 {
            output.push_str(&format!(
                "% ... and {} more screenshots\n",
                project.screenshots.len() - 3
            ));
        }
    }

    // 最新文件/版本
    if !project.latest_files_indexes.is_empty() {
        output.push_str("% \n");
        output.push_str(&format!(
            "% --- Latest Files ({} available) ---\n",
            project.latest_files_indexes.len()
        ));

        // 获取支持的Minecraft版本
        let mut versions: Vec<String> = project
            .latest_files_indexes
            .iter()
            .map(|f| f.game_version.clone())
            .collect();
        versions.sort();
        versions.dedup();

        if versions.len() > 10 {
            output.push_str(&format!(
                "minecraft-versions:   {} to {} ({} versions)\n",
                versions.first().unwrap_or(&"".to_string()),
                versions.last().unwrap_or(&"".to_string()),
                versions.len()
            ));
        } else if !versions.is_empty() {
            output.push_str(&format!("minecraft-versions:   {}\n", versions.join(", ")));
        }

        // 显示最新几个文件
        for (i, file) in project.latest_files.iter().take(3).enumerate() {
            output.push_str(&format!(
                "latest-file-{}:        {}\n",
                i + 1,
                file.display_name
            ));
            output.push_str(&format!("  filename:           {}\n", file.file_name));
            output.push_str(&format!(
                "  date:               {}\n",
                format_date(&file.file_date)
            ));
            if !file.game_versions.is_empty() {
                output.push_str(&format!(
                    "  versions:           {}\n",
                    file.game_versions.join(", ")
                ));
            }
        }
    }

    output.push_str("% \n");
    output.push_str("% ======================================================================\n");
    output.push_str(&format!(
        "% View on CurseForge: https://www.curseforge.com/minecraft/mc-mods/{}\n",
        project.slug
    ));
    output.push_str("% ======================================================================\n");

    output
}

// 格式化数字，添加千位分隔符
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

// 格式化日期
fn format_date(date_str: &str) -> String {
    if let Some(date_part) = date_str.split('T').next() {
        date_part.to_string()
    } else {
        date_str.to_string()
    }
}
