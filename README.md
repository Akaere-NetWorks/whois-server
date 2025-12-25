<div align="center">

# 🌐 WHOIS Server

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.92.0+-orange.svg)](https://www.rust-lang.org/)
[![RFC 3912](https://img.shields.io/badge/RFC-3912-green.svg)](https://datatracker.ietf.org/doc/html/rfc3912)
[![DN42](https://img.shields.io/badge/DN42-Compatible-blueviolet)](https://dn42.eu/)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Akaere-NetWorks/whois-server)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FAkaere-NetWorks%2Fwhois-server.svg?type=shield&issueType=license)](https://app.fossa.com/projects/git%2Bgithub.com%2FAkaere-NetWorks%2Fwhois-server?ref=badge_shield&issueType=license)

**A high-performance WHOIS server with DN42 support, geo-location services, and comprehensive query capabilities.**

*Deployed at [whois.akae.re](https://whois.akae.re) - Try it now!*

</div>

## 📑 Table of Contents

- [🌐 WHOIS Server](#-whois-server)
  - [📑 Table of Contents](#-table-of-contents)
  - [✨ Features](#-features)
  - [🌍 Public Instance](#-public-instance)
  - [🖥️ Web Dashboard](#️-web-dashboard)
    - [API Endpoints](#api-endpoints)
  - [🚀 Installation](#-installation)
    - [As a Standalone Server](#as-a-standalone-server)
    - [As a Rust Library](#as-a-rust-library)
  - [🔧 Usage](#-usage)
    - [Running the server](#running-the-server)
    - [Configuration](#configuration)
      - [Environment Variables](#environment-variables)
    - [Command-line options](#command-line-options)
    - [Testing with WHOIS clients](#testing-with-whois-clients)
  - [🔍 Query Types and Features](#-query-types-and-features)
    - [Standard WHOIS Queries](#standard-whois-queries)
    - [Enhanced Query Types](#enhanced-query-types)
    - [Geo-location Services](#geo-location-services)
    - [Network Intelligence \& Advanced Features](#network-intelligence--advanced-features)
  - [🛠️ Advanced Query Features](#️-advanced-query-features)
    - [IRR Explorer Integration (`-IRR` suffix)](#irr-explorer-integration--irr-suffix)
    - [Looking Glass Services (`-LG` suffix)](#looking-glass-services--lg-suffix)
    - [RADB Direct Access (`-RADB` suffix)](#radb-direct-access--radb-suffix)
    - [ALTDB Direct Access (`-ALTDB` suffix)](#altdb-direct-access--altdb-suffix)
    - [IRR Direct Access (Multiple registries)](#irr-direct-access-multiple-registries)
    - [Modrinth Integration (`-MODRINTH` suffix)](#modrinth-integration--modrinth-suffix)
    - [CurseForge Integration (`-CURSEFORGE` suffix)](#curseforge-integration--curseforge-suffix)
  - [📊 Statistics and Monitoring](#-statistics-and-monitoring)
  - [🏗️ Architecture](#️-architecture)
    - [Key Components](#key-components)
  - [📜 License](#-license)

## ✨ Features

- **🚀 High Performance** - Asynchronous Rust implementation with configurable connection limits
- **🌐 RFC 3912 Compliant** - Full WHOIS protocol support
- **🔍 Smart Query Detection** - Automatic identification of domains, IP addresses, ASNs, and special query types
- **🌟 Platform-Aware DN42 Integration** - Cross-platform DN42 support with automatic backend selection
- **📍 Geo-location Services** - Built-in IP geolocation using multiple data sources
- **🔧 BGP Tools Integration** - Network analysis and BGP information queries
- **📧 Email Search** - Contact information lookup capabilities
- **🛡️ IRR Explorer Integration** - Comprehensive routing registry analysis with RPKI validation
- **🔭 Looking Glass Services** - RIPE RIS BGP data in BIRD routing daemon format
- **📊 RADB Direct Access** - Routing Assets Database queries for AS-SET and route objects
- **🔐 RPKI Validation** - Resource Public Key Infrastructure validation for prefix-ASN pairs
- **🛡️ MANRS Integration** - Mutually Agreed Norms for Routing Security compliance checking
- **🌐 DNS Resolution** - Comprehensive DNS record lookups using Cloudflare 1.1.1.1
- **🔍 Network Analysis** - Traceroute functionality for network path analysis
- **🔐 SSL/TLS Analysis** - Certificate analysis and validation using rustls
- **🔍 Certificate Transparency** - CT logs search via crt.sh API integration
- **🎮 Minecraft Integration** - Server status and user profile queries using Server List Ping protocol
- **🎮 Steam Integration** - Game information, user profiles, and game search with price display
- **🎬 IMDb Integration** - Movie and TV show information with ratings, cast, and search functionality
- **📦 Package Repository Support** - Comprehensive package queries for 10 major repositories (Cargo, NPM, PyPI, GitHub, AUR, Debian, Ubuntu, NixOS, OpenSUSE, AOSC, Modrinth)
- **🎮 Modrinth Integration** - Minecraft mods, resource packs, datapacks, and shaders information with downloads statistics
- **Pixiv Integration** - Artwork and user information with image URLs, search, and ranking queries via pixivpy3
- **🎭 Entertainment Services** - Wikipedia articles, anime/comic/game character database, and Luotianyi lyrics
- **🛠️ Development Tools** - GitHub user/repository information and built-in help system
- **Response Patch System** - Remote-managed response customization with automatic updates from GitHub
  - Context-aware text replacement with line-based rules
  - SHA1 checksum verification for integrity
  - LMDB storage for persistence and fast loading
  - Online updates via `UPDATE-PATCH` command
  - Detailed documentation in [patches/README.md](patches/README.md)
- **�📈 Real-time Statistics** - Comprehensive usage tracking and monitoring
- **🌐 Web Dashboard** - Modern web interface for statistics and testing
- **🔒 Robust Error Handling** - Graceful handling of network issues and timeouts
- **📋 Traffic Logging** - Optional query/response dumping for debugging
- **🌈 IPv4 & IPv6 Support** - Complete dual-stack implementation
- **🎯 Intelligent Fallback** - Automatic fallback to DN42 for failed public queries

## 🌍 Public Instance

A public instance of this WHOIS server is deployed at **whois.akae.re**. You can query it using:

```bash
# Domain queries
whois -h whois.akae.re example.com

# ASN queries  
whois -h whois.akae.re AS213605

# DN42-specific queries (automatically routed to DN42 WHOIS)
whois -h whois.akae.re AS4242420000
whois -h whois.akae.re example.dn42

# IP geolocation
whois -h whois.akae.re 8.8.8.8-GEO

# BGP information  
whois -h whois.akae.re AS213605-BGPTOOL

# Email search
whois -h whois.akae.re contact@example.com-EMAIL

# RADB queries
whois -h whois.akae.re 1.1.1.0-RADB

# IRR Explorer analysis
whois -h whois.akae.re 192.0.2.0/24-IRR

# Looking Glass (BIRD-style routing data)
whois -h whois.akae.re 1.1.1.0-LG

# RPKI validation
whois -h whois.akae.re 192.0.2.0/24-AS213605-RPKI

# MANRS compliance check
whois -h whois.akae.re AS213605-MANRS

# DNS resolution
whois -h whois.akae.re example.com-DNS

# Network traceroute
whois -h whois.akae.re 8.8.8.8-TRACEROUTE

# SSL certificate analysis
whois -h whois.akae.re example.com-SSL

# Certificate Transparency search
whois -h whois.akae.re example.com-CRT

# Minecraft server status
whois -h whois.akae.re play.hypixel.net-MC

# Steam game information
whois -h whois.akae.re 730-STEAM

# IMDb movie search  
whois -h whois.akae.re Inception-IMDB

# Package repository queries
whois -h whois.akae.re rust-CARGO
whois -h whois.akae.re express-NPM
whois -h whois.akae.re requests-PYPI

# Modrinth (Minecraft mods/resource packs)
whois -h whois.akae.re sodium-MODRINTH
whois -h whois.akae.re iris-MODRINTH

# Pixiv artwork and user information
whois -h whois.akae.re 123456789-PIXIV
whois -h whois.akae.re user:12345678-PIXIV
whois -h whois.akae.re search:风景-PIXIV
whois -h whois.akae.re ranking-PIXIV

# Entertainment services
whois -h whois.akae.re "Linux-WIKIPEDIA"
whois -h whois.akae.re "Miku-ACGC"
whois -h whois.akae.re "LYRIC"

# Development tools
whois -h whois.akae.re torvalds-GITHUB
whois -h whois.akae.re "HELP"
```

## 🖥️ Web Dashboard

The server includes a modern web dashboard accessible at `http://your-server:9999` (default port). The dashboard provides:

- **📊 Real-time Statistics** - Query counts, response times, and server metrics with auto-refresh
- **🧪 Query Testing** - Interactive WHOIS query interface for all supported query types
- **📈 Visual Analytics** - Charts and graphs with 24-hour and 30-day views
- **🎨 Theme Support** - Light/dark/auto theme with beautiful pink-themed UI
- **📱 Responsive Design** - Works perfectly on desktop and mobile devices
- **🔄 Live Updates** - Statistics refresh every 30 seconds automatically
- **📋 Query Type Distribution** - Visual breakdown of query types and usage patterns
- **⚡ Performance Metrics** - Connection counts, data transfer, and response times

### API Endpoints

The web server provides several API endpoints for integration:

- **`/api/whois?q=<query>`** - JSON-formatted WHOIS response with structured data
- **`/raw/<query>`** - Raw WHOIS output (text/plain) without JSON formatting
- **`/pixiv/<query>`** - Pixiv-specific JSON API for artwork, user, search, and ranking queries
- **`/api/stats`** - Server statistics in JSON format
- **`/api/openapi.json`** - OpenAPI 3.0 specification

Example usage:

```bash
# Get JSON-formatted response
curl "http://localhost:9999/api/whois?q=google.com"

# Get raw WHOIS output (plain text)
curl "http://localhost:9999/raw/google.com"
curl "http://localhost:9999/raw/AS13335"
curl "http://localhost:9999/raw/8.8.8.8"

# Get Pixiv data in pure JSON format
curl "http://localhost:9999/pixiv/123456789"
curl "http://localhost:9999/pixiv/user:12345678"
curl "http://localhost:9999/pixiv/search:风景"
curl "http://localhost:9999/pixiv/ranking"

# Get server statistics
curl "http://localhost:9999/api/stats"
```

## 🚀 Installation

### As a Standalone Server

Ensure you have Rust and Cargo installed, then:

```bash
# Clone the repository
git clone https://github.com/yourusername/whois-server.git
cd whois-server

# Build in release mode
cargo build --release

# The executable will be available at target/release/whois-server
```

### As a Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
whois-server = { git = "https://github.com/Akaere-NetWorks/whois-server.git" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

Then use it in your code:

```rust
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Query any resource just like the whois command
    let result = query("example.com").await?;
    println!("{}", result);
    
    // Works with all query types
    let result = query("AS13335").await?;
    let result = query("1.1.1.1-GEO").await?;
    let result = query("example.com-DNS").await?;
    
    Ok(())
}
```

**📚 For detailed library usage examples and API documentation, see [LIBRARY_USAGE.md](LIBRARY_USAGE.md)**

## 🔧 Usage

### Running the server

```bash
# With default settings (WHOIS on port 43, Web dashboard on port 9999)
cargo run --release

# With custom ports
cargo run --release -- --port 4343 --web-port 8080

# With debug output enabled
cargo run --release -- --debug

# With specific listen address
cargo run --release -- --host 127.0.0.1 

# Enable traffic dumping for debugging
cargo run --release -- --dump-traffic --dump-dir ./logs
```

### Configuration

#### Environment Variables

The server supports configuration via environment variables. Create a `.env` file in the project root:

```bash
# Pixiv Integration (Optional)
PIXIV_REFRESH_TOKEN=your_refresh_token_here    # Required for Pixiv queries
PIXIV_PROXY_ENABLED=false                       # Enable image proxy (true/false)
PIXIV_PROXY_BASE_URL=http://localhost:8080/pixiv-proxy  # Proxy base URL

# Other configurations...
```

**Pixiv Setup:**
1. Obtain a refresh token from Pixiv (see [pixivpy documentation](https://github.com/upbit/pixivpy))
2. Set `PIXIV_REFRESH_TOKEN` in your `.env` file
3. Optionally enable the image proxy to bypass referrer restrictions
4. The proxy allows images to be accessed without Pixiv's referrer checks

### Command-line options

```
Options:
  -H, --host <HOST>              Listen address [default: 0.0.0.0]
  -p, --port <PORT>              WHOIS server port [default: 43]
      --web-port <PORT>          Web dashboard port [default: 9999]
  -d, --debug                    Enable debug output
  -t, --trace                    Enable trace output (extremely verbose)  
      --max-connections <N>      Maximum concurrent connections [default: 100]
      --timeout <SECONDS>        Connection timeout in seconds [default: 10]
      --dump-traffic             Write raw queries and responses to files for debugging
      --dump-dir <DIR>           Dump traffic directory [default: dumps]
      --help                     Print help
      --version                  Print version
```

### Testing with WHOIS clients

```bash
# Linux/Mac
whois -h localhost -p 43 example.com

# Windows PowerShell  
(New-Object System.Net.Sockets.TcpClient("localhost", 43)).GetStream().Write([Text.Encoding]::ASCII.GetBytes("example.com`r`n"), 0, 13)

# Using netcat
echo "example.com" | nc localhost 43

# Using telnet
telnet localhost 43
# Then type your query and press Enter
```

## 🔍 Query Types and Features

### Standard WHOIS Queries

| Query Type | Example | Description |
|------------|---------|-------------|
| **Domains** | `example.com` | Standard domain WHOIS lookup |
| **IPv4 Addresses** | `8.8.8.8` | IPv4 address registration info |
| **IPv6 Addresses** | `2001:4860:4860::8888` | IPv6 address registration info |
| **ASN Numbers** | `AS213605` | Autonomous System information |
| **CIDR Blocks** | `192.168.1.0/24` | Network block information |

### Enhanced Query Types

| Query Suffix | Example | Description |
|--------------|---------|-------------|
| **-EMAIL** | `admin@example.com-EMAIL` | Search for contact information |
| **-BGPTOOL** | `AS213605-BGPTOOL` | BGP routing and peering info |
| **-PREFIXES** | `AS213605-PREFIXES` | List all prefixes announced by ASN |
| **-RADB** | `1.1.1.0-RADB` | Query RADB (Routing Assets Database) directly |
| **-ALTDB** | `1.1.1.0-ALTDB` | Query ALTDB (Alternative Database) routing registry |
| **-AFRINIC** | `AS37271-AFRINIC` | Query AFRINIC IRR (African region) |
| **-APNIC** | `AS4134-APNIC` | Query APNIC IRR (Asia-Pacific region) |
| **-ARIN** | `AS7018-ARIN` | Query ARIN IRR (North American region) |
| **-BELL** | `AS577-BELL` | Query BELL IRR (Bell Canada) |
| **-JPIRR** | `AS2497-JPIRR` | Query JPIRR (Japan Internet Routing Registry) |
| **-LACNIC** | `AS27715-LACNIC` | Query LACNIC IRR (Latin America region) |
| **-LEVEL3** | `AS3356-LEVEL3` | Query LEVEL3 IRR (Level3/CenturyLink) |
| **-NTTCOM** | `AS2914-NTTCOM` | Query NTTCOM IRR (NTT Communications) |
| **-RIPE** | `AS3333-RIPE` | Query RIPE IRR (European region) |
| **-TC** | `AS262589-TC` | Query TC IRR (Brazilian Telecom) |
| **-IRR** | `192.0.2.0/24-IRR` | IRR Explorer - comprehensive routing registry analysis |
| **-LG** | `1.1.1.0-LG` | Looking Glass - RIPE RIS BGP routing data in BIRD format |
| **-RPKI** | `192.0.2.0/24-AS213605-RPKI` | RPKI validation for prefix-ASN combinations |
| **-MANRS** | `AS213605-MANRS` | MANRS compliance and routing security status |
| **-DNS** | `example.com-DNS` | DNS resolution with multiple record types |
| **-TRACEROUTE** | `8.8.8.8-TRACEROUTE` | Network traceroute analysis (alias: -TRACE) |
| **-SSL** | `example.com-SSL` | SSL/TLS certificate analysis and validation |
| **-CRT** | `example.com-CRT` | Certificate Transparency logs search |
| **-MINECRAFT** | `mc.hypixel.net-MINECRAFT` | Minecraft server status (alias: -MC) |
| **-MCU** | `Notch-MCU` | Minecraft user profile information |
| **-STEAM** | `730-STEAM` | Steam game/user information |
| **-STEAMSEARCH** | `Counter-Strike-STEAMSEARCH` | Steam game search |
| **-IMDB** | `Inception-IMDB` | IMDb movie/TV show information |
| **-IMDBSEARCH** | `Batman-IMDBSEARCH` | IMDb title search |
| **-CARGO** | `rust-CARGO` | Rust crate information |
| **-NPM** | `express-NPM` | NPM package information |
| **-PYPI** | `requests-PYPI` | Python package information |
| **-MODRINTH** | `sodium-MODRINTH` | Modrinth mods/resource packs for Minecraft |
| **-AUR** | `firefox-AUR` | Arch User Repository packages |
| **-DEBIAN** | `nginx-DEBIAN` | Debian package information |
| **-UBUNTU** | `vim-UBUNTU` | Ubuntu package information |
| **-NIXOS** | `git-NIXOS` | NixOS package information |
| **-OPENSUSE** | `gcc-OPENSUSE` | OpenSUSE package information |
| **-AOSC** | `kernel-AOSC` | AOSC package information |
| **-GITHUB** | `torvalds-GITHUB` | GitHub user/repository information |
| **-WIKIPEDIA** | `Linux-WIKIPEDIA` | Wikipedia article information |
| **-ACGC** | `Miku-ACGC` | Anime/Comic/Game character info |
| **-LYRIC** | `LYRIC` | Random Luotianyi lyrics |
| **-PIXIV** | `123456789-PIXIV` | Pixiv artwork information |
| **-PIXIV** | `user:12345678-PIXIV` | Pixiv user profile |
| **-PIXIV** | `search:keyword-PIXIV` | Search Pixiv artworks |
| **-PIXIV** | `ranking-PIXIV` | Pixiv daily ranking (top 10) |
| **-PIXIV** | `illusts:12345678-PIXIV` | User's artwork list |
| **HELP** | `HELP` | Show all available query types |

### Geo-location Services

| Query Suffix | Example | Description |
|--------------|---------|-------------|
| **-GEO** | `8.8.8.8-GEO` | IP geolocation information |
| **-RIRGEO** | `203.0.113.1-RIRGEO` | RIR-specific geographic data |

### Network Intelligence & Advanced Features

The server provides intelligent query routing and advanced networking tools:

- **DN42 Detection** - Automatically routes DN42 queries (AS42424xxx, .dn42 domains, private IPs)
- **Private IP Handling** - RFC1918 and other private ranges routed to DN42
- **Smart Referrals** - Uses IANA for initial queries, then follows referrals
- **Multi-source Data** - Combines information from multiple WHOIS servers
- **IRR Explorer Integration** - Access to comprehensive Internet Routing Registry data with RPKI validation
- **Looking Glass Services** - Real-time BGP routing data from RIPE Route Information Service (RIS)
- **RADB Direct Access** - Query Routing Assets Database for AS-SET expansions and route objects
- **Intelligent Fallback** - Automatically tries DN42 when public WHOIS returns no results
- **BIRD-style Output** - Looking Glass queries formatted as BIRD routing daemon configuration

## 🛠️ Advanced Query Features

### IRR Explorer Integration (`-IRR` suffix)

The IRR Explorer integration provides comprehensive routing registry analysis using data from [irrexplorer.nlnog.net](https://irrexplorer.nlnog.net/). This feature analyzes prefixes across multiple Internet Routing Registries (IRRs) and provides RPKI validation information.

**Supported IRR Databases:**
- RIPE, RADB, ARIN, APNIC, AFRINIC, LACNIC
- LEVEL3, ALTDB, BELL, JPIRR, NTTCOM
- RPKI (Resource Public Key Infrastructure) data

**Example:**
```bash
whois -h whois.akae.re 192.0.2.0/24-IRR
```

### Looking Glass Services (`-LG` suffix)

Looking Glass queries provide real-time BGP routing data from RIPE's Route Information Service (RIS). The output is formatted in BIRD routing daemon style, making it useful for network operators and researchers.

**Features:**
- Real-time BGP routing tables from multiple RRC (Route Collector) locations
- BIRD-style configuration format output
- Community and extended community information
- AS-Path and origin validation data

**Example:**
```bash
whois -h whois.akae.re 1.1.1.0-LG
```

### RADB Direct Access (`-RADB` suffix)

Direct queries to the Routing Assets Database (RADB) for AS-SET expansions, route objects, and routing policies. This is particularly useful for network operators managing routing policies and filters.

**Example:**
```bash
whois -h whois.akae.re AS-SET:AS-EXAMPLE-RADB
```

### ALTDB Direct Access (`-ALTDB` suffix)

Direct queries to the ALTDB (Alternative Database) routing registry for route objects and routing information. ALTDB provides an alternative source for routing registry data.

**Example:**
```bash
whois -h whois.akae.re AS-EXAMPLE-ALTDB
whois -h whois.akae.re 192.0.2.0/24-ALTDB
```

### IRR Direct Access (Multiple registries)

The server supports direct queries to 12+ major Internet Routing Registries (IRRs) worldwide, providing comprehensive access to routing information across different regions and ISPs.

**Supported Registries:**

- **-AFRINIC**: African Network Information Centre IRR
- **-APNIC**: Asia-Pacific Network Information Centre IRR
- **-ARIN**: American Registry for Internet Numbers IRR
- **-BELL**: Bell Canada routing registry
- **-JPIRR**: Japan Internet Routing Registry
- **-LACNIC**: Latin America and Caribbean Network Information Centre IRR
- **-LEVEL3**: Level3/CenturyLink routing registry
- **-NTTCOM**: NTT Communications routing registry
- **-RIPE**: Réseaux IP Européens Network Coordination Centre IRR
- **-TC**: Brazilian Telecom routing registry

**Examples:**
```bash
# Query AFRINIC IRR for African networks
whois -h whois.akae.re AS37271-AFRINIC

# Query APNIC IRR for Asia-Pacific networks
whois -h whois.akae.re AS4134-APNIC

# Query ARIN IRR for North American networks
whois -h whois.akae.re AS7018-ARIN

# Query RIPE IRR for European networks
whois -h whois.akae.re AS3333-RIPE

# Query LACNIC IRR for Latin American networks
whois -h whois.akae.re AS27715-LACNIC

# Query ISP-specific registries
whois -h whois.akae.re AS3356-LEVEL3
whois -h whois.akae.re AS2914-NTTCOM
```

**Use Cases:**
- Multi-registry validation and cross-referencing
- Regional routing information analysis
- ISP-specific routing policy research
- Route origin validation across different registries
- Network planning and AS-SET management

**Use Cases:**
- Multi-registry comparison
- Alternative routing data sources
- Cross-reference verification

### Modrinth Integration (`-MODRINTH` suffix)

The Modrinth integration provides comprehensive information about Minecraft mods, resource packs, datapacks, and shaders from [modrinth.com](https://modrinth.com).

**Features:**
- Project information with download statistics and follower counts
- Compatibility details (client/server side, mod loaders, Minecraft versions)
- License information and project links
- Gallery images and donation links
- Available versions and update history
- Smart search when exact project slug is not provided

**Popular Queries:**
```bash
# Performance optimization mods
whois -h whois.akae.re sodium-MODRINTH      # Sodium rendering optimization
whois -h whois.akae.re lithium-MODRINTH     # Lithium server optimization

# Shaders
whois -h whois.akae.re iris-MODRINTH        # Iris Shaders

# Utility mods
whois -h whois.akae.re jei-MODRINTH         # Just Enough Items
whois -h whois.akae.re rei-MODRINTH         # Roughly Enough Items
```

### CurseForge Integration (`-CURSEFORGE` suffix)

The CurseForge integration provides access to the extensive CurseForge mod database, featuring millions of Minecraft mods and addons.

**Features:**
- Project information with download counts and popularity metrics
- Category classification and tags
- Latest file versions with dependencies
- Screenshots and project gallery
- Author information and links
- Support for both project ID and search queries

**API Key Setup:**
```bash
# Get your API key from https://console.curseforge.com/
export CURSEFORGE_API_KEY="your-api-key-here"
```

**Popular Queries:**
```bash
# Query by project ID
whois -h whois.akae.re 238222-CURSEFORGE    # Just Enough Items (JEI)
whois -h whois.akae.re 223794-CURSEFORGE    # Tinkers Construct

# Query by project name (search)
whois -h whois.akae.re jei-CURSEFORGE       # Search for JEI
whois -h whois.akae.re optifine-CURSEFORGE  # Search for OptiFine
whois -h whois.akae.re biomes-CURSEFORGE    # Search for Biomes O' Plenty
```

> 📘 **Detailed Documentation**: For comprehensive technical documentation of all advanced features, including API details, implementation specifics, and usage examples, see [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md).

## 📊 Statistics and Monitoring

The server maintains comprehensive statistics including:

- **Query Metrics** - Total queries, queries per minute, response times
- **Query Type Distribution** - Breakdown by domain, IP, ASN, etc.
- **Geographic Analytics** - Query origins and target distributions  
- **Error Tracking** - Failed queries, timeouts, and error rates
- **Performance Metrics** - Connection counts, processing times

Statistics are available through:
- **Web Dashboard** - Visual charts and real-time data at `/api/stats`
- **JSON API** - Programmatic access to all metrics
- **Automatic Persistence** - Stats saved on server shutdown

## 🏗️ Architecture

The server is built with a modular Rust architecture organized into logical components:

```
src/
├── main.rs          # Application entry point and initialization
├── lib.rs           # Library API entry point for external usage
├── config.rs        # Configuration constants (WHOIS servers, ports, etc.)
├── core/            # Core application logic
│   ├── query.rs     # Query type detection and routing (35+ query types)
│   ├── query_processor.rs # Query processing and execution logic
│   ├── color.rs     # Terminal colorization support
│   ├── stats.rs     # Real-time statistics collection and persistence  
│   └── utils.rs     # Shared utility functions
├── server/          # TCP server implementations
│   ├── async_server.rs     # Tokio-based async server
│   ├── connection.rs       # Connection handling and query processing
│   └── utils.rs            # Server utility functions
├── ssh/             # SSH server support
│   ├── server.rs    # SSH server implementation
│   ├── handler.rs   # SSH connection handling
│   ├── certificates.rs # SSH certificate management
│   └── history.rs   # Command history support
├── services/        # External service integrations
│   ├── whois.rs     # Standard WHOIS protocol clients
│   ├── email.rs     # Email search functionality
│   ├── bgptool.rs   # BGP tools integration
│   ├── irr.rs       # IRR Explorer integration
│   ├── looking_glass.rs # RIPE RIS Looking Glass services
│   ├── rpki.rs      # RPKI validation services
│   ├── manrs.rs     # MANRS integration
│   ├── dns.rs       # DNS resolution service
│   ├── traceroute.rs # Network traceroute functionality
│   ├── ssl.rs       # SSL/TLS certificate analysis
│   ├── crt.rs       # Certificate Transparency logs
│   ├── minecraft.rs # Minecraft server status and user profiles
│   ├── steam.rs     # Steam game and user information
│   ├── imdb.rs      # IMDb movie and TV show information
│   ├── acgc.rs      # Anime/Comic/Game character database
│   ├── wikipedia.rs # Wikipedia article information
│   ├── lyric.rs     # Luotianyi random lyrics
│   ├── github.rs    # GitHub user and repository information
│   ├── meal.rs      # Random meal suggestions
│   ├── desc.rs      # Description service
│   ├── help.rs      # Built-in help system
│   ├── iana_cache.rs # IANA registry data caching
│   ├── packages/    # Package repository integrations (14+ distros)
│   │   ├── cargo.rs    # Rust crate information
│   │   ├── npm.rs      # NPM package information
│   │   ├── pypi.rs     # Python package information
│   │   ├── aur.rs      # Arch User Repository
│   │   ├── debian.rs   # Debian packages
│   │   ├── ubuntu.rs   # Ubuntu packages
│   │   ├── nixos.rs    # NixOS packages
│   │   ├── opensuse.rs # OpenSUSE packages
│   │   ├── aosc.rs     # AOSC packages
│   │   ├── alma.rs     # AlmaLinux packages
│   │   ├── epel.rs     # EPEL packages
│   │   └── openwrt.rs  # OpenWrt packages
│   └── geo/         # Geo-location services
│       ├── services.rs     # Service orchestration
│       ├── types.rs        # Data type definitions
│       ├── formatters.rs   # Output formatting
│       ├── ripe_api.rs     # RIPE database integration
│       ├── ipinfo_api.rs   # IPInfo service integration
│       ├── constants.rs    # Geographic constants
│       └── utils.rs        # Geographic utility functions
├── dn42/            # DN42 network support (platform-aware)
│   ├── manager.rs   # Platform detection and backend orchestration
│   ├── git_backend.rs      # Git repository backend (Unix-like)
│   ├── online_backend.rs   # HTTP API backend (Windows)
│   └── query.rs     # DN42-specific query processing
├── storage/         # Data persistence layer
│   └── lmdb.rs      # LMDB storage for caching and persistence
└── web/             # Web dashboard and HTTP API
    ├── dashboard.rs # Axum-based web interface and REST endpoints
    ├── json_formatter.rs # JSON response formatting
    ├── dashboard_template.html # Dashboard HTML template
    └── docs_template.html # API documentation template
```

### Key Components

- **Query Engine** - Intelligent query parsing and type detection with 35+ query types
- **IRR Registry Support** - Direct access to 12+ Internet Routing Registries (RADB, ALTDB, AFRINIC, APNIC, ARIN, BELL, JPIRR, LACNIC, LEVEL3, NTTCOM, RIPE, TC)
- **Platform-Aware DN42** - Automatic Windows/Unix backend selection with LMDB caching
- **Async Server Architecture** - High-performance Tokio-based TCP and SSH server implementations
- **Modular Services** - Clean separation of 30+ external service integrations
- **Library API** - Exportable Rust library with `query()` and `query_with_color()` functions
- **Web Interface** - Axum-based REST API and dashboard with real-time updates
- **Statistics Engine** - Real-time metrics collection with 24h/30d historical data
- **Advanced Network Tools** - IRR Explorer, Looking Glass, BGP Tools, RPKI validation, MANRS
- **Cross-platform Storage** - LMDB-based caching for performance and persistence
- **Package Repositories** - Support for 14+ Linux distributions and package managers
- **Colorization Support** - Terminal color schemes for enhanced readability
- **Intelligent Routing** - Smart query routing with multi-source fallback mechanisms

## 📜 License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 
