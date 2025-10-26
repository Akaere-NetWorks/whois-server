/*
 * JSON Output Formatter for WHOIS API
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WhoisApiResponse {
    pub success: bool,
    pub query: String,
    pub query_type: String,
    pub raw_output: Option<String>,
    pub fields: Option<Vec<WhoisField>>,
    pub error: Option<String>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhoisField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub timestamp: String,
    pub processing_time_ms: u64,
    pub source: String,
    pub version: String,
}

pub struct JsonFormatter {}

impl JsonFormatter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn format_response(
        &self,
        query: &str,
        raw_output: String,
        query_type: &str,
        processing_time_ms: u64,
    ) -> WhoisApiResponse {
        let fields = self.parse_whois_fields(&raw_output);

        WhoisApiResponse {
            success: !raw_output.trim().is_empty(),
            query: query.to_string(),
            query_type: query_type.to_string(),
            raw_output: Some(raw_output),
            fields: if fields.is_empty() {
                None
            } else {
                Some(fields)
            },
            error: None,
            metadata: ResponseMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                processing_time_ms,
                source: "whois-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    pub fn format_error(
        &self,
        query: &str,
        error_message: &str,
        query_type: &str,
        processing_time_ms: u64,
    ) -> WhoisApiResponse {
        WhoisApiResponse {
            success: false,
            query: query.to_string(),
            query_type: query_type.to_string(),
            raw_output: None,
            fields: None,
            error: Some(error_message.to_string()),
            metadata: ResponseMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                processing_time_ms,
                source: "whois-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    fn parse_whois_fields(&self, raw_output: &str) -> Vec<WhoisField> {
        let mut fields: Vec<WhoisField> = Vec::new();

        for line in raw_output.lines() {
            // 跳过注释行和空行
            let line = line.trim();
            if line.is_empty() || line.starts_with('%') || line.starts_with('#') {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();

                if !name.is_empty() && !value.is_empty() {
                    fields.push(WhoisField { name, value });
                }
            }
        }

        fields
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}
