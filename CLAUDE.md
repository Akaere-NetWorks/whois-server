# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Running
- `cargo build --release` - Build the optimized binary
- `cargo run --release` - Run server with default settings (WHOIS port 43, Web dashboard port 9999)
- `cargo run --release -- --help` - Show all command-line options

### Common Development Tasks
- `cargo test` - Run tests
- `cargo clippy` - Lint the code
- `cargo fmt` - Format code
- `cargo doc --open` - Generate and open documentation

### Testing the Server
- Standard WHOIS client: `whois -h localhost -p 43 example.com`
- Using netcat: `echo "example.com" | nc localhost 43`
- Web dashboard: Access at http://localhost:9999
- API endpoint: `curl "http://localhost:9999/api/whois?q=example.com"`

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

3. **Server Layer** (`src/server/`)
   - Async TCP server using Tokio
   - Connection pooling and timeout management
   - Traffic dumping support for debugging

4. **Services Layer** (`src/services/`)
   - Modular implementations for each query type
   - External API integrations (IRR Explorer, Looking Glass, package repos, etc.)
   - Geo-location services with multiple providers

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

### Query Types

The server supports an extensive range of query types identified by suffixes:

**Standard WHOIS:** Domains, IPv4/IPv6 addresses, ASNs
**Enhanced Network:** `-GEO`, `-BGPTOOL`, `-IRR`, `-LG`, `-RPKI`, `-MANRS`
**Internet Tools:** `-DNS`, `-SSL`, `-CRT`, `-TRACE`
**Package Repositories:** `-CARGO`, `-NPM`, `-PYPI`, `-AUR`, `-DEBIAN`, etc.
**Entertainment:** `-MC`, `-STEAM`, `-IMDB`, `-PIXIV`, `-WIKIPEDIA`
**Development:** `-GITHUB`, plus built-in `HELP` system

### Configuration

- Environment variables via `.env` file (Pixiv integration, proxy settings)
- CLI arguments for ports, debugging, connection limits
- Constants in `src/config.rs` for WHOIS servers and paths

### External Dependencies

- **rdap** library for RDAP protocol support
- **Tokio** for async runtime
- **Axum** for web server
- **LMDB** for high-performance storage
- **Various API clients** for external service integration

The architecture emphasizes modularity, performance, and extensibility, with clean separation between the protocol handling, query routing, and service implementations.