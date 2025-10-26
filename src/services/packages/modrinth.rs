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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ModrinthProject {
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    categories: Vec<String>,
    client_side: String,
    server_side: String,
    project_type: String,
    downloads: u64,
    followers: u32,
    #[serde(default)]
    versions: Vec<String>,
    license: License,
    #[serde(default)]
    gallery: Vec<GalleryImage>,
    #[serde(default)]
    donation_urls: Vec<DonationUrl>,
    #[serde(default)]
    date_created: Option<String>,
    #[serde(default)]
    date_modified: Option<String>,
    published: String,
    updated: String,
    #[serde(default)]
    approved: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    team: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    additional_categories: Vec<String>,
    #[serde(default)]
    issues_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    wiki_url: Option<String>,
    #[serde(default)]
    discord_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct License {
    id: String,
    name: String,
    url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GalleryImage {
    url: String,
    featured: bool,
    title: Option<String>,
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DonationUrl {
    id: String,
    platform: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
    offset: u32,
    limit: u32,
    total_hits: u32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SearchHit {
    slug: String,
    title: String,
    description: String,
    categories: Vec<String>,
    client_side: String,
    server_side: String,
    project_type: String,
    downloads: u64,
    follows: u32,
    icon_url: Option<String>,
    author: String,
    versions: Vec<String>,
    date_created: String,
    date_modified: String,
    latest_version: Option<String>,
    license: String,
    #[serde(default)]
    gallery: Vec<String>,
}

pub async fn query_modrinth(package_name: &str) -> Result<String> {
    let client = Client::builder().user_agent("Akaere-WHOIS/0.2.0").build()?;

    // 先尝试直接通过 slug/ID 获取项目
    let project_url = format!("https://api.modrinth.com/v2/project/{}", package_name);

    let project_result = client.get(&project_url).send().await;

    let result = if let Ok(response) = project_result {
        if response.status().is_success() {
            let project: ModrinthProject = response.json().await?;
            format_project_info(&project)
        } else {
            // 如果直接查询失败，尝试搜索
            search_modrinth(&client, package_name).await?
        }
    } else {
        // 如果请求失败，尝试搜索
        search_modrinth(&client, package_name).await?
    };

    Ok(result)
}

async fn search_modrinth(client: &Client, query: &str) -> Result<String> {
    let search_url = format!(
        "https://api.modrinth.com/v2/search?query={}&limit=5",
        urlencoding::encode(query)
    );

    let response = client.get(&search_url).send().await?;

    if !response.status().is_success() {
        return Ok(format!("% Modrinth query failed: {}", response.status()));
    }

    let search_result: SearchResponse = response.json().await?;

    if search_result.hits.is_empty() {
        return Ok(format!("% No Modrinth projects found for: {}", query));
    }

    let mut output = String::new();
    output.push_str(&format!("% Modrinth Search Results for: {}\n", query));
    output.push_str(&format!("% Total results: {}\n", search_result.total_hits));
    output.push_str("% \n");

    for (i, hit) in search_result.hits.iter().enumerate() {
        output.push_str(&format!("% --- Result {} ---\n", i + 1));
        output.push_str(&format!("project-slug:         {}\n", hit.slug));
        output.push_str(&format!("project-name:         {}\n", hit.title));
        output.push_str(&format!("project-type:         {}\n", hit.project_type));
        output.push_str(&format!("description:          {}\n", hit.description));
        output.push_str(&format!("author:               {}\n", hit.author));
        output.push_str(&format!("downloads:            {}\n", hit.downloads));
        output.push_str(&format!("followers:            {}\n", hit.follows));
        output.push_str(&format!(
            "categories:           {}\n",
            hit.categories.join(", ")
        ));
        output.push_str(&format!("client-side:          {}\n", hit.client_side));
        output.push_str(&format!("server-side:          {}\n", hit.server_side));
        output.push_str(&format!("license:              {}\n", hit.license));

        if !hit.versions.is_empty() {
            output.push_str(&format!(
                "mc-versions:          {} versions available\n",
                hit.versions.len()
            ));
        }

        if let Some(icon) = &hit.icon_url {
            output.push_str(&format!("icon-url:             {}\n", icon));
        }

        output.push_str(&format!("created:              {}\n", hit.date_created));
        output.push_str(&format!("updated:              {}\n", hit.date_modified));
        output.push_str(&format!(
            "modrinth-url:         https://modrinth.com/mod/{}\n",
            hit.slug
        ));

        if i < search_result.hits.len() - 1 {
            output.push_str("% \n");
        }
    }

    output.push_str("\n% Use exact slug for detailed info: <slug>-MODRINTH\n");

    Ok(output)
}

fn format_project_info(project: &ModrinthProject) -> String {
    let mut output = String::new();

    // 标题和基本信息
    output.push_str("% ======================================================================\n");
    output.push_str(&format!(
        "% Modrinth: {} ({})\n",
        project.title,
        project.project_type.to_uppercase()
    ));
    output.push_str("% ======================================================================\n");
    output.push_str("% \n");

    output.push_str(&format!("project-slug:         {}\n", project.slug));
    output.push_str(&format!("project-name:         {}\n", project.title));
    output.push_str(&format!("project-type:         {}\n", project.project_type));
    output.push_str(&format!("description:          {}\n", project.description));

    // 统计信息
    output.push_str("% \n");
    output.push_str("% --- Statistics ---\n");
    output.push_str(&format!(
        "downloads:            {:>12}\n",
        format_number(project.downloads)
    ));
    output.push_str(&format!(
        "followers:            {:>12}\n",
        format_number(project.followers as u64)
    ));

    // 分类
    if !project.categories.is_empty() {
        output.push_str(&format!(
            "categories:           {}\n",
            project.categories.join(", ")
        ));
    }
    if !project.additional_categories.is_empty() {
        output.push_str(&format!(
            "extra-categories:     {}\n",
            project.additional_categories.join(", ")
        ));
    }

    // 兼容性信息
    output.push_str("% \n");
    output.push_str("% --- Compatibility ---\n");
    output.push_str(&format!("client-side:          {}\n", project.client_side));
    output.push_str(&format!("server-side:          {}\n", project.server_side));

    if !project.loaders.is_empty() {
        output.push_str(&format!(
            "mod-loaders:          {}\n",
            project.loaders.join(", ")
        ));
    }

    if !project.game_versions.is_empty() {
        let total_versions = project.game_versions.len();
        let versions_display = if total_versions > 10 {
            let first = &project.game_versions[0];
            let last = &project.game_versions[total_versions - 1];
            format!("{} to {} ({} versions)", first, last, total_versions)
        } else {
            project.game_versions.join(", ")
        };
        output.push_str(&format!("minecraft-versions:   {}\n", versions_display));
    }

    // 许可证
    output.push_str("% \n");
    output.push_str("% --- License ---\n");
    output.push_str(&format!(
        "license:              {} ({})\n",
        project.license.name, project.license.id
    ));
    if let Some(license_url) = &project.license.url {
        output.push_str(&format!("license-url:          {}\n", license_url));
    }

    // 时间信息
    output.push_str("% \n");
    output.push_str("% --- Timeline ---\n");
    if let Some(created) = &project.date_created {
        output.push_str(&format!("created:              {}\n", format_date(created)));
    }
    output.push_str(&format!(
        "published:            {}\n",
        format_date(&project.published)
    ));
    output.push_str(&format!(
        "last-updated:         {}\n",
        format_date(&project.updated)
    ));
    if let Some(approved) = &project.approved {
        output.push_str(&format!(
            "approved:             {}\n",
            format_date(approved)
        ));
    }

    // 链接
    output.push_str("% \n");
    output.push_str("% --- Links ---\n");
    output.push_str(&format!(
        "modrinth-url:         https://modrinth.com/{}/{}\n",
        project.project_type, project.slug
    ));

    if let Some(source) = &project.source_url {
        output.push_str(&format!("source-code:          {}\n", source));
    }

    if let Some(issues) = &project.issues_url {
        output.push_str(&format!("issue-tracker:        {}\n", issues));
    }

    if let Some(wiki) = &project.wiki_url {
        output.push_str(&format!("wiki:                 {}\n", wiki));
    }

    if let Some(discord) = &project.discord_url {
        output.push_str(&format!("discord:              {}\n", discord));
    }

    // 捐赠
    if !project.donation_urls.is_empty() {
        output.push_str("% \n");
        output.push_str("% --- Support the Author ---\n");
        for donation in &project.donation_urls {
            output.push_str(&format!(
                "{:<20}  {}\n",
                format!("{}:", donation.platform),
                donation.url
            ));
        }
    }

    // 画廊
    if !project.gallery.is_empty() {
        output.push_str("% \n");
        output.push_str(&format!(
            "% --- Gallery ({} images) ---\n",
            project.gallery.len()
        ));
        for (i, image) in project.gallery.iter().take(3).enumerate() {
            if let Some(title) = &image.title {
                output.push_str(&format!(
                    "image-{}:              {} ({})\n",
                    i + 1,
                    title,
                    if image.featured { "featured" } else { "" }
                ));
            } else {
                output.push_str(&format!(
                    "image-{}:              {}\n",
                    i + 1,
                    if image.featured {
                        "featured"
                    } else {
                        "screenshot"
                    }
                ));
            }
            output.push_str(&format!("  url:                {}\n", image.url));
        }
        if project.gallery.len() > 3 {
            output.push_str(&format!(
                "% ... and {} more images on Modrinth\n",
                project.gallery.len() - 3
            ));
        }
    }

    // 版本
    if !project.versions.is_empty() {
        output.push_str("% \n");
        output.push_str(&format!(
            "% --- Versions ({} available) ---\n",
            project.versions.len()
        ));
        if project.versions.len() > 5 {
            output.push_str(&format!(
                "latest-versions:      {}\n",
                project.versions[..5].join(", ")
            ));
            output.push_str(&format!(
                "% ... and {} more versions available\n",
                project.versions.len() - 5
            ));
        } else {
            output.push_str(&format!(
                "available-versions:   {}\n",
                project.versions.join(", ")
            ));
        }
    }

    output.push_str("% \n");
    output.push_str("% ======================================================================\n");
    output.push_str(&format!(
        "% View full details at: https://modrinth.com/{}/{}\n",
        project.project_type, project.slug
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

// 格式化日期，只显示日期部分
fn format_date(date_str: &str) -> String {
    if let Some(date_part) = date_str.split('T').next() {
        date_part.to_string()
    } else {
        date_str.to_string()
    }
}
