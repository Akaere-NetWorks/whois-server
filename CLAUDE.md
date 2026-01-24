# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Running
- `cargo build --release` - Build the optimized binary
- `cargo run --release` - Run server with default settings (WHOIS port 43, Web dashboard port 9999)
- `cargo run --release -- --help` - Show all command-line options

### Code Quality
- `cargo clippy` - Lint the code (required for CI/CD)
- `cargo fmt` - Format code (required for CI/CD)
- `cargo doc --open` - Generate and open documentation

### Testing
- `cargo test` - Run tests (minimal coverage - only color scheme tests)
- Note: The project has minimal test coverage. Manual testing via WHOIS client is recommended

### Docker Development
- `docker build -t whois-server .` - Build Docker image
- `docker run -p 43:43 -p 9999:9999 whois-server` - Run with ports exposed
- Images are automatically built and published to GitHub Container Registry for main branch pushes

### CI/CD Workflows
- **Build Workflow**: Runs on Ubuntu 22.04 and 24.04 with Rust toolchain setup
  - Includes clippy and rustfmt checks
  - Caches Cargo dependencies for faster builds
  - Creates artifacts for each Ubuntu version
- **Docker Workflow**: Multi-architecture Docker builds with automatic publishing
  - Builds and publishes to GitHub Container Registry (ghcr.io)
  - Triggers on pushes to main branch and version tags

### Development Dependencies
- **Rust**: 1.92.0+ (stable toolchain, uses 2024 edition)
- **Lua**: 5.4+ development headers (liblua5.4-dev, pkg-config) for plugin support
- **Git**: Required for DN42 registry synchronization
- **Standard Cargo toolchain**: build, test, clippy, fmt

### Testing the Server
- Standard WHOIS client: `whois -h localhost -p 43 example.com`
- Using netcat: `echo "example.com" | nc localhost 43`
- Web dashboard: Access at http://localhost:9999
- API endpoint: `curl "http://localhost:9999/api/whois?q=example.com"`

### Logging System

The project uses a custom systemd-style logger (not the standard `log` crate):
- Logger implementation in `src/core/logger.rs`
- Macro-based logging: `log_info!()`, `log_error!()`, `log_debug!()`, `log_warn!()`, `log_init_*!()`
- Initialize with `init_from_args(debug, trace, quiet)` from CLI args
- Specialized macros for initialization and task logging with structured output

### Command-Line Options
```
-H, --host <HOST>              Listen address [default: 0.0.0.0]
-p, --port <PORT>              WHOIS server port [default: 43]
    --web-port <PORT>          Web dashboard port [default: 9999]
    --ssh-port <PORT>          SSH server port [default: 2222]
-d, --debug                    Enable debug output
-t, --trace                    Enable trace output (extremely verbose)
    --max-connections <N>      Maximum concurrent connections [default: 100]
    --timeout <SECONDS>        Connection timeout in seconds [default: 10]
    --dump-traffic             Write raw queries and responses to files for debugging
    --dump-dir <DIR>           Dump traffic directory [default: dumps]
    --enable-color             Enable colored terminal output
    --enable-ssh               Enable SSH server
    --ssh-cache-dir <DIR>      SSH cache directory [default: ./ssh-cache]
```

### Patch Management
The server uses a response patch system for remote customization:
- Patches are stored in LMDB at `./cache/patches-lmdb`
- Update patches: `echo "UPDATE-PATCH" | nc localhost 43`
- Verify patches: `./patches/verify-patches.sh`
- Update JSON data: `./patches/update-patches-json.sh`

## High-Level Architecture

This is a comprehensive WHOIS server built in Rust with extensive query capabilities beyond standard WHOIS lookups.

### Core Architecture

**Main Entry Points:**
- `src/main.rs` - Server application entry point with CLI argument parsing
- `src/lib.rs` - Library interface exposing `query()` and `query_with_color()` functions

**Key Components:**

1. **Query Engine** (`src/core/query.rs`)
   - Detects query types from 35+ supported formats (domains, IPs, ASNs, special suffixed queries)
   - Routes queries to appropriate handlers
   - Supports intelligent DN42 detection and fallback

2. **Query Processing** (`src/core/query_processor.rs`)
   - Central processing pipeline for all query types
   - Handles statistics collection and response formatting
   - Manages color scheme support for terminal output
   - Processes response patches for customization
   - Integrates with Lua plugin system for extensibility

3. **Server Layer** (`src/server/`)
   - Async TCP server using Tokio
   - Connection pooling and timeout management
   - Traffic dumping support for debugging

4. **Services Layer** (`src/services/`)
   - Modular implementations for each query type
   - External API integrations (IRR Explorer, Looking Glass, package repos, etc.)
   - Geo-location services with multiple providers
   - Pure Rust Pixiv client implementation (no Python dependency)

### Specialized Systems

**DN42 Integration** (`src/dn42/`)
- Platform-aware backend selection (Git for Unix-like, HTTP API for Windows)
- LMDB caching for performance
- Automatic maintenance and synchronization

**Web Dashboard** (`src/web/`)
- Axum-based REST API and web interface
- Real-time statistics with JSON API endpoints
- Responsive UI with theme support

**SSH Server** (`src/ssh/`)
- Alternative access method with command history
- Certificate-based authentication support

**Storage Layer** (`src/storage/`)
- LMDB-based caching for DN42 registry, statistics, and patches
- Persistent data management with TTL support

**Plugin System** (`src/plugins/`)
- Lua-based plugin architecture for extensibility
- Secure sandboxed execution environment
- Plugin registry and loader for dynamic extension loading

### Query Types

The server supports 50+ query types identified by suffixes. Query detection is in `src/core/query.rs`:

**Standard WHOIS:** Domains, IPv4/IPv6 addresses, ASNs, CIDR blocks
**Enhanced Network:** `-GEO`, `-BGPTOOL`, `-IRR`, `-LG`, `-RPKI`, `-MANRS`, `-PEERINGDB`, `-RDAP`
**IRR Direct Access:** `-RADB`, `-ALTDB`, `-AFRINIC`, `-APNIC`, `-ARIN`, `-BELL`, `-JPIRR`, `-LACNIC`, `-LEVEL3`, `-NTTCOM`, `-RIPE`, `-TC`
**Internet Tools:** `-DNS`, `-SSL`, `-CRT`, `-TRACE`, `-PING`, `-NTP`
**Package Repositories:** `-CARGO`, `-NPM`, `-PYPI`, `-AUR`, `-DEBIAN`, `-UBUNTU`, `-NIXOS`, `-OPENSUSE`, `-OPENWRT`, `-ALMA`, `-EPEL`, `-AOSC`, `-MODRINTH`, `-CURSEFORGE`
**Entertainment:** `-MC`, `-MCU`, `-STEAM`, `-STEAMSEARCH`, `-IMDB`, `-IMDBSEARCH`, `-PIXIV`, `-WIKIPEDIA`, `-ACGC`
**Development:** `-GITHUB`, `-ICP`, `-PEN`
**Utility:** `HELP`, `UPDATE-PATCH`, `-EMAIL`, `-DESC`, `-MEAL`, `-MEAL-CN`, `-LYRIC`, `-CFSTATUS`, `-RIRGEO`, `-PREFIXES`

### Configuration

**Environment Variables (.env file):**
- `PIXIV_REFRESH_TOKEN` - Pixiv API refresh token for artwork queries
- `PIXIV_PROXY_ENABLED` - Enable/disable Pixiv image proxy (true/false)
- `PIXIV_PROXY_BASE_URL` - Proxy base URL for bypassing referrer checks

**CLI Configuration:**
- Ports, host, debugging flags via command-line arguments (see above)
- Connection limits and timeouts configurable
- Traffic dumping for debugging purposes

**Code Configuration:**
- WHOIS server endpoints in `src/config.rs`
- IRR database hosts and ports in `src/config.rs`
- Service endpoints embedded in individual service modules
- Plugin system: Lua scripts in `plugins/` directory for custom query handlers

### External Dependencies

**Key Libraries:**
- **rdap** (git: https://github.com/Akaere-NetWorks/rdap.git) - RDAP protocol client library
- **Tokio** (1.48.0) - Async runtime with full features
- **Axum** (0.7) - Web framework for dashboard and API
- **LMDB** (0.8.0) - High-performance embedded database
- **mlua** (0.11) - Lua 5.4 integration with async support
- **russh** (0.45) - SSH server implementation
- **regex** (1.12.2) - Query pattern matching
- **reqwest** (0.11) - HTTP client with rustls TLS
- **serde/serde_json** - Serialization for APIs and storage
- **clap** (4.5) - CLI argument parsing
- **chrono** - Time handling for statistics
- **cidr** - IP address and CIDR block manipulation
- **tokio-cron-scheduler** - Periodic task scheduling (DN42 sync, PEN updates)

**Service Integrations:**
The server integrates with numerous external APIs for query processing. See individual service modules in `src/services/` for specific endpoints used.

### Architecture Patterns

**Query Flow:**
1. Query received → `src/core/query.rs` detects type (35+ patterns)
2. Routed to `src/core/query_processor.rs` for processing
3. Handler selected from `src/services/` based on query type
4. Response formatted with optional colorization
5. Statistics collected in `src/storage/lmdb.rs`
6. Patches applied (if configured) before response

**Extension Points:**
- Add new query types:
  1. Add variant to `QueryType` enum in `src/core/query.rs`
  2. Add detection pattern in `analyze_query()` function
  3. Create handler function in `src/services/` (or `src/services/packages/` for package repos)
  4. Add match arm in `process_query()` in `src/core/query_processor.rs`
  5. Export in `src/services/mod.rs`
- Add package repositories: Create module in `src/services/packages/` following the pattern of existing modules
- Create plugins: Add Lua scripts to `plugins/` directory (see `src/plugins/` for plugin API)
- Customize responses: Add patch rules in `patches/` directory with JSON metadata

### Important Implementation Details

**DN42 Detection Logic:**
- `.dn42` domains → DN42 backend
- Private IPv4 (RFC1918, etc.) → DN42 backend
- Private IPv6 (fc00::/7, etc.) → DN42 backend
- AS numbers starting with `AS42424` → DN42 backend
- Uses platform-aware backends: Git for Unix-like systems, HTTP API for Windows

**Query Routing:**
1. Special queries (HELP, UPDATE-PATCH, meal suggestions) handled immediately
2. Plugin queries handled by Lua plugin system if suffix matches registered plugin
3. Suffixed queries routed to specialized handlers
4. Standard queries (domain/IP/ASN) use IANA referral or DN42 based on detection

**Color System:**
- Supports Dark, Light, and Auto color schemes in `src/core/color/`
- Colorization applied after query processing, before patch application
- Protocol-aware colorization for structured data

**Statistics Collection:**
- Real-time tracking via `Arc<Stats>` in `src/storage/lmdb.rs`
- Metrics: query counts, type distribution, response times, geographic data
- Saved on shutdown and loaded on startup
- Exposed via web API at `/api/stats`