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

# Check with clippy for additional linting
cargo clippy

# Format code according to Rust standards
cargo fmt

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
- `--enable-color`: Enable WHOIS-COLOR protocol support (default: true)

### Testing
The project does not include traditional unit tests. Testing is performed by running the server and querying it with WHOIS clients:

```bash
# Test with standard WHOIS client
whois -h localhost -p 43 example.com

# Test with netcat
echo "example.com" | nc localhost 43

# Test WHOIS-COLOR protocol capability detection
echo -e "X-WHOIS-COLOR-PROBE: 1\r\n\r\n" | nc localhost 43

# Test colored responses (RIPE style)
echo -e "X-WHOIS-COLOR: ripe\r\nexample.com\r\n" | nc localhost 43

# Test colored responses (BGPTools style)
echo -e "X-WHOIS-COLOR: bgptools\r\n8.8.8.8\r\n" | nc localhost 43

# Test web dashboard
curl http://localhost:9999/api/stats

# Test against the public deployment instance
whois -h whois.akae.re example.com
whois -h whois.akae.re AS213605-MANRS
whois -h whois.akae.re 8.8.8.8-GEO

# Test Steam services
whois -h whois.akae.re 730-STEAM  # Counter-Strike 2 game info
whois -h whois.akae.re Counter-Strike-STEAMSEARCH  # Game search
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
- **`lib.rs`**: Library interface for the whois-server crate

### Core Components

**Core Module (`core/`)**:
- `query.rs`: Query type detection and routing (15+ query types)
- `stats.rs`: Real-time statistics collection and persistence
- `utils.rs`: Shared utility functions
- `color.rs`: WHOIS-COLOR protocol v1.0 implementation with RIPE and BGPTools schemes

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
- `ssl.rs`: SSL/TLS certificate analysis service for domain certificate information
- `crt.rs`: Certificate Transparency logs service via crt.sh API integration
- `minecraft.rs`: Minecraft server status queries using Server List Ping protocol
- `steam.rs`: Steam API integration for game information and user profiles with price display
- `aur.rs`: Arch User Repository package information queries
- `debian.rs`: Debian package database queries and information

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
6. Network diagnostics: -DNS, -TRACEROUTE/-TRACE suffixes
7. SSL/TLS certificates: -SSL suffix for domain certificate analysis
8. Certificate Transparency: -CRT suffix for CT log searches
9. Minecraft servers: -MINECRAFT or -MC suffix for server status
10. Steam integration: -STEAM (game/user info), -STEAMSEARCH (game search)
11. Package queries: -AUR (Arch User Repository), -DEBIAN (Debian packages)
12. DN42-specific queries (auto-detected)
13. IANA registry caching for efficient resource lookups

### WHOIS-COLOR Protocol Support
The server implements WHOIS-COLOR protocol v1.0 for enhanced terminal output:

**Protocol Features**:
- **Capability Detection**: Automatic detection via `X-WHOIS-COLOR-PROBE` header
- **Backward Compatibility**: Standard WHOIS operation when color is not supported
- **Multiple Schemes**: RIPE-style and BGPTools-style colorization
- **Query-Type Aware**: Different coloring patterns based on query type

**Supported Color Schemes**:
- **RIPE Style**: Traditional WHOIS database coloring with attribute-based colors
  - Network resources (routes, prefixes) in green
  - ASN information in yellow/gold
  - Contact information in blue
  - Geographic data in magenta
  - Status information in bright red
  - Timestamps in gray
- **BGPTools Style**: Network analysis focused coloring
  - ASN numbers highlighted with backgrounds
  - IP addresses and prefixes in bright colors
  - Status validation with colored backgrounds (green/red/yellow)
  - Query-specific highlighting for specialized responses

**Usage Examples**:
```bash
# Check server color support
echo -e "X-WHOIS-COLOR-PROBE: 1\r\n\r\n" | nc whois.akae.re 43

# Query with RIPE-style coloring
echo -e "X-WHOIS-COLOR: ripe\r\nexample.com\r\n" | nc whois.akae.re 43

# Query with BGPTools-style coloring  
echo -e "X-WHOIS-COLOR: bgptools\r\nAS15169\r\n" | nc whois.akae.re 43
```

**Implementation Details**:
- Only activates when client explicitly requests color support
- Graceful fallback to standard WHOIS on any protocol issues
- Query-type specific colorization for all supported query types (-EMAIL, -GEO, -BGP, etc.)
- ANSI escape code based with terminal compatibility
- Configurable via `--enable-color` flag (default: enabled)

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
- **SSL/TLS Certificates**: rustls-based certificate analysis with comprehensive certificate chain parsing
- **Certificate Transparency**: crt.sh API integration for CT log searches with robust error handling
- **Minecraft Servers**: Server List Ping protocol implementation for server status queries
- **Steam API**: Game information, user profiles, and game search with price display and discount detection
- **Package Repositories**: AUR (Arch User Repository) and Debian package database integration

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
- Environment variables and `.env` file support (for API keys)
- Runtime configuration via web dashboard

**Environment Variables**:
- `STEAM_API_KEY`: Required for Steam user profile queries (obtain from https://steamcommunity.com/dev/apikey)

**Example .env file**:
```
STEAM_API_KEY=your_steam_api_key_here
```

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
- **serde**: JSON serialization/deserialization for external APIs
- **dotenv**: Environment variable loading from .env files
- **rustls**: TLS implementation for SSL certificate analysis
- **chrono**: Date and time handling for timestamps

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
- All source files include AGPL-3.0-or-later copyright headers

## Web Dashboard Features
- Real-time statistics with auto-refresh
- Interactive WHOIS query testing interface
- Light/dark/auto theme support with pink-themed UI
- Responsive design for desktop and mobile
- Charts and visual analytics with 24h/30d views
- Query type distribution and performance metrics

## Important Implementation Details

### Recent Updates
- **Steam API Integration**: Added comprehensive Steam game and user information queries with price display, discount detection, and game search functionality
- **Enhanced Color Support**: Implemented conditional price coloring in WHOIS-COLOR protocol (green for discounts, white for full price)
- **Package Repository Support**: Added AUR and Debian package database integration
- **DNS Service Optimization**: The DNS service has been optimized to use Cloudflare's 1.1.1.1 as a fixed DNS server instead of multiple root servers for improved reliability and performance
- **Enhanced Network Services**: Added traceroute and IANA cache services for comprehensive network analysis
- **Platform-Aware Caching**: Improved LMDB caching strategies for cross-platform compatibility

### Steam Service Implementation
The Steam service provides comprehensive game and user information:

**Query Types**:
- `-STEAM`: Game information by App ID or user profiles by Steam ID
- `-STEAMSEARCH`: Fuzzy game search with top 10 results

**Key Features**:
- **Dual API Approach**: Steam Store API (primary) with App List API fallback for robust search
- **Price Intelligence**: Automatic discount detection with "price (10%↓)" formatting
- **Conditional Coloring**: Green for discounted/free games, white for full-price games
- **Environment Configuration**: Supports `.env` file for `STEAM_API_KEY` (required for user profiles)
- **App ID Detection**: Automatic differentiation between App IDs (<10M) and Steam IDs (17 digits)

**Implementation Notes**:
- Steam Store API provides detailed price/discount information for search results
- App List API serves as fallback with basic information only
- Price extraction handles various Steam price formats and currencies (defaults to USD)
- WHOIS-format responses with proper attribute-value pair formatting

### Adding New Services
When adding new external service integrations:
1. Create new module in `services/`
2. Export functions in `services/mod.rs`
3. Add query detection logic in `core/query.rs` (order matters for overlapping suffixes)
4. Update connection handling in `server/connection.rs`
5. Add color support in `core/color.rs` for both RIPE and BGPTools schemes

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