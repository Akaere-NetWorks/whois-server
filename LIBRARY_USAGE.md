# 🦀 Using whois-server as a Rust Library

## 📦 Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
whois-server = { git = "https://github.com/Akaere-NetWorks/whois-server.git" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

## 🚀 Quick Start

### Basic Usage

The simplest way to use this library is to call the `query()` function with a query string:

```rust
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Query a domain
    let result = query("example.com").await?;
    println!("{}", result);
    
    Ok(())
}
```

### Supported Query Types

This library supports all whois-server query features, including:

#### 1. Standard WHOIS Queries
```rust
// Domain query
let result = query("example.com").await?;

// IP address query
let result = query("1.1.1.1").await?;

// ASN query
let result = query("AS13335").await?;
```

#### 2. DN42 Network Queries
```rust
// DN42 domain
let result = query("example.dn42").await?;

// DN42 ASN
let result = query("AS4242420000").await?;

// DN42 private IP
let result = query("172.20.0.1").await?;
```

#### 3. Enhanced Query Features
```rust
// Geo-location query
let result = query("1.1.1.1-GEO").await?;

// BGP tools query
let result = query("1.1.1.0-BGPTOOL").await?;

// DNS query
let result = query("google.com-DNS").await?;

// IRR Explorer query
let result = query("192.0.2.0/24-IRR").await?;

// RPKI validation
let result = query("192.0.2.0/24-AS13335-RPKI").await?;

// Looking Glass query
let result = query("1.1.1.0-LG").await?;
```

#### 4. Network Diagnostic Tools
```rust
// Traceroute
let result = query("8.8.8.8-TRACE").await?;

// SSL certificate analysis
let result = query("example.com-SSL").await?;

// Certificate Transparency logs
let result = query("example.com-CRT").await?;
```

#### 5. Gaming and Entertainment Services
```rust
// Minecraft server status
let result = query("mc.hypixel.net-MC").await?;

// Steam game query
let result = query("730-STEAM").await?;

// IMDb movie query
let result = query("Inception-IMDB").await?;
```

#### 6. Package Repository Queries
```rust
// Cargo (Rust)
let result = query("tokio-CARGO").await?;

// NPM (Node.js)
let result = query("express-NPM").await?;

// PyPI (Python)
let result = query("requests-PYPI").await?;

// AUR (Arch Linux)
let result = query("firefox-AUR").await?;
```

#### 7. Developer Tools
```rust
// GitHub user/repository
let result = query("torvalds-GITHUB").await?;
let result = query("torvalds/linux-GITHUB").await?;

// Wikipedia query
let result = query("Rust-WIKIPEDIA").await?;
```

### Query with Color Scheme

If you want formatted colored output:

```rust
use whois_server::{query_with_color, ColorScheme};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use RIPE color scheme
    let result = query_with_color("example.com", Some(ColorScheme::Ripe)).await?;
    println!("{}", result);
    
    // Use BGPTools color scheme
    let result = query_with_color("AS13335", Some(ColorScheme::BgpTools)).await?;
    println!("{}", result);
    
    // No colors
    let result = query_with_color("1.1.1.1", None).await?;
    println!("{}", result);
    
    Ok(())
}
```

## 📚 Complete Examples

See `examples/library_usage.rs` for more examples:

```bash
cargo run --example library_usage
```

## 🔧 Advanced Usage

### Custom Query Type Parsing

If you need to manually parse query types:

```rust
use whois_server::{analyze_query, process_query, QueryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let input = "example.com";
    
    // Parse query type
    let query_type = analyze_query(input);
    println!("Query type: {:?}", query_type);
    
    // Process query
    let result = process_query(input, &query_type, None).await?;
    println!("{}", result);
    
    Ok(())
}
```

### Batch Queries

```rust
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let queries = vec![
        "example.com",
        "AS13335",
        "1.1.1.1-GEO",
    ];
    
    for q in queries {
        match query(q).await {
            Ok(result) => println!("=== {} ===\n{}\n", q, result),
            Err(e) => eprintln!("Error querying {}: {}", q, e),
        }
    }
    
    Ok(())
}
```

### Concurrent Queries

```rust
use whois_server::query;
use tokio::task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let queries = vec!["example.com", "AS13335", "1.1.1.1"];
    
    let mut handles = vec![];
    
    for q in queries {
        let q = q.to_string();
        let handle = task::spawn(async move {
            query(&q).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        match handle.await? {
            Ok(result) => println!("{}\n", result),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## 🎯 Real-World Use Cases

### 1. Network Monitoring Tool

```rust
use whois_server::query;
use std::time::Duration;

async fn monitor_network() -> anyhow::Result<()> {
    loop {
        // Check AS information
        if let Ok(result) = query("AS13335").await {
            println!("AS Status: {}", result);
        }
        
        // Check IP geo-location
        if let Ok(result) = query("1.1.1.1-GEO").await {
            println!("Geo Info: {}", result);
        }
        
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}
```

### 2. Automated BGP Analysis

```rust
use whois_server::query;

async fn analyze_prefix(prefix: &str) -> anyhow::Result<()> {
    // Basic information
    let whois_info = query(prefix).await?;
    println!("WHOIS: {}", whois_info);
    
    // IRR data
    let irr_info = query(&format!("{}-IRR", prefix)).await?;
    println!("IRR: {}", irr_info);
    
    // Looking Glass
    let lg_info = query(&format!("{}-LG", prefix)).await?;
    println!("Looking Glass: {}", lg_info);
    
    Ok(())
}
```

### 3. Domain Information Aggregation

```rust
use whois_server::query;

async fn domain_report(domain: &str) -> anyhow::Result<()> {
    // WHOIS information
    let whois = query(domain).await?;
    
    // DNS records
    let dns = query(&format!("{}-DNS", domain)).await?;
    
    // SSL certificate
    let ssl = query(&format!("{}-SSL", domain)).await?;
    
    // CT logs
    let ct = query(&format!("{}-CRT", domain)).await?;
    
    println!("=== Domain Report for {} ===", domain);
    println!("\n--- WHOIS ---\n{}", whois);
    println!("\n--- DNS ---\n{}", dns);
    println!("\n--- SSL ---\n{}", ssl);
    println!("\n--- CT Logs ---\n{}", ct);
    
    Ok(())
}
```

## 📖 API Reference

### `query(input: &str) -> anyhow::Result<String>`

Main query function that accepts any query string and returns the result.

**Parameters:**
- `input`: Query string (same format as whois command line)

**Returns:**
- `Ok(String)`: Query result
- `Err`: Error when query fails

### `query_with_color(input: &str, color_scheme: Option<ColorScheme>) -> anyhow::Result<String>`

Query function with color scheme support.

**Parameters:**
- `input`: Query string
- `color_scheme`: Optional color scheme
  - `Some(ColorScheme::Ripe)`: RIPE style
  - `Some(ColorScheme::BgpTools)`: BGPTools style
  - `None`: No colors

**Returns:**
- `Ok(String)`: Formatted query result
- `Err`: Error when query fails

### `analyze_query(query: &str) -> QueryType`

Parse query string and return query type.

**Parameters:**
- `query`: Query string

**Returns:**
- `QueryType`: Identified query type enum

### `process_query(query: &str, query_type: &QueryType, color_scheme: Option<ColorScheme>) -> anyhow::Result<String>`

Low-level query processing function that requires manual query type specification.

**Parameters:**
- `query`: Query string
- `query_type`: Query type
- `color_scheme`: Optional color scheme

**Returns:**
- `Ok(String)`: Query result
- `Err`: Error when query fails

## 🔗 Related Resources

- [Complete Feature Documentation](ADVANCED_FEATURES.md)
- [Project GitHub](https://github.com/Akaere-NetWorks/whois-server)
- [DN42 Network](https://dn42.dev/)

## 📝 License

AGPL-3.0-or-later

Copyright (C) 2025 Akaere Networks
