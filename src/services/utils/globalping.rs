//! Globalping API client for ping and traceroute measurements
//!
//! This module provides an async client for the Globalping API (https://globalping.io)
//! which performs network measurements from multiple vantage points worldwide.
//!
//! API documentation: https://globalping.io/docs

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use crate::{log_debug, log_error, log_warn};

const GLOBALPING_API_BASE: &str = "https://api.globalping.io/v1/measurements";
const MAX_POLL_ATTEMPTS: u32 = 60; // Maximum polling attempts (60 seconds)
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Globalping measurement request
#[derive(Debug, Serialize)]
pub struct GlobalpingRequest {
    #[serde(rename = "type")]
    pub measurement_type: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "measurementOptions")]
    pub measurement_options: Option<MeasurementOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<MeasurementLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "inProgressUpdates")]
    pub in_progress_updates: Option<bool>,
}

/// Measurement location filter
#[derive(Debug, Serialize, Clone)]
pub struct MeasurementLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Measurement options - different for ping and traceroute
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MeasurementOptions {
    Ping(PingOptions),
    Traceroute(TracerouteOptions),
}

/// Options specific to ping measurements
#[derive(Debug, Serialize)]
pub struct PingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
}

impl Default for PingOptions {
    fn default() -> Self {
        Self {
            packets: Some(4),
            protocol: Some("ICMP".to_string()),
            port: None,
        }
    }
}

/// Options specific to traceroute measurements
#[derive(Debug, Serialize)]
pub struct TracerouteOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
}

impl Default for TracerouteOptions {
    fn default() -> Self {
        Self {
            protocol: Some("ICMP".to_string()),
            port: None,
        }
    }
}

/// Globalping API response - submission confirmation
#[derive(Debug, Deserialize)]
pub struct GlobalpingResponse {
    pub id: String,
}

/// Globalping measurement results
#[derive(Debug, Deserialize)]
pub struct GlobalpingResult {
    #[serde(default)]
    pub results: Vec<ProbeResult>,
    pub status: String,
}

/// Individual measurement result from a probe
#[derive(Debug, Deserialize)]
pub struct ProbeResult {
    pub probe: ProbeInfo,
    pub result: TestResult,
}

/// Probe information
#[derive(Debug, Deserialize)]
pub struct ProbeInfo {
    pub continent: Option<String>,
    pub region: Option<String>,
    pub country: String,
    pub state: Option<String>,
    pub city: Option<String>,
    pub asn: u32,
    pub network: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Test result from probe
#[derive(Debug, Deserialize)]
pub struct TestResult {
    pub status: String,
    #[serde(default)]
    pub raw_output: Option<String>,
    #[serde(rename = "resolvedAddress")]
    pub resolved_address: Option<String>,
    #[serde(rename = "resolvedHostname")]
    pub resolved_hostname: Option<String>,
    #[serde(default)]
    pub timings: Option<Vec<Timing>>,
    #[serde(default)]
    pub stats: Option<Stats>,
    #[serde(default)]
    pub hops: Option<Vec<HopResult>>,
}

/// Timing information for ping
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Timing {
    pub rtt: f64,
}

/// Statistics for ping
#[derive(Debug, Deserialize)]
pub struct Stats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub total: u32,
    pub loss: u32,
    pub rcv: u32,
    #[serde(default)]
    pub drop: u32,
}

/// Traceroute hop result (from Globalping API)
/// The API returns an array of hops with resolved addresses and timings
#[derive(Debug, Deserialize, Clone)]
pub struct HopResult {
    #[serde(default)]
    pub hop: Option<u32>,  // Not always present in API response
    #[serde(default)]
    pub result: Option<Vec<HopDetail>>,
    #[serde(rename = "resolvedAddress")]
    #[serde(default)]
    pub resolved_address: Option<String>,
    #[serde(rename = "resolvedHostname")]
    #[serde(default)]
    pub resolved_hostname: Option<String>,
    #[serde(default)]
    pub timings: Option<Vec<HopTiming>>,
}

/// Individual hop detail (from old format - kept for compatibility)
#[derive(Debug, Deserialize, Clone)]
pub struct HopDetail {
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub rtt: Option<f64>,
}

/// Hop timing from traceroute response
#[derive(Debug, Deserialize, Clone)]
pub struct HopTiming {
    pub rtt: f64,
}

/// Target information (for compatibility)
#[derive(Debug, Deserialize)]
pub struct TargetInfo {
    pub address: String,
}

/// Hop information (for compatibility - old format)
#[derive(Debug, Deserialize)]
pub struct Hop {
    pub hop: u32,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub rtt: Vec<LatencyValue>,
}

/// Latency value (for compatibility - old format)
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
pub enum LatencyValue {
    Number(f64),
    Null,
}

impl LatencyValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            LatencyValue::Number(n) if *n >= 0.0 => Some(*n),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, LatencyValue::Null)
    }
}

/// Measurement result (for compatibility - old format)
#[derive(Debug, Deserialize)]
pub struct MeasurementResult {
    pub target: TargetInfo,
    #[serde(default)]
    pub hops: Option<Vec<Hop>>,
    #[serde(default)]
    pub packets_sent: Option<u32>,
    #[serde(default)]
    pub packets_received: Option<u32>,
    #[serde(rename = "rtt", default)]
    pub rtt: Option<f64>,
    #[serde(default)]
    pub latency: Option<Vec<LatencyValue>>,
}

/// Client for Globalping API
pub struct GlobalpingClient {
    client: Client,
    api_token: Option<String>,
}

impl GlobalpingClient {
    /// Create a new Globalping client
    ///
    /// API token is optional - Globalping API is public and free to use
    /// but provides higher limits with authentication
    pub fn new() -> Result<Self> {
        let api_token = std::env::var("GLOBALPING_API_TOKEN").ok();

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("whois-server/1.0")
            .build()?;

        log_debug!("Globalping client initialized (authenticated: {})", api_token.is_some());

        Ok(Self { client, api_token })
    }

    /// Submit a measurement request to Globalping
    ///
    /// Returns the measurement ID for polling results
    pub async fn submit_measurement(&self, request: &GlobalpingRequest) -> Result<String> {
        log_debug!("Submitting {} measurement to {}", request.measurement_type, request.target);

        // Log the request JSON for debugging
        let request_json = serde_json::to_string_pretty(request)
            .unwrap_or_else(|_| "[Failed to serialize]".to_string());
        log_debug!("Request JSON:\n{}", request_json);

        let mut req_builder = self.client
            .post(GLOBALPING_API_BASE)
            .header("Content-Type", "application/json");

        // Add authorization header if token is available
        if let Some(token) = &self.api_token {
            req_builder = req_builder.header("Authorization", &format!("Bearer {}", token));
        }

        let response = req_builder
            .json(request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to submit measurement: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            log_error!("Globalping API error: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("Globalping API returned error: {} - {}", status, error_text));
        }

        let result: GlobalpingResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse Globalping response: {}", e))?;

        log_debug!("Measurement submitted successfully, ID: {}", result.id);
        Ok(result.id)
    }

    /// Get measurement results by ID
    ///
    /// Returns the current state of the measurement
    pub async fn get_results(&self, id: &str) -> Result<GlobalpingResult> {
        let url = format!("{}/{}", GLOBALPING_API_BASE, id);

        let mut req_builder = self.client.get(&url);

        // Add authorization header if token is available
        if let Some(token) = &self.api_token {
            req_builder = req_builder.header("Authorization", &format!("Bearer {}", token));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get measurement results: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            return Err(anyhow::anyhow!("Globalping API error: {} - {}", status, error_text));
        }

        let result: GlobalpingResult = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse Globalping result: {}", e))?;

        Ok(result)
    }

    /// Wait for measurement to complete and return results
    ///
    /// Polls the measurement status until it completes or times out
    pub async fn wait_for_results(&self, id: &str, timeout_secs: u64) -> Result<GlobalpingResult> {
        log_debug!("Waiting for measurement {} to complete (timeout: {}s)", id, timeout_secs);

        let max_attempts = timeout_secs.min(MAX_POLL_ATTEMPTS as u64);
        let mut attempts = 0;

        loop {
            let result = self.get_results(id).await?;

            // Check if measurement is finished
            if result.status == "finished" {
                log_debug!("Measurement {} completed after {} attempts", id, attempts + 1);
                return Ok(result);
            }

            // Check if measurement failed
            if result.status == "failed" {
                return Err(anyhow::anyhow!("Measurement {} failed", id));
            }

            attempts += 1;
            if attempts >= max_attempts {
                log_warn!("Measurement {} timed out after {} attempts", id, attempts);
                return Err(anyhow::anyhow!("Measurement timed out after {} seconds", timeout_secs));
            }

            log_debug!("Measurement {} in progress (status: {}), polling... ({}/{})",
                id, result.status, attempts, max_attempts);
            sleep(POLL_INTERVAL).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_value_parsing() {
        // Test that we can handle different latency value formats
        let valid = LatencyValue::Number(10.5);
        assert_eq!(valid.as_f64(), Some(10.5));
        assert!(!valid.is_null());

        let null = LatencyValue::Null;
        assert_eq!(null.as_f64(), None);
        assert!(null.is_null());

        let negative = LatencyValue::Number(-1.0);
        assert_eq!(negative.as_f64(), None);
    }
}
