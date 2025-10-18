# 📦 Using whois-server as a Rust Library

## ✅ Completed Features

The project now fully supports being used as a Rust library by other projects!

### Core Changes

1. **Updated `src/lib.rs`**
   - Added detailed module-level documentation
   - Exported main public APIs: `query()` and `query_with_color()`
   - Exported commonly used types: `QueryType`, `ColorScheme`, `analyze_query`

2. **Updated `Cargo.toml`**
   - Explicitly defined library and binary targets
   - Added package metadata (description, repository, keywords, categories)
   - Configured correct library name `whois_server`

3. **Created Example Programs**
   - `examples/simple_query.rs` - Simplest usage example
   - `examples/library_usage.rs` - Complete feature demonstration

4. **Updated Documentation**
   - `LIBRARY_USAGE.md` - Detailed usage guide
   - `README.md` - Added library usage section
   - `ADVANCED_FEATURES.md` - Added library usage chapter

## 🚀 How to Use

### 1. Add Dependency to Other Projects

In your `Cargo.toml`:

```toml
[dependencies]
whois-server = { git = "https://github.com/Akaere-NetWorks/whois-server.git" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

### 2. Use the Simple API

```rust
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Pass in query just like using whois command
    let result = query("example.com").await?;
    println!("{}", result);
    
    Ok(())
}
```

### 3. All Query Types Supported

```rust
// Standard WHOIS queries
query("example.com").await?;        // Domain
query("1.1.1.1").await?;            // IP address
query("AS13335").await?;            // ASN

// DN42 queries
query("example.dn42").await?;       // DN42 domain
query("AS4242420000").await?;       // DN42 ASN
query("172.20.0.1").await?;         // DN42 private IP

// Enhanced features
query("1.1.1.1-GEO").await?;        // Geo-location
query("1.1.1.0-BGPTOOL").await?;    // BGP tools
query("google.com-DNS").await?;     // DNS query
query("192.0.2.0/24-IRR").await?;   // IRR Explorer
query("1.1.1.0-LG").await?;         // Looking Glass
query("example.com-SSL").await?;    // SSL certificate
query("8.8.8.8-TRACE").await?;      // Traceroute

// Package repository queries
query("tokio-CARGO").await?;        // Cargo (Rust)
query("express-NPM").await?;        // NPM (Node.js)
query("requests-PYPI").await?;      // PyPI (Python)
query("firefox-AUR").await?;        // AUR (Arch)

// Entertainment and tools
query("mc.hypixel.net-MC").await?;  // Minecraft server
query("730-STEAM").await?;          // Steam game
query("torvalds-GITHUB").await?;    // GitHub user
query("Rust-WIKIPEDIA").await?;     // Wikipedia
query("HELP").await?;               // Help information
```

## 📚 Run Examples

```bash
# Simple example
cargo run --example simple_query

# Complete feature demonstration
cargo run --example library_usage
```

## 📖 View Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate library documentation only (without dependencies)
cargo doc --lib --no-deps
```

## 🔧 API Documentation

### `query(input: &str) -> anyhow::Result<String>`

**Main query function** - Accepts any query string and returns the result.

- **Parameter**: `input` - Query string (same format as whois command line)
- **Returns**: Query result string on success, error on failure

### `query_with_color(input: &str, color_scheme: Option<ColorScheme>) -> anyhow::Result<String>`

**Query function with color scheme** - Supports formatted output.

- **Parameters**: 
  - `input` - Query string
  - `color_scheme` - Optional color scheme:
    - `Some(ColorScheme::Ripe)` - RIPE style
    - `Some(ColorScheme::BgpTools)` - BGPTools style
    - `None` - No colors
- **Returns**: Formatted query result

### `analyze_query(query: &str) -> QueryType`

**Query type parsing function** - Identifies query type.

- **Parameter**: `query` - Query string
- **Returns**: `QueryType` enum

### `process_query(query: &str, query_type: &QueryType, color_scheme: Option<ColorScheme>) -> anyhow::Result<String>`

**Low-level query processing function** - Requires manual query type specification.

- **Parameters**: 
  - `query` - Query string
  - `query_type` - Query type
  - `color_scheme` - Optional color scheme
- **Returns**: Query result

## 🎯 Real-World Use Case Examples

### Network Monitoring

```rust
use whois_server::query;
use std::time::Duration;

async fn monitor_as() -> anyhow::Result<()> {
    loop {
        match query("AS13335").await {
            Ok(info) => println!("AS Info: {}", info),
            Err(e) => eprintln!("Error: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}
```

### Batch Domain Analysis

```rust
use whois_server::query;

async fn analyze_domains(domains: Vec<&str>) -> anyhow::Result<()> {
    for domain in domains {
        // WHOIS information
        let whois = query(domain).await?;
        println!("WHOIS: {}", whois);
        
        // DNS records
        let dns = query(&format!("{}-DNS", domain)).await?;
        println!("DNS: {}", dns);
        
        // SSL certificate
        let ssl = query(&format!("{}-SSL", domain)).await?;
        println!("SSL: {}", ssl);
    }
    Ok(())
}
```

### Concurrent Queries

```rust
use whois_server::query;
use tokio::task;

async fn concurrent_queries() -> anyhow::Result<()> {
    let queries = vec!["example.com", "AS13335", "1.1.1.1"];
    
    let handles: Vec<_> = queries.iter().map(|q| {
        let q = q.to_string();
        task::spawn(async move { query(&q).await })
    }).collect();
    
    for handle in handles {
        match handle.await? {
            Ok(result) => println!("{}", result),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## ✨ Advantages

1. **Zero Configuration** - Direct import and use, no additional configuration needed
2. **Type Safety** - Full Rust type system support
3. **Async First** - High-performance async implementation based on tokio
4. **Feature Complete** - Supports 25+ query types
5. **Error Handling** - Uses `anyhow::Result` for clear error messages
6. **Well Documented** - Detailed documentation and examples

## 📝 License

AGPL-3.0-or-later

Copyright (C) 2025 Akaere Networks

## 🔗 Related Links

- [Complete Usage Documentation (LIBRARY_USAGE.md)](LIBRARY_USAGE.md)
- [Advanced Features Documentation (ADVANCED_FEATURES.md)](ADVANCED_FEATURES.md)
- [Project Repository](https://github.com/Akaere-NetWorks/whois-server)
