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
```

## Architecture Overview

### Core Modules
- **main.rs**: Application entry point, initializes logging, stats, and starts servers
- **config.rs**: Command-line argument parsing using clap
- **query.rs**: Query type detection and analysis (11+ query types supported)
- **whois.rs**: WHOIS protocol client implementations for external servers
- **web.rs**: Axum-based web dashboard and REST API endpoints
- **stats.rs**: Real-time statistics collection and persistence

### Server Implementations
Two server architectures are available:
- **async_server.rs**: Tokio-based async server (default, high performance)
- **blocking_server.rs**: Blocking server (fallback for compatibility)

Both servers share:
- **connection.rs**: Connection handling logic and query processing
- **utils.rs**: Server utility functions

### Specialized Services
- **geo/**: Complete geo-location service module with RIPE and IPInfo integrations
- **bgptool.rs**: BGP tools integration for network analysis
- **irr.rs**: IRR Explorer integration for routing registry analysis
- **looking_glass.rs**: RIPE RIS Looking Glass services with BIRD-style output
- **email.rs**: Email search functionality
- **dn42.rs**: DN42 network integration with periodic data synchronization
- **rpki.rs**: RPKI validation services for prefix-ASN validation
- **manrs.rs**: MANRS (Mutually Agreed Norms for Routing Security) integration
- **lmdb_storage.rs**: LMDB-based storage for DN42 data caching

### Query Types Supported
1. Standard WHOIS: domains, IPv4/IPv6, ASNs, CIDR blocks
2. Enhanced queries: -EMAIL, -BGPTOOL, -PREFIXES suffixes
3. Geo-location: -GEO, -RIRGEO suffixes
4. Advanced routing: -IRR, -LG, -RADB suffixes
5. Security validation: -RPKI (prefix-ASN-RPKI format), -MANRS suffixes
6. DN42-specific queries (auto-detected)

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
- **DN42 Registry**: Comprehensive DN42 network support with local caching

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
- DN42 data: LMDB-based caching with periodic sync
- Traffic dumps: Optional raw query/response logging for debugging

## Key Dependencies
- **tokio**: Async runtime (with "full" features)
- **axum**: Web framework for dashboard
- **reqwest**: HTTP client with rustls-tls
- **clap**: Command-line parsing with derive features
- **tracing**: Structured logging
- **lmdb**: Lightning Memory-Mapped Database for DN42 caching
- **cidr**: CIDR block handling
- **regex**: Pattern matching for query analysis

## Performance Characteristics
- Dual server architecture (async/blocking)
- Configurable connection limits and timeouts
- Non-blocking external API calls
- Efficient connection pooling
- Real-time statistics with minimal overhead
- LMDB-based caching for DN42 data reduces external calls

## Development Notes
- Rust 2024 edition
- AGPL-3.0-or-later license
- No traditional test suite - integration testing via live queries
- Extensive logging with configurable verbosity levels
- Production deployment at whois.akae.re

## Web Dashboard Features
- Real-time statistics with auto-refresh
- Interactive WHOIS query testing interface
- Light/dark/auto theme support with pink-themed UI
- Responsive design for desktop and mobile
- Charts and visual analytics with 24h/30d views
- Query type distribution and performance metrics