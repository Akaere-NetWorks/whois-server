//! Utility modules for network and API services

pub mod doh;
pub mod globalping;
pub mod ip_info;

// Re-export commonly used types from doh
pub use doh::{DohClient, DohResponse, DohAnswer};

// Re-export commonly used types from globalping
pub use globalping::{
    GlobalpingClient, GlobalpingRequest, GlobalpingResponse, GlobalpingResult,
    ProbeResult, TestResult, ProbeInfo, Timing, Stats, HopResult, HopDetail, HopTiming,
    MeasurementOptions, PingOptions, TracerouteOptions, MeasurementLocation
};

// Re-export commonly used types from ip_info
pub use ip_info::{IpInfo, IpInfoClient};
