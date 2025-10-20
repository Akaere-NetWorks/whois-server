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

use anyhow::{ Context, Result };
use reqwest;
use serde::{ Deserialize, Serialize };
use tracing::{ debug, error };

const GITHUB_API_URL: &str = "https://api.github.com";

#[derive(Debug, Deserialize, Serialize)]
struct GitHubUser {
    login: String,
    id: u64,
    avatar_url: String,
    html_url: String,
    #[serde(rename = "type")]
    user_type: String,
    site_admin: bool,
    name: Option<String>,
    company: Option<String>,
    blog: Option<String>,
    location: Option<String>,
    email: Option<String>,
    hireable: Option<bool>,
    bio: Option<String>,
    twitter_username: Option<String>,
    public_repos: u32,
    public_gists: u32,
    followers: u32,
    following: u32,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubRepository {
    id: u64,
    name: String,
    full_name: String,
    html_url: String,
    clone_url: String,
    ssh_url: String,
    description: Option<String>,
    homepage: Option<String>,
    language: Option<String>,
    private: bool,
    fork: bool,
    archived: bool,
    disabled: bool,
    stargazers_count: u32,
    watchers_count: u32,
    forks_count: u32,
    open_issues_count: u32,
    size: u32,
    default_branch: String,
    topics: Option<Vec<String>>,
    has_issues: bool,
    has_projects: bool,
    has_wiki: bool,
    has_pages: bool,
    has_downloads: bool,
    license: Option<GitHubLicense>,
    owner: GitHubOwner,
    created_at: String,
    updated_at: String,
    pushed_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubOwner {
    login: String,
    id: u64,
    avatar_url: String,
    html_url: String,
    #[serde(rename = "type")]
    user_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubLicense {
    key: String,
    name: String,
    spdx_id: Option<String>,
    url: Option<String>,
}

pub async fn process_github_query(query: &str) -> Result<String> {
    debug!("Processing GitHub query: {}", query);

    if query.is_empty() {
        return Err(anyhow::anyhow!("Query cannot be empty"));
    }

    // Determine if this is a user/org query or repository query
    if query.contains('/') {
        // Repository query format: owner/repo
        let parts: Vec<&str> = query.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository format. Use: owner/repository"));
        }

        let owner = parts[0];
        let repo = parts[1];

        // Validate GitHub username/repo name format
        if !is_valid_github_name(owner) || !is_valid_github_name(repo) {
            return Err(anyhow::anyhow!("Invalid GitHub username or repository name format"));
        }

        match query_github_repository(owner, repo).await {
            Ok(repository) => Ok(format_github_repository_response(&repository, query)),
            Err(e) => {
                error!("GitHub repository query failed for {}: {}", query, e);
                Ok(format_github_not_found(query, "repository"))
            }
        }
    } else {
        // User/organization query
        if !is_valid_github_name(query) {
            return Err(anyhow::anyhow!("Invalid GitHub username format"));
        }

        match query_github_user(query).await {
            Ok(user) => Ok(format_github_user_response(&user, query)),
            Err(e) => {
                error!("GitHub user query failed for {}: {}", query, e);
                Ok(format_github_not_found(query, "user"))
            }
        }
    }
}

fn is_valid_github_name(name: &str) -> bool {
    !name.is_empty() &&
        name.len() <= 39 &&
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') &&
        !name.starts_with('-') &&
        !name.ends_with('-') &&
        !name.contains("--")
}

async fn query_github_user(username: &str) -> Result<GitHubUser> {
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WHOIS-Server/1.0)")
        .build()
        .context("Failed to create HTTP client")?;

    let user_url = format!("{}/users/{}", GITHUB_API_URL, urlencoding::encode(username));

    debug!("Querying GitHub API: {}", user_url);

    let response = client
        .get(&user_url)
        .send().await
        .context("Failed to send request to GitHub API")?;

    if response.status() == 404 {
        return Err(anyhow::anyhow!("GitHub user not found"));
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("GitHub API returned status: {}", response.status()));
    }

    let user_data: GitHubUser = response.json().await.context("Failed to parse GitHub user data")?;

    Ok(user_data)
}

async fn query_github_repository(owner: &str, repo: &str) -> Result<GitHubRepository> {
    let client = reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WHOIS-Server/1.0)")
        .build()
        .context("Failed to create HTTP client")?;

    let repo_url = format!(
        "{}/repos/{}/{}",
        GITHUB_API_URL,
        urlencoding::encode(owner),
        urlencoding::encode(repo)
    );

    debug!("Querying GitHub API: {}", repo_url);

    let response = client
        .get(&repo_url)
        .send().await
        .context("Failed to send request to GitHub API")?;

    if response.status() == 404 {
        return Err(anyhow::anyhow!("GitHub repository not found"));
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("GitHub API returned status: {}", response.status()));
    }

    let repo_data: GitHubRepository = response
        .json().await
        .context("Failed to parse GitHub repository data")?;

    Ok(repo_data)
}

fn format_github_user_response(user: &GitHubUser, query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("GitHub User Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    output.push_str(&format!("username: {}\n", user.login));
    output.push_str(&format!("user-id: {}\n", user.id));
    output.push_str(&format!("user-type: {}\n", user.user_type));

    if let Some(name) = &user.name {
        output.push_str(&format!("display-name: {}\n", name));
    }

    if let Some(bio) = &user.bio {
        output.push_str(&format!("bio: {}\n", bio));
    }

    if let Some(company) = &user.company {
        output.push_str(&format!("company: {}\n", company));
    }

    if let Some(location) = &user.location {
        output.push_str(&format!("location: {}\n", location));
    }

    if let Some(email) = &user.email {
        output.push_str(&format!("email: {}\n", email));
    }

    if let Some(blog) = &user.blog
        && !blog.is_empty() {
            output.push_str(&format!("website: {}\n", blog));
        }

    if let Some(twitter) = &user.twitter_username {
        output.push_str(&format!("twitter: @{}\n", twitter));
    }

    output.push_str(&format!("public-repos: {}\n", user.public_repos));
    output.push_str(&format!("public-gists: {}\n", user.public_gists));
    output.push_str(&format!("followers: {}\n", user.followers));
    output.push_str(&format!("following: {}\n", user.following));

    if user.site_admin {
        output.push_str("site-admin: true\n");
    }

    if let Some(hireable) = user.hireable {
        output.push_str(&format!("hireable: {}\n", hireable));
    }

    output.push_str(&format!("created-at: {}\n", user.created_at));
    output.push_str(&format!("updated-at: {}\n", user.updated_at));

    output.push_str(&format!("github-url: {}\n", user.html_url));
    output.push_str(&format!("avatar-url: {}\n", user.avatar_url));
    output.push_str(&format!("api-url: {}/users/{}\n", GITHUB_API_URL, user.login));
    output.push_str("source: GitHub API\n");
    output.push('\n');
    output.push_str("% Information retrieved from GitHub\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_github_repository_response(repo: &GitHubRepository, query: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("GitHub Repository Information: {}\n", query));
    output.push_str("=".repeat(60).as_str());
    output.push('\n');

    output.push_str(&format!("repository-name: {}\n", repo.name));
    output.push_str(&format!("full-name: {}\n", repo.full_name));
    output.push_str(&format!("repository-id: {}\n", repo.id));

    if let Some(description) = &repo.description {
        output.push_str(&format!("description: {}\n", description));
    }

    output.push_str(&format!("owner: {}\n", repo.owner.login));
    output.push_str(&format!("owner-type: {}\n", repo.owner.user_type));

    if let Some(language) = &repo.language {
        output.push_str(&format!("language: {}\n", language));
    }

    if let Some(homepage) = &repo.homepage
        && !homepage.is_empty() {
            output.push_str(&format!("homepage: {}\n", homepage));
        }

    if let Some(license) = &repo.license {
        output.push_str(&format!("license: {}\n", license.name));
        if let Some(spdx_id) = &license.spdx_id {
            output.push_str(&format!("license-spdx: {}\n", spdx_id));
        }
    }

    output.push_str(&format!("default-branch: {}\n", repo.default_branch));

    output.push_str(&format!("stars: {}\n", repo.stargazers_count));
    output.push_str(&format!("watchers: {}\n", repo.watchers_count));
    output.push_str(&format!("forks: {}\n", repo.forks_count));
    output.push_str(&format!("open-issues: {}\n", repo.open_issues_count));

    let size_mb = (repo.size as f64) / 1024.0;
    output.push_str(&format!("size: {:.2} MB\n", size_mb));

    if repo.private {
        output.push_str("visibility: private\n");
    } else {
        output.push_str("visibility: public\n");
    }

    if repo.fork {
        output.push_str("fork: true\n");
    }

    if repo.archived {
        output.push_str("archived: true\n");
    }

    if repo.disabled {
        output.push_str("disabled: true\n");
    }

    // Features
    let mut features = Vec::new();
    if repo.has_issues {
        features.push("issues");
    }
    if repo.has_projects {
        features.push("projects");
    }
    if repo.has_wiki {
        features.push("wiki");
    }
    if repo.has_pages {
        features.push("pages");
    }
    if repo.has_downloads {
        features.push("downloads");
    }

    if !features.is_empty() {
        output.push_str(&format!("features: {}\n", features.join(", ")));
    }

    // Topics
    if let Some(topics) = &repo.topics
        && !topics.is_empty() {
            output.push_str(&format!("topics: {}\n", topics.join(", ")));
        }

    output.push_str(&format!("created-at: {}\n", repo.created_at));
    output.push_str(&format!("updated-at: {}\n", repo.updated_at));

    if let Some(pushed_at) = &repo.pushed_at {
        output.push_str(&format!("pushed-at: {}\n", pushed_at));
    }

    output.push_str(&format!("github-url: {}\n", repo.html_url));
    output.push_str(&format!("clone-url: {}\n", repo.clone_url));
    output.push_str(&format!("ssh-url: {}\n", repo.ssh_url));
    output.push_str(&format!("api-url: {}/repos/{}\n", GITHUB_API_URL, repo.full_name));
    output.push_str("source: GitHub API\n");
    output.push('\n');
    output.push_str("% Information retrieved from GitHub\n");
    output.push_str("% Query processed by WHOIS server\n");

    output
}

fn format_github_not_found(query: &str, resource_type: &str) -> String {
    format!(
        "GitHub {} Not Found: {}\n\
        No {} with this name was found on GitHub.\n\
        \n\
        You can search manually at: https://github.com/search?q={}\n\
        \n\
        % {} not found on GitHub\n\
        % Query processed by WHOIS server\n",
        resource_type.to_uppercase(),
        query,
        resource_type,
        urlencoding::encode(query),
        resource_type.to_uppercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_name_validation() {
        // Valid names
        assert!(is_valid_github_name("user123"));
        assert!(is_valid_github_name("user-name"));
        assert!(is_valid_github_name("123user"));

        // Invalid names
        assert!(!is_valid_github_name(""));
        assert!(!is_valid_github_name("-user"));
        assert!(!is_valid_github_name("user-"));
        assert!(!is_valid_github_name("user--name"));
        assert!(!is_valid_github_name(&"a".repeat(40)));
    }

    #[tokio::test]
    async fn test_github_service_creation() {
        let result = process_github_query("nonexistent-user-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("GitHub"));
    }
}
