# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a high-performance WHOIS server implemented in Rust with DN42 support, geo-location services, and comprehensive query capabilities. The server implements RFC 3912 with custom extensions and provides both WHOIS protocol service (port 43) and a modern web dashboard (port 9999).

## Build and Development Commands

```bash
# Build the project in release mode
cargo build --release

# Run with default settings (WHOIS on port 43, Web on port 9999)
cargo run --release

# Run with custom configuration
cargo run --release -- --port 4343 --web-port 8080 --debug

# Run in blocking mode (alternative server implementation)
cargo run --release -- --use-blocking

# Enable traffic dumping for debugging
cargo run --release -- --dump-traffic --dump-dir ./logs

# Check for compilation errors and warnings
cargo check

# Build with detailed error messages
cargo build --message-format=short
```

### Available Command-Line Options
- `--host`: Listen address (default: 0.0.0.0)
- `--port`: WHOIS server port (default: 43)
- `--web-port`: Web dashboard port (default: 9999)
- `--debug`: Enable debug output
- `--trace`: Enable trace output (extremely verbose)
- `--max-connections`: Maximum concurrent connections (default: 100)
- `--timeout`: Connection timeout in seconds (default: 10)
- `--dump-traffic`: Write raw queries/responses to files for debugging
- `--dump-dir`: Directory for traffic dumps (default: dumps)
- `--use-blocking`: Use blocking (non-async) network operations

### Testing
The project does not include traditional unit tests. Testing is performed by running the server and querying it with WHOIS clients:

```bash
# Test with standard WHOIS client
whois -h localhost -p 43 example.com

# Test with netcat
echo "example.com" | nc localhost 43

# Test web dashboard
curl http://localhost:9999/api/stats

# Test against the public deployment instance
whois -h whois.akae.re example.com
whois -h whois.akae.re AS213605-MANRS
whois -h whois.akae.re 8.8.8.8-GEO
```

## Architecture Overview

The codebase is organized into logical modules for maintainability and clarity:

### Module Structure
- **`core/`**: Core application logic (query processing, statistics, utilities)
- **`server/`**: TCP server implementations (async and blocking)
- **`services/`**: External service integrations (WHOIS, BGP, geo-location, etc.)
- **`storage/`**: Data persistence layer (LMDB storage)
- **`web/`**: Web dashboard and HTTP API
- **`dn42/`**: DN42 network support with platform-aware backends
- **`config.rs`**: Configuration and command-line parsing
- **`main.rs`**: Application entry point

### Core Components

**Core Module (`core/`)**:
- `query.rs`: Query type detection and routing (11+ query types)
- `stats.rs`: Real-time statistics collection and persistence
- `utils.rs`: Shared utility functions

**Server Module (`server/`)**:
Two server architectures available:
- `async_server.rs`: Tokio-based async server (default, high performance)
- `blocking_server.rs`: Blocking server (fallback for compatibility)
- `connection.rs`: Connection handling and query processing pipeline

**Services Module (`services/`)**:
- `whois.rs`: Standard WHOIS protocol client implementations
- `geo/`: Complete geo-location service with RIPE and IPInfo integrations
- `bgptool.rs`: BGP tools integration for network analysis
- `irr.rs`: IRR Explorer integration for routing registry analysis
- `looking_glass.rs`: RIPE RIS Looking Glass services with BIRD-style output
- `email.rs`: Email search functionality across registry data
- `rpki.rs`: RPKI validation services for prefix-ASN validation
- `manrs.rs`: MANRS (Mutually Agreed Norms for Routing Security) integration
- `dns.rs`: DNS resolution service with fixed 1.1.1.1 DNS server for enhanced queries
- `traceroute.rs`: Network traceroute functionality for path analysis
- `iana_cache.rs`: IANA registry data caching for efficient lookups

**DN42 Module (`dn42/`)**:
Platform-aware DN42 implementation with automatic backend selection:
- `manager.rs`: Platform detection and backend orchestration
- `git_backend.rs`: Git repository-based backend (Unix-like systems)
- `online_backend.rs`: HTTP API-based backend (Windows systems)
- `query.rs`: DN42-specific query processing and formatting

**Storage Module (`storage/`)**:
- `lmdb.rs`: LMDB-based storage for caching and persistence

**Web Module (`web/`)**:
- `dashboard.rs`: Axum-based web dashboard and REST API endpoints

### Platform-Aware DN42 Implementation

The DN42 module automatically detects the operating system and uses the appropriate backend:

**Windows Systems**: Uses online file access via `https://git.pysio.online/pysio/mirrors-dn42/-/raw/master/data`
- Direct HTTP fetching of DN42 registry files
- LMDB-based caching with 1-day expiration
- No Git dependency required

**Unix-like Systems**: Uses Git repository cloning and synchronization
- Local Git repository with periodic updates
- LMDB caching for fast access
- Full offline operation capability

The `DN42Manager` in `dn42/manager.rs` handles this platform detection and provides a unified interface regardless of the underlying implementation.

### Query Types Supported
1. Standard WHOIS: domains, IPv4/IPv6, ASNs, CIDR blocks
2. Enhanced queries: -EMAIL, -BGPTOOL, -PREFIXES suffixes
3. Geo-location: -GEO, -RIRGEO suffixes
4. Advanced routing: -IRR, -LG, -RADB suffixes
5. Security validation: -RPKI (prefix-ASN-RPKI format), -MANRS suffixes
6. Network diagnostics: -DNS, -TRACEROUTE suffixes
7. DN42-specific queries (auto-detected)

### Intelligent Query Routing
- Automatic DN42 detection for AS4242420000-AS4242423999, .dn42 domains, private IPs
- Smart referral following (IANA → specific registries)
- Automatic fallback to DN42 for failed public queries
- Multi-source data aggregation

### External Integrations
- **RIPE Database**: Primary data source for European IP allocations
- **IPInfo API**: Commercial geo-location data
- **IRR Explorer (irrexplorer.nlnog.net)**: Routing registry analysis with RPKI validation
- **RIPE RIS**: Real-time BGP routing data for Looking Glass services
- **RADB**: Direct access to Routing Assets Database
- **DN42 Registry**: Comprehensive DN42 network support with platform-aware caching
- **Cloudflare DNS (1.1.1.1)**: Fixed DNS server for DNS resolution queries
- **IANA Registry**: Cached registry data for efficient resource allocation lookups

### Statistics and Monitoring
The stats module provides comprehensive metrics:
- Request counts, response times, data transfer
- Query type distribution and geographic analytics
- 24-hour and 30-day historical data retention
- Real-time dashboard with auto-refresh (30-second intervals)
- JSON API endpoint at `/api/stats`

### Configuration Management
Configuration is handled through:
- Command-line arguments (primary)
- No configuration files required
- Runtime configuration via web dashboard

### Data Persistence
- Statistics: JSON-based persistence on shutdown/startup
- DN42 data: LMDB-based caching with different strategies per platform
- Traffic dumps: Optional raw query/response logging for debugging

## Key Dependencies
- **tokio**: Async runtime (with "full" features)
- **axum**: Web framework for dashboard
- **reqwest**: HTTP client with rustls-tls
- **clap**: Command-line parsing with derive features
- **tracing**: Structured logging
- **lmdb**: Lightning Memory-Mapped Database for caching
- **cidr**: CIDR block handling
- **regex**: Pattern matching for query analysis

## Performance Characteristics
- Dual server architecture (async/blocking)
- Platform-aware DN42 backend selection
- Configurable connection limits and timeouts
- Non-blocking external API calls
- Efficient connection pooling
- Real-time statistics with minimal overhead
- LMDB-based caching reduces external API calls

## Development Notes
- Rust 2024 edition
- AGPL-3.0-or-later license
- No traditional test suite - integration testing via live queries
- Extensive logging with configurable verbosity levels
- Modular architecture for easy maintenance and extension
- Cross-platform compatibility with automatic feature detection

## Web Dashboard Features
- Real-time statistics with auto-refresh
- Interactive WHOIS query testing interface
- Light/dark/auto theme support with pink-themed UI
- Responsive design for desktop and mobile
- Charts and visual analytics with 24h/30d views
- Query type distribution and performance metrics

## Important Implementation Details

### Recent Updates
- **DNS Service Optimization**: The DNS service has been optimized to use Cloudflare's 1.1.1.1 as a fixed DNS server instead of multiple root servers for improved reliability and performance
- **Enhanced Network Services**: Added traceroute and IANA cache services for comprehensive network analysis
- **Platform-Aware Caching**: Improved LMDB caching strategies for cross-platform compatibility

### Adding New Services
When adding new external service integrations:
1. Create new module in `services/`
2. Export functions in `services/mod.rs`
3. Add query detection logic in `core/query.rs`
4. Update connection handling in `server/connection.rs`

### DN42 Backend Development
The DN42 system supports dual backends:
- Modify `dn42/manager.rs` for cross-platform logic
- Update `dn42/git_backend.rs` for Git-based features
- Update `dn42/online_backend.rs` for HTTP-based features
- Common query processing in `dn42/query.rs`

### LMDB Storage
All persistent data uses LMDB for performance:
- Storage interface in `storage/lmdb.rs`
- Shared storage instances via `create_shared_storage()`
- Thread-safe operations with proper error handling

### Module Dependencies
Key dependency relationships:
- `server/connection.rs` orchestrates all service modules
- `dn42/manager.rs` provides platform-aware DN42 access
- `core/query.rs` handles query routing to appropriate services
- `core/stats.rs` collects metrics from all operations