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

use std::fs;
use std::path::Path;
use regex::Regex;
use tracing::{ debug, warn, error, info };
use once_cell::sync::Lazy;
use std::sync::RwLock;

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
    /// All diff hunks in this patch
    pub hunks: Vec<DiffHunk>,
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
    /// Query must contain keyword
    QueryContains,
    /// Response must contain keyword
    ResponseContains,
    /// Query must match regex
    QueryMatches,
    /// Response must match regex
    ResponseMatches,
}

/// Collection of patches from a single file
#[derive(Debug, Clone)]
pub struct PatchFile {
    pub filename: String,
    pub patches: Vec<Patch>,
}

/// Global patch manager
static PATCH_MANAGER: Lazy<RwLock<PatchManager>> = Lazy::new(|| {
    RwLock::new(PatchManager::new())
});

/// Manages all patch files
pub struct PatchManager {
    patch_files: Vec<PatchFile>,
    loaded: bool,
}

impl PatchManager {
    /// Create a new patch manager
    pub fn new() -> Self {
        PatchManager {
            patch_files: Vec::new(),
            loaded: false,
        }
    }

    /// Load all patch files from the patches directory
    pub fn load_patches(&mut self, patches_dir: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let path = Path::new(patches_dir);

        if !path.exists() {
            warn!("Patches directory does not exist: {}", patches_dir);
            return Ok(0);
        }

        self.patch_files.clear();
        let mut total_patches = 0;

        // Read and sort patch files by filename (001-, 002-, etc.)
        let mut entries: Vec<_> = fs
            ::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e
                    .path()
                    .extension()
                    .and_then(|s| s.to_str()) == Some("patch")
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let file_path = entry.path();

            match self.load_patch_file(&file_path) {
                Ok(patch_file) => {
                    debug!(
                        "Loaded patch file: {} ({} patches)",
                        patch_file.filename,
                        patch_file.patches.len()
                    );
                    total_patches += patch_file.patches.len();
                    self.patch_files.push(patch_file);
                }
                Err(e) => {
                    error!("Failed to load patch file {:?}: {}", file_path, e);
                }
            }
        }

        self.loaded = true;
        info!("Loaded {} patch files with {} total patches", self.patch_files.len(), total_patches);
        Ok(total_patches)
    }

    /// Load a single patch file
    fn load_patch_file(&self, path: &Path) -> Result<PatchFile, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let lines: Vec<&str> = content.lines().collect();
        let mut patches = Vec::new();
        let mut current_conditions: Vec<PatchCondition> = Vec::new();
        let mut current_excludes: Vec<String> = Vec::new();
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

            // Parse condition headers
            if line.starts_with("# QUERY_CONTAINS:") {
                let value = line.trim_start_matches("# QUERY_CONTAINS:").trim().to_string();
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::QueryContains,
                    value,
                    regex: None,
                });
                i += 1;
                continue;
            }

            if line.starts_with("# RESPONSE_CONTAINS:") {
                let value = line.trim_start_matches("# RESPONSE_CONTAINS:").trim().to_string();
                current_conditions.push(PatchCondition {
                    condition_type: ConditionType::ResponseContains,
                    value,
                    regex: None,
                });
                i += 1;
                continue;
            }

            if line.starts_with("# QUERY_MATCHES:") {
                let pattern = line.trim_start_matches("# QUERY_MATCHES:").trim().to_string();
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
                let pattern = line.trim_start_matches("# RESPONSE_MATCHES:").trim().to_string();
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
                hunks: current_hunks,
            });
        }

        Ok(PatchFile { filename, patches })
    }

    /// Parse a single diff hunk
    fn parse_diff_hunk(
        &self,
        lines: &[&str],
        index: &mut usize
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
            if
                line.trim().starts_with("---") ||
                line.trim().starts_with("# QUERY_") ||
                line.trim().starts_with("# RESPONSE_")
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
            Ok(
                Some(DiffHunk {
                    remove_lines,
                    add_lines,
                    context_before,
                    context_after,
                })
            )
        } else {
            Ok(None)
        }
    }

    /// Apply all patches to a response
    pub fn apply_patches(&self, query: &str, mut response: String) -> String {
        if !self.loaded || self.patch_files.is_empty() {
            debug!("No patches loaded or patch system not initialized");
            return response;
        }

        debug!("Processing {} patch files", self.patch_files.len());
        for patch_file in &self.patch_files {
            debug!("Checking {} patches from file", patch_file.patches.len());
            for patch in &patch_file.patches {
                if self.check_conditions(query, &response, &patch.conditions) {
                    debug!("Conditions matched, applying patch with {} hunks", patch.hunks.len());
                    response = self.apply_patch(response, patch);
                } else {
                    debug!(
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
            debug!("No conditions - patch will always apply");
            return true; // No conditions means always apply
        }

        debug!("Checking {} conditions (OR logic)", conditions.len());
        // OR logic: if any condition is true, apply the patch
        for condition in conditions {
            let result = match condition.condition_type {
                ConditionType::QueryContains => {
                    let matches = query.contains(&condition.value);
                    debug!("QUERY_CONTAINS '{}': {}", condition.value, matches);
                    matches
                }
                ConditionType::ResponseContains => {
                    let matches = response.contains(&condition.value);
                    debug!("RESPONSE_CONTAINS '{}': {}", condition.value, matches);
                    matches
                }
                ConditionType::QueryMatches => {
                    if let Some(regex) = &condition.regex {
                        let matches = regex.is_match(query);
                        debug!("QUERY_MATCHES '{}': {}", condition.value, matches);
                        matches
                    } else {
                        false
                    }
                }
                ConditionType::ResponseMatches => {
                    if let Some(regex) = &condition.regex {
                        let matches = regex.is_match(response);
                        debug!("RESPONSE_MATCHES '{}': {}", condition.value, matches);
                        matches
                    } else {
                        false
                    }
                }
            };

            if result {
                debug!("Condition matched! Patch will be applied");
                return true; // Any condition being true is enough
            }
        }

        false // No conditions matched
    }

    /// Apply a single patch
    fn apply_patch(&self, mut response: String, patch: &Patch) -> String {
        for hunk in &patch.hunks {
            response = self.apply_hunk(response, hunk, &patch.excludes);
        }
        response
    }

    /// Apply a single diff hunk
    fn apply_hunk(&self, response: String, hunk: &DiffHunk, excludes: &[String]) -> String {
        if hunk.remove_lines.is_empty() {
            return response;
        }

        // Simple case: direct replacement
        if hunk.remove_lines.len() == 1 && hunk.add_lines.len() == 1 {
            let old = &hunk.remove_lines[0];
            let new = &hunk.add_lines[0];

            // Apply replacement line by line, skipping excluded patterns
            return response
                .lines()
                .map(|line| {
                    // Check if this line should be excluded
                    for exclude_pattern in excludes {
                        if line.contains(exclude_pattern) {
                            debug!(
                                "Skipping replacement for excluded line: {}",
                                line.chars().take(60).collect::<String>()
                            );
                            return line.to_string();
                        }
                    }
                    // Apply replacement
                    line.replace(old, new)
                })
                .collect::<Vec<_>>()
                .join("\n");
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
                debug!("Skipping multi-line replacement due to excluded pattern: {}", exclude_pattern);
                return response;
            }
        }

        response.replace(&old_text, &new_text)
    }
}

/// Initialize the patch system
pub fn init_patches(patches_dir: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut manager = PATCH_MANAGER.write().unwrap();
    manager.load_patches(patches_dir)
}

/// Apply patches to a WHOIS response
pub fn apply_response_patches(query: &str, response: String) -> String {
    debug!("Applying patches for query: {}", query);
    let manager = PATCH_MANAGER.read().unwrap();
    let result = manager.apply_patches(query, response);
    debug!("Patch application completed");
    result
}

/// Reload all patch files
#[allow(dead_code)]
pub fn reload_patches(patches_dir: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut manager = PATCH_MANAGER.write().unwrap();
    manager.load_patches(patches_dir)
}

/// Get the number of loaded patches
pub fn get_patches_count() -> (usize, usize) {
    let manager = PATCH_MANAGER.read().unwrap();
    let files = manager.patch_files.len();
    let patches = manager.patch_files
        .iter()
        .map(|pf| pf.patches.len())
        .sum();
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
        let result = manager.apply_hunk(response, &hunk, &excludes);
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
