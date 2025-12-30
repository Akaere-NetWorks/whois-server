//! Utility modules for network and API services

pub mod doh;
pub mod globalping;
pub mod ip_info;

// Re-export commonly used types from doh
pub use doh::DohClient;

// Re-export commonly used types from globalping
#[allow(dead_code)]
pub use globalping::{
    GlobalpingClient, GlobalpingRequest, GlobalpingResult,
    MeasurementOptions, PingOptions, TracerouteOptions, MeasurementLocation
};

// Re-export commonly used types from ip_info
pub use ip_info::IpInfoClient;
