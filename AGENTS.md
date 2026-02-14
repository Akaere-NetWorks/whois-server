# AGENTS.md - Development Guidelines for whois-server

This file provides guidelines for AI agents working on this Rust WHOIS server project.

## Build, Test, and Lint Commands

### Build
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Test
```bash
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo test --lib         # Run only library tests
cargo test --bin whois-server  # Run only binary tests
```

### Lint/Format
```bash
cargo fmt                # Format code
cargo fmt --check        # Check formatting without modifying
cargo clippy             # Run linter
cargo clippy --fix       # Auto-fix minor issues
```

## Code Style Guidelines

### File Headers
Every Rust file must start with the AGPL-3.0 license header (see LICENSE_HEADER).

### Imports
External crates first, then local modules. Group related imports. Use `use crate::*` sparingly.

### Naming Conventions
- **Modules**: `snake_case` (e.g., `query_processor.rs`)
- **Functions**: `snake_case` (e.g., `query_whois()`)
- **Types/Structs/Enums**: `PascalCase` (e.g., `QueryType`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_WHOIS_SERVER`)
- **Private fields**: Leading underscore not required

### Formatting
Use standard `rustfmt` formatting. Line length: 100 chars preferred. Indentation: 4 spaces (no tabs). Trailing commas in multi-line structs/lists.

### Error Handling
Use `anyhow::Result<T>` for fallible functions. Use `anyhow::anyhow!("message")` for errors with context. Use `?` operator for propagation.

```rust
pub async fn query_whois(query: &str) -> Result<String> {
    let stream = TcpStream::connect(&address).await
        .map_err(|e| anyhow::anyhow!("Cannot connect: {}", e))?;
    Ok(response)
}
```

### Async Code
Use `tokio` for async runtime. Always specify timeout for network operations. Use `tokio::time::timeout()` wrapper.

```rust
use tokio::time::{timeout, Duration};
let result = timeout(Duration::from_secs(10), operation).await?;
```

### Logging
Use custom systemd-style logging macros (defined in `src/core/logger.rs`):
- Log levels: `log_info!`, `log_debug!`, `log_warn!`, `log_error!`, `log_crit!`, `log_emerg!`, `log_alert!`, `log_notice!`
- Init macros: `log_init_start!`, `log_init_ok!`, `log_init_failed!`, `log_init_warn!`
- Task macros: `log_task_start!`, `log_task_complete!`, `log_task_complete_with_details!`

```rust
log_debug!("Querying WHOIS server: {}", address);
log_init_start!("DN42 System");
log_init_ok_with_details!("Plugin System", &format!("{} plugins loaded", count));
log_error!("Failed to connect: {}", error);
```

### Module Organization
Each major feature in its own directory under `src/`. Modules have `mod.rs` listing submodules. Re-export common types with `pub use` in `mod.rs`.
```
src/
├── core/          # Core functionality
├── services/      # Query handlers for different query types
├── dn42/          # DN42 integration
├── plugins/       # Lua plugin system
├── web/           # Web dashboard
├── ssh/           # SSH server
└── config.rs      # Constants and CLI args
```

### Testing
Tests in `tests.rs` files or `#[cfg(test)]` modules. Use `#[test]` attribute.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        let query_type = analyze_query("example.com");
        assert!(matches!(query_type, QueryType::Domain(_)));
    }
}
```

### Documentation
Module-level docs with `//!`, function-level docs with `///`. Include examples in doc comments.

```rust
//! WHOIS query processor
//!
//! Handles all types of WHOIS queries including domains, IPs, ASNs, and custom query types.

/// Queries WHOIS server for given domain
///
/// # Example
/// ```ignore
/// let result = query_whois("example.com").await?;
/// ```
pub async fn query_whois(query: &str) -> Result<String> {
```

### Pattern Matching
Use `match` extensively for query type dispatch. Handle all variants explicitly. Use `_` for truly unknown/ignore cases.

### String Formatting
Use `format!()` for string building. Use `.to_uppercase()` for case-insensitive comparisons. Prefer string slices (`&str`) over owned `String` where possible.

### Configuration
Constants in `src/config.rs`. CLI args with `clap::Parser` derive macro. Environment variables loaded with `dotenv::dotenv()`.

### Dependencies
Check `Cargo.toml` before adding new crates. Prefer minimal dependency versions. Async-first: use tokio ecosystem.

### DO NOTs
- DO NOT add inline comments unless documenting complex logic
- DO NOT use unwrap/expect in production code; handle errors properly
- DO NOT commit sensitive data (check .dockerignore)
- DO NOT modify git config or run force commits
- DO NOT skip pre-commit hooks with --no-verify unless explicitly requested
- DO NOT use #[allow(...)] attributes extensively; fix the lint instead
