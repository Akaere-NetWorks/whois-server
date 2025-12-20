// WHOIS Server - Response Patch System
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patch system for automatic text replacement in WHOIS responses
//!
//! This module implements a flexible patch system using unified diff format
//! that allows automatic text replacement in WHOIS query responses based on:
//! - Query content (input keywords)
//! - Response content (output keywords)
//! - Regular expressions
//!
//! Patches use standard unified diff format for compatibility and readability.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use crate::{log_debug, log_error, log_info, log_warn};
/// Strip ANSI color codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // ANSI escape code pattern: \x1b[...m
    let re = Regex::new(r"\x1b\[[0-9;]*m").expect("Invalid ANSI regex pattern");
    re.replace_all(s, "").to_string()
}

/// A single diff hunk (one replacement operation)
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Lines to remove (starting with -)
    pub remove_lines: Vec<String>,
    /// Lines to add (starting with +)
    pub add_lines: Vec<String>,
    /// Context lines for matching (reserved for future use)
    #[allow(dead_code)]
    pub context_before: Vec<String>,
    #[allow(dead_code)]
    pub context_after: Vec<String>,
}

/// A complete patch with conditions
#[derive(Debug, Clone)]
pub struct Patch {
    /// Conditions that must be met for this patch to apply
    pub conditions: Vec<PatchCondition>,
    /// Patterns to exclude from replacement (blacklist)
    pub excludes: Vec<String>,
    /// Context rules - only replace if certain patterns found in context
    pub context_rules: Vec<ContextRule>,
    /// All diff hunks in this patch
    pub hunks: Vec<DiffHunk>,
}

/// Context-based replacement rule
#[derive(Debug, Clone)]
pub struct ContextRule {
    /// Pattern to look for in context
    pub pattern: String,
    /// Direction to search: "before" or "after"
    pub direction: ContextDirection,
    /// Number of lines to search
    pub lines: usize,
    /// Action: "skip" or "only"
    pub action: ContextAction,
}

/// Context search direction
#[derive(Debug, Clone, PartialEq)]
pub enum ContextDirection {
    Before, // Look backwards (upwards in file)
    After,  // Look forwards (downwards in file)
}

/// Context action type
#[derive(Debug, Clone, PartialEq)]
pub enum ContextAction {
    Skip, // Skip replacement if pattern found
    Only, // Only replace if pattern found
}

/// Result of context rule checking
#[derive(Debug, Clone, PartialEq)]
enum ContextCheckResult {
    Allow,           // No rules or all rules allow replacement
    Skip,            // Skip rule matched - don't replace
    OnlyButNotFound, // Only rule exists but pattern not found
}

/// Condition for applying a patch
#[derive(Debug, Clone)]
pub struct PatchCondition {
    pub condition_type: ConditionType,
    pub value: String,
    pub regex: Option<Regex>,
}

/// Type of condition
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Query contains this string
    QueryContains,
    /// Response contains this string
    ResponseContains,
    /// Query matches this regex
    QueryMatches,
    /// Response matches this regex
    ResponseMatches,
}

/// Metadata for patch updates from remote repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchMetadata {
    pub version: String,
    pub last_updated: String,
    pub repository: String,
    pub patches: Vec<PatchInfo>,
    pub metadata: MetadataInfo,
}

/// Information about a single patch file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInfo {
    pub name: String,
    pub description: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    pub priority: i32,
    pub enabled: bool,
    pub modified: String,
}

/// Metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataInfo {
    pub format_version: String,
    pub update_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

/// Collection of patches from a single file
#[derive(Debug, Clone)]
pub struct PatchFile {
    pub filename: String,
    pub patches: Vec<Patch>,
}

/// Global patch manager
static PATCH_MANAGER: Lazy<RwLock<PatchManager>> = Lazy::new(|| RwLock::new(PatchManager::new()));

/// Manages all patch files
pub struct PatchManager {
    patch_files: Vec<PatchFile>,
    loaded: bool,
    storage: Option<crate::storage::lmdb::LmdbStorage>,
}

impl Default for PatchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchManager {
    /// Create a new patch manager
    pub fn new() -> Self {
        PatchManager {
            patch_files: Vec::new(),
            loaded: false,
            storage: None,
        }
    }

    /// Initialize LMDB storage for patches
    fn init_storage(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.storage.is_none() {
            let storage = crate::storage::lmdb::LmdbStorage::new("./cache/patches_cache")?;
            self.storage = Some(storage);
        }
        Ok(())
    }

    /// Download and update patches from remote repository (async)
    pub async fn update_patches_from_remote(
        &mut self,
        update_url: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.init_storage()?;

        let url = update_url.unwrap_or(
            "https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/patches.json"
        );

        log_info!("Fetching patch metadata from: {}", url);

        // Download patches.json (async) with cache-busting
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Pragma", "no-cache")
            .header("Expires", "0")
            .send()
            .await?;
        let metadata: PatchMetadata = response.json().await?;

        let mut output = String::new();
        output.push_str("% Patch Update Report\n");
        output.push_str(&format!("% Downloaded from: {}\n", url));
        output.push_str(&format!("% Last Updated: {}\n", metadata.last_updated));
        output.push_str(&format!(
            "% Format Version: {}\n",
            metadata.metadata.format_version
        ));
        output.push_str("%\n");

        let mut success_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;

        for patch_info in &metadata.patches {
            if !patch_info.enabled {
                log_debug!("Skipping disabled patch: {}", patch_info.name);
                continue;
            }

            output.push_str(&format!("\npatch:           {}\n", patch_info.name));
            output.push_str(&format!("description:     {}\n", patch_info.description));
            output.push_str(&format!("url:             {}\n", patch_info.url));
            output.push_str(&format!("sha1-expected:   {}\n", patch_info.sha1));
            output.push_str(&format!("size-expected:   {} bytes\n", patch_info.size));
            output.push_str(&format!("priority:        {}\n", patch_info.priority));
            output.push_str(&format!("modified:        {}\n", patch_info.modified));

            match self.download_and_verify_patch(patch_info).await {
                Ok((actual_sha1, was_updated)) => {
                    output.push_str(&format!("sha1-actual:     {}\n", actual_sha1));

                    if actual_sha1 == patch_info.sha1 {
                        if was_updated {
                            output.push_str("status:          ✓ VERIFIED (downloaded)\n");
                            success_count += 1;
                        } else {
                            output.push_str("status:          ✓ UP-TO-DATE (skipped)\n");
                            skipped_count += 1;
                        }
                    } else {
                        output.push_str("status:          ✗ SHA1 MISMATCH\n");
                        failed_count += 1;
                    }
                }
                Err(e) => {
                    output.push_str("status:          ✗ FAILED\n");
                    output.push_str(&format!("error:           {}\n", e));
                    failed_count += 1;
                }
            }
        }

        output.push_str("\n% Summary\n");
        output.push_str(&format!("% Total patches: {}\n", metadata.patches.len()));
        output.push_str(&format!("% Downloaded: {}\n", success_count));
        output.push_str(&format!("% Up-to-date (skipped): {}\n", skipped_count));
        output.push_str(&format!("% Failed: {}\n", failed_count));
        output.push_str("%\n");

        // Reload patches from LMDB into memory
        // Reload patches from LMDB into memory if any patches were processed
        if success_count > 0 || skipped_count > 0 {
            log_info!("Reloading patches from LMDB storage...");
            match self.load_patches_from_storage() {
                Ok(count) => {
                    output.push_str(&format!(
                        "% Patches reloaded: {} patch files loaded into memory\n",
                        count
                    ));
                    log_info!("Successfully reloaded {} patch files", count);
                }
                Err(e) => {
                    output.push_str(&format!("% Warning: Failed to reload patches: {}\n", e));
                    log_warn!("Failed to reload patches after update: {}", e);
                }
            }
        }
        output.push_str("%\n");
        output.push_str("% Run 'whois help' for more information\n");

        Ok(output)
    }

    /// Download a patch file and verify its SHA1 (async)
    /// Returns: (actual_sha1, was_updated)
    async fn download_and_verify_patch(
        &mut self,
        patch_info: &PatchInfo,
    ) -> Result<(String, bool), Box<dyn std::error::Error>> {
        // Check if patch already exists in LMDB with same SHA1
        if let Some(storage) = &self.storage {
            let meta_key = format!("meta:{}", patch_info.name);
            if let Ok(Some(existing_meta_json)) = storage.get(&meta_key)
                && let Ok(existing_info) = serde_json::from_str::<PatchInfo>(&existing_meta_json)
            {
                if existing_info.sha1 == patch_info.sha1 {
                    log_debug!(
                        "Patch {} already exists with same SHA1, skipping download",
                        patch_info.name
                    );
                    return Ok((patch_info.sha1.clone(), false));
                } else {
                    log_debug!(
                        "Patch {} exists but SHA1 changed: {} -> {}",
                        patch_info.name, existing_info.sha1, patch_info.sha1
                    );
                }
            }
        }

        log_debug!("Downloading patch: {}", patch_info.name);

        // Download patch content (async) with cache-busting
        let client = reqwest::Client::new();
        let response = client
            .get(&patch_info.url)
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Pragma", "no-cache")
            .header("Expires", "0")
            .send()
            .await?;
        let content = response.text().await?;

        // Calculate SHA1
        let actual_sha1 = self.calculate_sha1(&content);

        // Store in LMDB if verification passes
        if actual_sha1 == patch_info.sha1 {
            if let Some(storage) = &self.storage {
                let key = format!("patch:{}", patch_info.name);
                storage.put(&key, &content)?;

                // Store metadata
                let meta_key = format!("meta:{}", patch_info.name);
                let meta_json = serde_json::to_string(&patch_info)?;
                storage.put(&meta_key, &meta_json)?;

                log_debug!("Stored patch {} in LMDB", patch_info.name);
            }
            Ok((actual_sha1, true))
        } else {
            Ok((actual_sha1, false))
        }
    }

    /// Calculate SHA1 checksum
    fn calculate_sha1(&self, content: &str) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Load all patches from LMDB storage
    pub fn load_patches_from_storage(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        self.init_storage()?;

        let storage = self.storage.as_ref().ok_or("Storage not initialized")?;

        self.patch_files.clear();
        let mut total_patches = 0;

        // List all keys from LMDB and find patch metadata
        let mut patch_names = Vec::new();

        match storage.list_keys() {
            Ok(keys) => {
                // Filter keys that start with "meta:" to get patch metadata
                for key in keys {
                    if key.starts_with("meta:") {
                        // Extract patch name from "meta:001-ruinetwork.patch" -> "001-ruinetwork.patch"
                        let patch_name = key.strip_prefix("meta:").unwrap_or(&key);
                        patch_names.push(patch_name.to_string());
                        log_debug!("Found patch in storage: {}", patch_name);
                    }
                }

                // Sort by name (numeric prefix ensures correct order)
                patch_names.sort();

                log_debug!("Found {} patches in storage", patch_names.len());
            }
            Err(e) => {
                log_warn!("Failed to list keys from LMDB: {}", e);
            }
        }

        for name in patch_names {
            let key = format!("patch:{}", name);
            match storage.get(&key) {
                Ok(Some(content)) => match self.parse_patch_content(&name, &content) {
                    Ok(patch_file) => {
                        log_debug!(
                            "Loaded patch from storage: {} ({} patches)",
                            patch_file.filename,
                            patch_file.patches.len()
                        );
                        total_patches += patch_file.patches.len();
                        self.patch_files.push(patch_file);
                    }
                    Err(e) => {
                        log_error!("Failed to parse patch {}: {}", name, e);
                    }
                },
                #[allow(non_snake_case)]
                Ok(None) => {
                    log_debug!("Patch {} not found in storage", name);
                }
                Err(e) => {
                    log_debug!("Could not read patch {}: {}", name, e);
                }
            }
        }

        self.loaded = true;
        log_info!("Loaded {} patches from LMDB storage", total_patches);
        Ok(total_patches)
    }

    /// Parse patch content from string
    fn parse_patch_content(
        &self,
        filename: &str,
        content: &str,
    ) -> Result<PatchFile, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut patches = Vec::new();
        let mut current_conditions: Vec<PatchCondition> = Vec::new();
        let mut current_excludes: Vec<String> = Vec::new();
        let mut current_context_rules: Vec<ContextRule> = Vec::new();
        let mut current_hunks: Vec<DiffHunk> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Parse exclude patterns
            if line.starts_with("# EXCLUDE:") {
                let pattern = line.trim_start_matches("# EXCLUDE:").trim().to_string();
                current_excludes.push(pattern);
                i += 1;
                continue;
            }

            // Parse context rules: # SKIP_AFTER: pattern, lines
            if line.starts_with("# SKIP_AFTER:") {
                let params = line.trim_start_matches("# SKIP_AFTER:").trim();
                if let Some((pattern, lines_str)) = params.split_once(',') {
                    let pattern = pattern.trim().to_string();
                    let lines = lines_str.trim().parse::<usize>().unwrap_or(10);
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::After,
                        lines,
                        action: ContextAction::Skip,
                    });
                }
                i += 1;
                continue;
            }

            // Parse context rules: # SKIP_BEFORE: pattern, lines
            if line.starts_with("# SKIP_BEFORE:") {
                let params = line.trim_start_matches("# SKIP_BEFORE:").trim();
                if let Some((pattern, lines_str)) = params.split_once(',') {
                    let pattern = pattern.trim().to_string();
                    let lines = lines_str.trim().parse::<usize>().unwrap_or(10);
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::Before,
                        lines,
                        action: ContextAction::Skip,
                    });
                }
                i += 1;
                continue;
            }

            // Parse context rules: # ONLY_AFTER: pattern, lines
            if line.starts_with("# ONLY_AFTER:") {
                let params = line.trim_start_matches("# ONLY_AFTER:").trim();
                if let Some((pattern, lines_str)) = params.split_once(',') {
                    let pattern = pattern.trim().to_string();
                    let lines = lines_str.trim().parse::<usize>().unwrap_or(10);
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::After,
                        lines,
                        action: ContextAction::Only,
                    });
                } else {
                    // Default to 50 lines if no number specified
                    let pattern = params.trim().to_string();
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::After,
                        lines: 50,
                        action: ContextAction::Only,
                    });
                }
                i += 1;
                continue;
            }

            // Parse context rules: # ONLY_BEFORE: pattern, lines
            if line.starts_with("# ONLY_BEFORE:") {
                let params = line.trim_start_matches("# ONLY_BEFORE:").trim();
                if let Some((pattern, lines_str)) = params.split_once(',') {
                    let pattern = pattern.trim().to_string();
                    let lines = lines_str.trim().parse::<usize>().unwrap_or(10);
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::Before,
                        lines,
                        action: ContextAction::Only,
                    });
                } else {
                    // Default to 50 lines if no number specified
                    let pattern = params.trim().to_string();
                    current_context_rules.push(ContextRule {
                        pattern,
                        direction: ContextDirection::Before,
                        lines: 50,
                        action: ContextAction::Only,
                    });
                }
                i += 1;
                continue;
            }

            // Parse condition headers
            if line.starts_with("# QUERY_CONTAINS:") {
                let value = line
                    .trim_start_matches("# QUERY_CONTAINS:")
                    .trim()
                    .to_string();
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::QueryContains,
                    value,
                    regex: None,
                });
                i += 1;
                continue;
            }

            if line.starts_with("# RESPONSE_CONTAINS:") {
                let value = line
                    .trim_start_matches("# RESPONSE_CONTAINS:")
                    .trim()
                    .to_string();
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::ResponseContains,
                    value,
                    regex: None,
                });
                i += 1;
                continue;
            }

            if line.starts_with("# QUERY_MATCHES:") {
                let pattern = line
                    .trim_start_matches("# QUERY_MATCHES:")
                    .trim()
                    .to_string();
                let regex = Regex::new(&pattern)?;
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::QueryMatches,
                    value: pattern,
                    regex: Some(regex),
                });
                i += 1;
                continue;
            }

            if line.starts_with("# RESPONSE_MATCHES:") {
                let pattern = line
                    .trim_start_matches("# RESPONSE_MATCHES:")
                    .trim()
                    .to_string();
                let regex = Regex::new(&pattern)?;
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::ResponseMatches,
                    value: pattern,
                    regex: Some(regex),
                });
                i += 1;
                continue;
            }

            // Skip other comments
            if line.starts_with('#') {
                i += 1;
                continue;
            }

            // Parse diff section
            if line.starts_with("---") {
                // Parse a complete diff hunk
                if let Some(hunk) = self.parse_diff_hunk(&lines, &mut i)? {
                    current_hunks.push(hunk);
                }
                continue;
            }

            i += 1;
        }

        // Create final patch if we have hunks
        if !current_hunks.is_empty() {
            patches.push(Patch {
                conditions: current_conditions,
                excludes: current_excludes,
                context_rules: current_context_rules,
                hunks: current_hunks,
            });
        }

        Ok(PatchFile {
            filename: filename.to_string(),
            patches,
        })
    }

    /// Parse a single diff hunk
    fn parse_diff_hunk(
        &self,
        lines: &[&str],
        index: &mut usize,
    ) -> Result<Option<DiffHunk>, Box<dyn std::error::Error>> {
        // Skip "---" line
        if *index >= lines.len() || !lines[*index].trim().starts_with("---") {
            return Ok(None);
        }
        *index += 1;

        // Skip "+++" line
        if *index >= lines.len() || !lines[*index].trim().starts_with("+++") {
            return Ok(None);
        }
        *index += 1;

        // Skip @@ line (hunk header)
        if *index >= lines.len() || !lines[*index].trim().starts_with("@@") {
            return Ok(None);
        }
        *index += 1;

        let mut remove_lines = Vec::new();
        let mut add_lines = Vec::new();
        let mut context_before = Vec::new();
        let mut context_after = Vec::new();
        let mut in_removal = false;
        let mut in_addition = false;

        while *index < lines.len() {
            let line = lines[*index];

            // Stop at next diff section or condition
            if line.trim().starts_with("---")
                || line.trim().starts_with("# QUERY_")
                || line.trim().starts_with("# RESPONSE_")
            {
                break;
            }

            // Empty line might end the hunk
            if line.trim().is_empty() {
                *index += 1;
                break;
            }

            // Parse diff line
            if line.starts_with('-') {
                let content = line[1..].to_string();
                remove_lines.push(content);
                in_removal = true;
                in_addition = false;
            } else if line.starts_with('+') {
                let content = line[1..].to_string();
                add_lines.push(content);
                in_removal = false;
                in_addition = true;
            } else if line.starts_with(' ') {
                let content = line[1..].to_string();
                if !in_removal && !in_addition {
                    context_before.push(content);
                } else {
                    context_after.push(content);
                }
            }

            *index += 1;
        }

        if !remove_lines.is_empty() || !add_lines.is_empty() {
            Ok(Some(DiffHunk {
                remove_lines,
                add_lines,
                context_before,
                context_after,
            }))
        } else {
            Ok(None)
        }
    }

    /// Apply all patches to a response
    pub fn apply_patches(&self, query: &str, mut response: String) -> String {
        if !self.loaded || self.patch_files.is_empty() {
            log_debug!("No patches loaded or patch system not initialized");
            return response;
        }

        log_debug!("Processing {} patch files", self.patch_files.len());
        for patch_file in &self.patch_files {
            log_debug!("Checking {} patches from file", patch_file.patches.len());
            for patch in &patch_file.patches {
                if self.check_conditions(query, &response, &patch.conditions) {
                    log_debug!(
                        "Conditions matched, applying patch with {} hunks",
                        patch.hunks.len()
                    );
                    response = self.apply_patch(response, patch);
                } else {
                    log_debug!(
                        "Conditions not matched for patch with {} conditions",
                        patch.conditions.len()
                    );
                }
            }
        }

        response
    }

    /// Check if all conditions are met (OR logic - any condition matches)
    fn check_conditions(&self, query: &str, response: &str, conditions: &[PatchCondition]) -> bool {
        if conditions.is_empty() {
            log_debug!("No conditions - patch will always apply");
            return true; // No conditions means always apply
        }

        log_debug!("Checking {} conditions (OR logic)", conditions.len());
        // OR logic: if any condition is true, apply the patch
        for condition in conditions {
            let result = match condition.condition_type {
                ConditionType::QueryContains => {
                    let matches = query.contains(&condition.value);
                    log_debug!("QUERY_CONTAINS '{}': {}", condition.value, matches);
                    matches
                }
                ConditionType::ResponseContains => {
                    let matches = response.contains(&condition.value);
                    log_debug!("RESPONSE_CONTAINS '{}': {}", condition.value, matches);
                    matches
                }
                ConditionType::QueryMatches => {
                    if let Some(regex) = &condition.regex {
                        let matches = regex.is_match(query);
                        log_debug!("QUERY_MATCHES '{}': {}", condition.value, matches);
                        matches
                    } else {
                        false
                    }
                }
                ConditionType::ResponseMatches => {
                    if let Some(regex) = &condition.regex {
                        let matches = regex.is_match(response);
                        log_debug!("RESPONSE_MATCHES '{}': {}", condition.value, matches);
                        matches
                    } else {
                        false
                    }
                }
            };

            if result {
                log_debug!("Condition matched! Patch will be applied");
                return true; // Any condition being true is enough
            }
        }

        false // No conditions matched
    }

    /// Apply a single patch
    fn apply_patch(&self, mut response: String, patch: &Patch) -> String {
        for hunk in &patch.hunks {
            response = self.apply_hunk(response, hunk, &patch.excludes, &patch.context_rules);
        }
        response
    }

    /// Check context rules for a given line
    fn check_context_rules(
        lines: &[&str],
        line_idx: usize,
        rules: &[ContextRule],
    ) -> ContextCheckResult {
        if rules.is_empty() {
            return ContextCheckResult::Allow;
        }

        let mut has_only_rule = false;
        let mut only_rule_satisfied = false;

        for rule in rules {
            let range = match rule.direction {
                ContextDirection::Before => {
                    let start = line_idx.saturating_sub(rule.lines);
                    start..line_idx
                }
                ContextDirection::After => {
                    let end = (line_idx + rule.lines + 1).min(lines.len());
                    line_idx + 1..end
                }
            };

            // Check if pattern exists in the range
            let mut pattern_found = false;
            for i in range {
                if i < lines.len() {
                    let stripped = strip_ansi_codes(lines[i]);
                    if stripped.contains(&rule.pattern) {
                        pattern_found = true;
                        break;
                    }
                }
            }

            match rule.action {
                ContextAction::Skip => {
                    if pattern_found {
                        log_debug!(
                            "Context rule SKIP matched: pattern '{}' found {} lines {}",
                            rule.pattern,
                            rule.lines,
                            if rule.direction == ContextDirection::Before {
                                "before"
                            } else {
                                "after"
                            }
                        );
                        return ContextCheckResult::Skip;
                    }
                }
                ContextAction::Only => {
                    has_only_rule = true;
                    if pattern_found {
                        only_rule_satisfied = true;
                        log_debug!(
                            "Context rule ONLY matched: pattern '{}' found {} lines {}",
                            rule.pattern,
                            rule.lines,
                            if rule.direction == ContextDirection::Before {
                                "before"
                            } else {
                                "after"
                            }
                        );
                    }
                }
            }
        }

        // If there's an ONLY rule but it wasn't satisfied, don't replace
        if has_only_rule && !only_rule_satisfied {
            return ContextCheckResult::OnlyButNotFound;
        }

        ContextCheckResult::Allow
    }

    /// Check if a source: field should be replaced based on the block type
    /// Only replace source: in user-created objects (aut-num, organisation, person, etc.)
    /// Do NOT replace in registry blocks (as-block, route, etc.)
    fn should_replace_source_in_block(lines: &[&str], line_idx: usize) -> bool {
        // Look backwards up to 50 lines to find the object type
        let start = line_idx.saturating_sub(50);

        for i in (start..line_idx).rev() {
            let stripped = strip_ansi_codes(lines[i]);
            let trimmed = stripped.trim();

            // Stop at empty lines or comment lines (new block started)
            if trimmed.is_empty() || trimmed.starts_with('%') {
                // If we hit a block boundary without finding a user object, don't replace
                return false;
            }

            // Check for user object types that should be patched
            if trimmed.starts_with("aut-num:")
                || trimmed.starts_with("organisation:")
                || trimmed.starts_with("person:")
                || trimmed.starts_with("role:")
            {
                return true;
            }

            // Check for registry object types that should NOT be patched
            if trimmed.starts_with("as-block:")
                || trimmed.starts_with("route:")
                || trimmed.starts_with("route6:")
                || trimmed.starts_with("inet6num:")
                || trimmed.starts_with("inetnum:")
            {
                return false;
            }
        }

        // If we didn't find any object type, don't replace
        false
    }

    /// Apply a single diff hunk
    fn apply_hunk(
        &self,
        response: String,
        hunk: &DiffHunk,
        excludes: &[String],
        context_rules: &[ContextRule],
    ) -> String {
        if hunk.remove_lines.is_empty() {
            return response;
        }

        // Simple case: direct replacement
        if hunk.remove_lines.len() == 1 && hunk.add_lines.len() == 1 {
            let old = &hunk.remove_lines[0];
            let new = &hunk.add_lines[0];

            // Detect line ending style (\r\n or \n)
            let has_crlf = response.contains("\r\n");
            let line_ending = if has_crlf { "\r\n" } else { "\n" };

            // Check if this is a "line starts with" match (old starts with '^')
            let is_line_start_match = old.starts_with('^');
            let match_prefix = if is_line_start_match {
                &old[1..] // Remove the '^' marker
            } else {
                old
            };

            // Apply replacement line by line, skipping excluded patterns
            let lines: Vec<&str> = response.lines().collect();
            let mut result_lines = Vec::new();

            for (idx, line) in lines.iter().enumerate() {
                // Check if this line should be excluded
                let mut should_skip = false;
                for exclude_pattern in excludes {
                    if line.contains(exclude_pattern) {
                        log_debug!(
                            "Skipping replacement for excluded line: {}",
                            line.chars().take(60).collect::<String>()
                        );
                        should_skip = true;
                        break;
                    }
                }

                if should_skip {
                    result_lines.push(line.to_string());
                    continue;
                }

                // Check context rules
                let context_check = Self::check_context_rules(&lines, idx, context_rules);
                if context_check == ContextCheckResult::Skip {
                    log_debug!("Skipping replacement due to context rule");
                    result_lines.push(line.to_string());
                    continue;
                } else if context_check == ContextCheckResult::OnlyButNotFound {
                    log_debug!("Skipping replacement - ONLY rule not satisfied");
                    result_lines.push(line.to_string());
                    continue;
                }

                // For line-start matches on 'source:', check if this line belongs to an excluded block
                if is_line_start_match && match_prefix == "source:" {
                    // Look backwards (up to 50 lines) to check the block type
                    let should_replace = Self::should_replace_source_in_block(&lines, idx);
                    if !should_replace {
                        log_debug!(
                            "Skipping source: replacement - not in user object block (aut-num/organisation/person)"
                        );
                        result_lines.push(line.to_string());
                        continue;
                    }
                }

                // Apply replacement
                if is_line_start_match {
                    // Line-start match: replace entire line if it starts with the pattern
                    // Strip ANSI color codes for matching
                    let stripped_line = strip_ansi_codes(line);
                    if stripped_line.trim_start().starts_with(match_prefix) {
                        log_debug!(
                            "Line-start match: replacing entire line starting with '{}'",
                            match_prefix
                        );
                        result_lines.push(new.to_string());
                    } else {
                        result_lines.push(line.to_string());
                    }
                } else {
                    // Normal substring replacement
                    result_lines.push(line.replace(old, new));
                }
            }

            return result_lines.join(line_ending);
        }

        // Multi-line replacement
        let old_text = hunk.remove_lines.join("\n");
        let new_text = if hunk.add_lines.is_empty() {
            String::new()
        } else {
            hunk.add_lines.join("\n")
        };

        // Check if any line in the match contains an excluded pattern
        for exclude_pattern in excludes {
            if old_text.contains(exclude_pattern) {
                log_debug!(
                    "Skipping multi-line replacement due to excluded pattern: {}",
                    exclude_pattern
                );
                return response;
            }
        }

        response.replace(&old_text, &new_text)
    }
}

/// Initialize the patch system - load from LMDB storage
pub fn init_patches(_patches_dir: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut manager = PATCH_MANAGER.write().map_err(|_| anyhow::anyhow!("Patch manager mutex poisoned"))?;
    manager.load_patches_from_storage()
}

/// Update patches from remote repository (async)
pub async fn update_patches_from_remote(
    update_url: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Spawn blocking task to avoid Send issues with RwLock
    let url = update_url.map(|s| s.to_string());
    let result = tokio::task::spawn_blocking(move || {
        let mut manager = PATCH_MANAGER.write().map_err(|_| "Patch manager mutex poisoned".to_string())?;
        // Use tokio runtime handle to run async code in blocking context
        match tokio::runtime::Handle::current()
            .block_on(manager.update_patches_from_remote(url.as_deref()))
        {
            Ok(output) => Ok(output),
            Err(e) => Err(e.to_string()),
        }
    })
    .await?;

    result.map_err(|e| e.into())
}

/// Process UPDATE-PATCH query - for use by query processor (async)
pub async fn process_update_patch_query() -> Result<String, Box<dyn std::error::Error>> {
    match update_patches_from_remote(None).await {
        Ok(output) => Ok(output),
        Err(e) => {
            let error_msg = format!(
                "% Patch Update Failed\n\
                 % Error: {}\n\
                 %\n\
                 % Please check:\n\
                 % - Internet connectivity\n\
                 % - GitHub repository accessibility\n\
                 % - LMDB storage permissions\n",
                e
            );
            Ok(error_msg)
        }
    }
}

/// Apply patches to a WHOIS response
pub fn apply_response_patches(query: &str, response: String) -> String {
    log_debug!("Applying patches for query: {}", query);
    let manager = PATCH_MANAGER.read().expect("Patch manager mutex poisoned in apply_response_patches");
    let result = manager.apply_patches(query, response);
    log_debug!("Patch application completed");
    result
}

/// Reload all patch files from LMDB storage
#[allow(dead_code)]
pub fn reload_patches(_patches_dir: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut manager = PATCH_MANAGER.write().map_err(|_| anyhow::anyhow!("Patch manager mutex poisoned"))?;
    manager.load_patches_from_storage()
}

/// Get the number of loaded patches
pub fn get_patches_count() -> (usize, usize) {
    let manager = PATCH_MANAGER.read().expect("Patch manager mutex poisoned in get_patches_count");
    let files = manager.patch_files.len();
    let patches = manager.patch_files.iter().map(|pf| pf.patches.len()).sum();
    (files, patches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_replacement() {
        let hunk = DiffHunk {
            remove_lines: vec!["RuiNetwork".to_string()],
            add_lines: vec!["Ruifeng Enterprise".to_string()],
            context_before: vec![],
            context_after: vec![],
        };

        let manager = PatchManager::new();
        let response = "netname: RuiNetwork".to_string();
        let excludes: Vec<String> = vec![];
        let context_rules: Vec<ContextRule> = vec![];
        let result = manager.apply_hunk(response, &hunk, &excludes, &context_rules);
        assert_eq!(result, "netname: Ruifeng Enterprise");
    }

    #[test]
    fn test_query_condition() {
        let condition = PatchCondition {
            condition_type: ConditionType::QueryContains,
            value: "RuiNetwork".to_string(),
            regex: None,
        };

        let manager = PatchManager::new();

        // Should match
        assert!(manager.check_conditions("AS-RuiNetwork", "", &[condition.clone()]));

        // Should not match
        assert!(!manager.check_conditions("AS12345", "", &[condition]));
    }

    #[test]
    fn test_response_condition() {
        let condition = PatchCondition {
            condition_type: ConditionType::ResponseContains,
            value: "RuiNetwork".to_string(),
            regex: None,
        };

        let manager = PatchManager::new();

        // Should match
        assert!(manager.check_conditions("", "netname: RuiNetwork", &[condition.clone()]));

        // Should not match
        assert!(!manager.check_conditions("", "netname: Other", &[condition]));
    }
}
