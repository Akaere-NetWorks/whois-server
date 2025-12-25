//! # WHOIS Server Library
//!
//! A comprehensive WHOIS query library with support for:
//! - Standard WHOIS queries (domains, IPs, ASNs)
//! - DN42 network integration
//! - Enhanced features (geo-location, BGP tools, DNS, SSL, etc.)
//! - Package repository queries
//! - Entertainment and development tools
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! whois-server = { git = "https://github.com/Akaere-NetWorks/whois-server.git" }
//! tokio = { version = "1.35", features = ["full"] }
//! anyhow = "1.0"
//! ```
//!
//! Basic usage:
//! ```no_run
//! use whois_server::query;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let result = query("example.com").await?;
//!     println!("{}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Supported Query Types
//!
//! ### Standard WHOIS
//! - Domains: `query("example.com")`
//! - IPv4: `query("1.1.1.1")`
//! - IPv6: `query("2001:4860:4860::8888")`
//! - ASN: `query("AS13335")`
//!
//! ### Enhanced Queries (add suffix to query)
//! - Geo-location: `query("1.1.1.1-GEO")`
//! - BGP Tools: `query("1.1.1.0-BGPTOOL")`
//! - DNS records: `query("google.com-DNS")`
//! - IRR Explorer: `query("192.0.2.0/24-IRR")`
//! - Looking Glass: `query("1.1.1.0-LG")`
//! - RPKI validation: `query("192.0.2.0/24-AS13335-RPKI")`
//! - SSL certificate: `query("example.com-SSL")`
//! - Traceroute: `query("8.8.8.8-TRACE")`
//!
//! ### Package Repositories
//! - Cargo: `query("tokio-CARGO")`
//! - NPM: `query("express-NPM")`
//! - PyPI: `query("requests-PYPI")`
//! - GitHub: `query("torvalds-GITHUB")`
//!
//! ### Entertainment & Tools
//! - Minecraft server: `query("mc.hypixel.net-MC")`
//! - Steam game: `query("730-STEAM")`
//! - Wikipedia: `query("Rust-WIKIPEDIA")`
//! - Help: `query("HELP")`
//!
//! For complete documentation, see [LIBRARY_USAGE.md](https://github.com/Akaere-NetWorks/whois-server/blob/main/LIBRARY_USAGE.md)

pub mod config;
pub mod core;
pub mod dn42;
pub mod plugins;
pub mod server;
pub mod services;
pub mod ssh;
pub mod storage;
pub mod web;

// Re-export commonly used types for convenience
pub use core::query_processor::process_query;
pub use core::{ ColorScheme, QueryType, analyze_query };

/// Simple API for querying WHOIS information
///
/// This is the main entry point for using this crate as a library.
/// Pass in any query string just like you would to the whois command.
///
/// # Examples
///
/// ```no_run
/// use whois_server::query;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Query a domain
///     let result = query("example.com").await?;
///     println!("{}", result);
///
///     // Query an ASN
///     let result = query("AS13335").await?;
///     println!("{}", result);
///
///     // Query with special suffix
///     let result = query("1.1.1.0-GEO").await?;
///     println!("{}", result);
///
///     Ok(())
/// }
/// ```
pub async fn query(input: &str) -> anyhow::Result<String> {
    let query_type = analyze_query(input);
    process_query(input, &query_type, None, None).await
}

/// Query with color scheme support
///
/// Same as `query()` but with optional color scheme for formatted output.
///
/// # Examples
///
/// ```no_run
/// use whois_server::{query_with_color, ColorScheme};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let result = query_with_color("example.com", Some(ColorScheme::Dark)).await?;
///     println!("{}", result);
///     Ok(())
/// }
/// ```
pub async fn query_with_color(
    input: &str,
    color_scheme: Option<ColorScheme>
) -> anyhow::Result<String> {
    let query_type = analyze_query(input);
    process_query(input, &query_type, color_scheme, None).await
}
