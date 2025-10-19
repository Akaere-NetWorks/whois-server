# 🛠️ Advanced Features Documentation

This document provides detailed technical information about the advanced features implemented in the WHOIS server.

## 📋 Table of Contents

- [Platform-Aware DN42 Integration](#platform-aware-dn42-integration)
- [IRR Explorer Integration](#irr-explorer-integration)
- [Looking Glass Services](#looking-glass-services)
- [RADB Direct Access](#radb-direct-access)
- [ALTDB Direct Access](#altdb-direct-access)
- [IRR (Internet Routing Registry) Direct Access](#irr-internet-routing-registry-direct-access)
- [RPKI Validation](#rpki-validation)
- [MANRS Integration](#manrs-integration)
- [DNS Resolution Service](#dns-resolution-service)
- [Traceroute Network Analysis](#traceroute-network-analysis)
- [SSL/TLS Certificate Analysis](#ssltls-certificate-analysis)
- [Certificate Transparency Search](#certificate-transparency-search)
- [Minecraft Server Status](#minecraft-server-status)
- [Steam Integration](#steam-integration)
- [IMDb Integration](#imdb-integration)
- [Package Repository Queries](#package-repository-queries)
- [Entertainment Services](#entertainment-services)
- [Development Tools](#development-tools)
- [Intelligent Query Routing](#intelligent-query-routing)
- [Web Dashboard API](#web-dashboard-api)
- [Statistics and Monitoring](#statistics-and-monitoring)

## 🌐 Platform-Aware DN42 Integration

### Overview
The DN42 module implements automatic platform detection and uses the most appropriate backend for the operating system. This ensures optimal performance and compatibility across different environments.

### Platform Detection
The system automatically detects the operating system and selects the appropriate backend:

**Windows Systems**:
- Uses online file access via `https://git.pysio.online/pysio/mirrors-dn42/-/raw/master/data`
- Direct HTTP fetching of DN42 registry files
- LMDB-based caching with 1-day expiration
- No Git dependency required

**Unix-like Systems**:
- Uses Git repository cloning and synchronization
- Local Git repository with periodic updates
- LMDB caching for fast access
- Full offline operation capability

### Architecture
```
dn42/
├── manager.rs         # Platform detection and backend orchestration
├── git_backend.rs     # Git repository backend (Unix-like)
├── online_backend.rs  # HTTP API backend (Windows)
└── query.rs          # DN42-specific query processing
```

### Implementation Details
- `DN42Manager` handles platform detection automatically
- Unified interface regardless of underlying implementation
- LMDB storage used for caching in both modes
- Seamless fallback and error handling
- Configurable cache expiration and cleanup

### Supported Query Types
- ASN queries (AS4242420000-AS4242423999)
- .dn42 domain queries
- Private IP address ranges (RFC1918, etc.)
- DN42-specific object types (person, maintainer, etc.)

## 🛡️ IRR Explorer Integration

### Overview
The IRR Explorer integration provides comprehensive routing registry analysis using the [IRR Explorer API](https://irrexplorer.nlnog.net/). This feature analyzes prefixes across multiple Internet Routing Registries and provides RPKI validation.

### Supported Query Format
```
<prefix>-IRR
```

### Examples
```bash
# Analyze a specific prefix
whois -h whois.akae.re 192.0.2.0/24-IRR

# Analyze a larger prefix
whois -h whois.akae.re 10.0.0.0/8-IRR
```

### Response Format
The response includes:
- **Overall Category**: Assessment of routing registry consistency
- **Goodness Score**: Numerical quality rating
- **BGP Origins**: ASNs announcing the prefix
- **RPKI Status**: RPKI validation information
- **IRR Database Coverage**: Which IRRs contain route objects
- **Messages**: Warnings and recommendations

### Supported IRR Databases
- **RIPE**: European registry
- **RADB**: Routing Assets Database
- **ARIN**: American registry
- **APNIC**: Asia-Pacific registry
- **AFRINIC**: African registry
- **LACNIC**: Latin American registry
- **LEVEL3**: Level 3 Communications registry
- **ALTDB**: Alternative registry
- **BELL**: Bell Canada registry
- **JPIRR**: Japan Internet Routing Registry
- **NTTCOM**: NTT Communications registry
- **RPKI**: Resource Public Key Infrastructure data

## 🔭 Looking Glass Services

### Overview
Looking Glass queries provide real-time BGP routing data from RIPE's Route Information Service (RIS). The output is formatted in BIRD routing daemon style.

### Supported Query Format
```
<resource>-LG
```

### Examples
```bash
# Get BGP data for an ASN
whois -h whois.akae.re 1.1.1.0-LG

# Get BGP data for a prefix
whois -h whois.akae.re 192.0.2.0/24-LG
```

### Response Format (BIRD-style)
```bird
% RIPE STAT Looking Glass data for AS213605
% Data from RIPE NCC Route Information Service (RIS)
% Output in BIRD routing daemon style

# Routes for prefix 203.0.113.0/24
route 203.0.113.0/24 via 192.0.2.1 {
    # Peer: 192.0.2.1 (AS64496)
    # AS-Path: 64496 213605
    # Origin: IGP
    # Communities: 64496:1000
    # Last Updated: 2025-01-15T10:30:00
    bgp_path.len = 2;
    bgp_origin = IGP;
    bgp_next_hop = 192.0.2.1;
    bgp_community.add((64496,1000));
}
```

### Features
- **RRC Locations**: Data from multiple Route Collector locations worldwide
- **BGP Attributes**: Complete AS-path, communities, and next-hop information
- **BIRD Compatibility**: Output can be used as BIRD configuration reference
- **Real-time Data**: Fresh BGP routing table information

## 📊 RADB Direct Access

### Overview
Direct queries to the Routing Assets Database for AS-SET expansions, route objects, and routing policies.

### Supported Query Format
```
<resource>-RADB
```

### Examples
```bash
# Query an AS-SET
whois -h whois.akae.re AS-SET:AS-EXAMPLE-RADB

# Query route objects
whois -h whois.akae.re 1.1.1.0-RADB

# Query maintainer objects
whois -h whois.akae.re MAINT-EXAMPLE-RADB
```

### Use Cases
- **AS-SET Expansion**: Get all ASNs in an AS-SET
- **Route Objects**: Find registered route objects for an ASN
- **Policy Information**: Routing policies and contact information

## � ALTDB Direct Access

### Overview
Direct queries to the ALTDB (Alternative Database) routing registry for route objects and routing information.

### Supported Query Format
```
<resource>-ALTDB
```

### Examples
```bash
# Query an AS-SET
whois -h whois.akae.re AS-EXAMPLE-ALTDB

# Query route objects
whois -h whois.akae.re 192.0.2.0/24-ALTDB

# Query maintainer objects
whois -h whois.akae.re MAINT-EXAMPLE-ALTDB
```

### Features
- **Alternative Registry**: Access to ALTDB routing registry data
- **Route Objects**: Query route object information
- **AS Information**: Autonomous System details from ALTDB
- **Maintainer Data**: Contact and administrative information

### Use Cases
- **Multi-Registry Queries**: Compare data across RADB and ALTDB
- **Alternative Sources**: Access routing information from ALTDB
- **Registry Verification**: Cross-reference routing data

## 🌍 IRR (Internet Routing Registry) Direct Access

### Overview
Direct access to multiple Internet Routing Registry databases for comprehensive routing information queries.

### Supported Registries

#### AFRINIC IRR
African Network Information Centre routing registry.

**Format**: `<resource>-AFRINIC`

**Examples**:
```bash
whois -h whois.akae.re AS37271-AFRINIC
whois -h whois.akae.re 197.155.0.0/16-AFRINIC
```

#### APNIC IRR
Asia-Pacific Network Information Centre routing registry.

**Format**: `<resource>-APNIC`

**Examples**:
```bash
whois -h whois.akae.re AS4134-APNIC
whois -h whois.akae.re 202.12.28.0/24-APNIC
```

#### ARIN IRR
American Registry for Internet Numbers routing registry.

**Format**: `<resource>-ARIN`

**Examples**:
```bash
whois -h whois.akae.re AS7018-ARIN
whois -h whois.akae.re 8.8.8.0/24-ARIN
```

#### BELL IRR
Bell Canada routing registry.

**Format**: `<resource>-BELL`

**Examples**:
```bash
whois -h whois.akae.re AS577-BELL
whois -h whois.akae.re AS-BELL-BELL
```

#### JPIRR
Japan Internet Routing Registry.

**Format**: `<resource>-JPIRR`

**Examples**:
```bash
whois -h whois.akae.re AS2497-JPIRR
whois -h whois.akae.re AS-JPNIC-JPIRR
```

#### LACNIC IRR
Latin America and Caribbean Network Information Centre routing registry.

**Format**: `<resource>-LACNIC`

**Examples**:
```bash
whois -h whois.akae.re AS27715-LACNIC
whois -h whois.akae.re 200.0.0.0/8-LACNIC
```

#### LEVEL3 IRR
Level3/CenturyLink routing registry.

**Format**: `<resource>-LEVEL3`

**Examples**:
```bash
whois -h whois.akae.re AS3356-LEVEL3
whois -h whois.akae.re AS-LEVEL3-LEVEL3
```

#### NTTCOM IRR
NTT Communications routing registry.

**Format**: `<resource>-NTTCOM`

**Examples**:
```bash
whois -h whois.akae.re AS2914-NTTCOM
whois -h whois.akae.re AS-NTTCOM-NTTCOM
```

#### RIPE IRR
Réseaux IP Européens Network Coordination Centre routing registry.

**Format**: `<resource>-RIPE`

**Examples**:
```bash
whois -h whois.akae.re AS3333-RIPE
whois -h whois.akae.re 193.0.0.0/8-RIPE
```

#### TC (Telecom) IRR
Brazilian Telecom routing registry.

**Format**: `<resource>-TC`

**Examples**:
```bash
whois -h whois.akae.re AS262589-TC
whois -h whois.akae.re 200.160.0.0/20-TC
```

### Features
- **Multi-Registry Support**: Query 12+ major Internet Routing Registries
- **Global Coverage**: Access regional registries (AFRINIC, APNIC, ARIN, LACNIC, RIPE)
- **ISP-Specific Registries**: Query Bell, Level3, NTTCOM, and TC registries
- **AS-SET Queries**: Retrieve Autonomous System sets and routing policies
- **Route Objects**: Query route object information from specific registries
- **Maintainer Information**: Access contact and administrative data

### Use Cases
- **Multi-Registry Validation**: Cross-reference routing data across multiple IRRs
- **Regional Analysis**: Query region-specific routing information
- **ISP Policy Research**: Access ISP-maintained routing registries
- **Route Origin Validation**: Verify route origins in different registries
- **Network Planning**: Research AS-SETs and routing policies

## � RPKI Validation

### Overview
RPKI (Resource Public Key Infrastructure) validation provides cryptographic verification of IP address and ASN bindings to prevent route hijacking and improve routing security.

### Supported Query Format
```
<prefix>-<asn>-RPKI
```

### Examples
```bash
# Validate prefix-ASN binding
whois -h whois.akae.re 192.0.2.0/24-AS213605-RPKI

# Check RPKI status for a larger prefix
whois -h whois.akae.re 203.0.113.0/24-AS64496-RPKI
```

### Response Information
- **RPKI Status**: Valid, Invalid, or Not Found
- **ROA Details**: Route Origin Authorization information
- **Validity Period**: Certificate validity dates
- **Trust Anchor**: Issuing authority information
- **Cryptographic Validation**: Digital signature verification

### Use Cases
- **Route Validation**: Verify legitimate route announcements
- **Security Monitoring**: Detect potential route hijacks
- **Policy Enforcement**: Implement RPKI-based filtering
- **Compliance Checking**: Ensure RPKI best practices

## 🛡️ MANRS Integration

### Overview
MANRS (Mutually Agreed Norms for Routing Security) integration provides compliance checking and routing security assessment for network operators.

### Supported Query Format
```
<asn>-MANRS
```

### Examples
```bash
# Check MANRS compliance for an ASN
whois -h whois.akae.re AS213605-MANRS

# Verify routing security practices
whois -h whois.akae.re AS64496-MANRS
```

### Response Information
- **MANRS Participation**: Whether the ASN participates in MANRS
- **Implementation Status**: Which MANRS actions are implemented
- **Action Compliance**: Detailed breakdown of MANRS actions 1-4
- **Contact Information**: Network operator contact details
- **Certification Status**: MANRS observatory data

### MANRS Actions Checked
1. **Filtering**: Prevent propagation of incorrect routing information
2. **Anti-spoofing**: Prevent traffic with spoofed source IP addresses
3. **Coordination**: Facilitate global operational communication and coordination
4. **Global Validation**: Facilitate validation of routing information on a global scale

### Use Cases
- **Security Assessment**: Evaluate network security practices
- **Partner Evaluation**: Assess potential peering partners
- **Compliance Monitoring**: Track MANRS implementation progress
- **Industry Standards**: Align with routing security best practices

## 🌐 DNS Resolution Service

### Overview
The DNS resolution service provides comprehensive DNS record lookups using Cloudflare's 1.1.1.1 DNS server for enhanced reliability and performance.

### Supported Query Format
```
<domain>-DNS
```

### Examples
```bash
# DNS record lookup for a domain
whois -h whois.akae.re example.com-DNS

# Multiple record types returned
whois -h whois.akae.re google.com-DNS
```

### Response Information
- **A Records**: IPv4 addresses
- **AAAA Records**: IPv6 addresses
- **MX Records**: Mail exchange servers
- **NS Records**: Authoritative name servers
- **TXT Records**: Text records including SPF, DKIM
- **CNAME Records**: Canonical name aliases

### Features
- **Fixed DNS Server**: Uses Cloudflare 1.1.1.1 for consistent results
- **Multiple Record Types**: Comprehensive DNS record enumeration
- **Enhanced Reliability**: Optimized for performance and accuracy
- **Security Focus**: Uses secure DNS resolution methods

## 🔍 Traceroute Network Analysis

### Overview
Network traceroute functionality provides path analysis to show the route packets take to reach a destination.

### Supported Query Format
```
<destination>-TRACEROUTE
```
Or use the shorter alias:
```
<destination>-TRACE
```

### Examples
```bash
# Traceroute to an IP address
whois -h whois.akae.re 8.8.8.8-TRACEROUTE

# Traceroute to a domain
whois -h whois.akae.re google.com-TRACE
```

### Response Information
- **Hop-by-hop Analysis**: Each router in the path
- **Round-trip Times**: Latency measurements
- **IP Addresses**: Router IP addresses along the path
- **Hostname Resolution**: Reverse DNS lookups where available

### Use Cases
- **Network Troubleshooting**: Identify routing issues
- **Performance Analysis**: Measure network latency
- **Path Discovery**: Understand network topology
- **Connectivity Diagnosis**: Isolate network problems

## 🔐 SSL/TLS Certificate Analysis

### Overview
SSL/TLS certificate analysis service provides comprehensive certificate information for domains using rustls-based certificate analysis.

### Supported Query Format
```
<domain>-SSL
```

### Examples
```bash
# SSL certificate analysis for a domain
whois -h whois.akae.re example.com-SSL

# HTTPS certificate chain analysis
whois -h whois.akae.re google.com-SSL
```

### Response Information
- **Certificate Details**: Subject, issuer, serial number
- **Validity Period**: Not before/after dates
- **Public Key Information**: Algorithm and key size
- **Certificate Chain**: Full chain of trust
- **Extensions**: Subject Alternative Names, key usage
- **Signature Algorithm**: Cryptographic signature details

### Features
- **rustls Integration**: Modern TLS implementation
- **Certificate Chain Parsing**: Complete chain analysis
- **Security Assessment**: Certificate validation status
- **Comprehensive Details**: Full certificate information

### Use Cases
- **Security Auditing**: Verify certificate configuration
- **Compliance Checking**: Ensure proper TLS setup
- **Troubleshooting**: Diagnose SSL/TLS issues
- **Monitoring**: Track certificate expiration

## 🔍 Certificate Transparency Search

### Overview
Certificate Transparency logs service provides access to public certificate transparency logs via crt.sh API integration.

### Supported Query Format
```
<domain>-CRT
```

### Examples
```bash
# Search CT logs for a domain
whois -h whois.akae.re example.com-CRT

# Find all certificates for a domain
whois -h whois.akae.re *.google.com-CRT
```

### Response Information
- **Certificate Entries**: All CT log entries for the domain
- **Log Sources**: Which CT logs contain the certificates
- **Issue Dates**: When certificates were issued
- **Issuing CAs**: Certificate authorities that issued certificates
- **Subject Names**: All subject and SAN entries

### Features
- **crt.sh Integration**: Access to comprehensive CT log database
- **Wildcard Support**: Search for wildcard certificates
- **Historical Data**: Access to historical certificate data
- **Robust Error Handling**: Reliable API integration

### Use Cases
- **Security Research**: Monitor certificate issuance
- **Phishing Detection**: Find suspicious certificates
- **Certificate Inventory**: Catalog all certificates for a domain
- **Compliance Monitoring**: Track certificate transparency compliance

## 🎮 Minecraft Server Status

### Overview
Minecraft server status service provides server information using the Server List Ping protocol.

### Supported Query Format
```
<server>-MINECRAFT
```
Or use the shorter alias:
```
<server>-MC
```

### Examples
```bash
# Check Minecraft server status
whois -h whois.akae.re play.hypixel.net-MINECRAFT

# Quick server check with short alias
whois -h whois.akae.re mc.server.com-MC
```

### Response Information
- **Server Status**: Online/offline status
- **Player Count**: Current and maximum players
- **Server Version**: Minecraft version information
- **Message of the Day**: Server MOTD
- **Protocol Version**: Server protocol details
- **Latency**: Connection response time

### Features
- **Server List Ping**: Native Minecraft protocol implementation
- **Real-time Status**: Current server information
- **Performance Metrics**: Connection latency measurement
- **Version Compatibility**: Support for multiple Minecraft versions

### Use Cases
- **Server Monitoring**: Check server availability
- **Player Tracking**: Monitor player counts
- **Version Checking**: Verify server compatibility
- **Network Testing**: Test Minecraft server connectivity

## 🎮 Steam Integration

### Overview
The Steam integration provides comprehensive game and user information using Steam's Store API and Web API.

### Supported Query Formats
```
<app_id>-STEAM
<steam_id>-STEAM
<search_term>-STEAMSEARCH
```

### Examples
```bash
# Game information by App ID
whois -h whois.akae.re 730-STEAM

# User profile by Steam ID
whois -h whois.akae.re 76561198000000000-STEAM

# Game search
whois -h whois.akae.re Counter-Strike-STEAMSEARCH
```

### Response Information
- **Game Details**: Name, description, release date
- **Price Information**: Current price, discounts, and special offers
- **User Profiles**: Steam level, games owned, achievements
- **Store Information**: Metacritic scores, developer, publisher
- **Search Results**: Top 10 matching games with price information

### Features
- **Dual API Approach**: Steam Store API with App List fallback
- **Price Intelligence**: Automatic discount detection
- **Environment Configuration**: Supports `.env` file for Steam API key
- **App ID Detection**: Automatic differentiation between games and users

## 🎬 IMDb Integration

### Overview
The IMDb integration provides comprehensive movie and TV show information using the OMDb API.

### Supported Query Formats
```
<title>-IMDB
<imdb_id>-IMDB
<search_term>-IMDBSEARCH
```

### Examples
```bash
# Movie information by title
whois -h whois.akae.re Inception-IMDB

# Movie by IMDb ID
whois -h whois.akae.re tt1375666-IMDB

# Title search
whois -h whois.akae.re Batman-IMDBSEARCH
```

### Response Information
- **Basic Information**: Title, year, runtime, genre
- **Ratings**: IMDb rating, Metacritic score, Rotten Tomatoes
- **Cast and Crew**: Director, writer, main actors
- **Plot Summary**: Detailed plot description
- **Technical Details**: Language, country, awards

### Environment Configuration
Requires `OMDB_API_KEY` in `.env` file (obtain from http://www.omdbapi.com/apikey.aspx)

## 📦 Package Repository Queries

### Overview
Comprehensive package database integration covering 9 major package repositories.

### Supported Repositories
- **Cargo**: Rust crate registry (`rust-CARGO`)
- **NPM**: Node.js package registry (`express-NPM`)
- **PyPI**: Python Package Index (`requests-PYPI`)
- **AUR**: Arch User Repository (`firefox-AUR`)
- **Debian**: Debian package database (`nginx-DEBIAN`)
- **Ubuntu**: Ubuntu package database (`vim-UBUNTU`)
- **NixOS**: NixOS package database (`git-NIXOS`)
- **OpenSUSE**: OpenSUSE package database (`gcc-OPENSUSE`)
- **AOSC**: AOSC package database (`kernel-AOSC`)

### Response Information
- **Package Details**: Name, version, description
- **Dependencies**: Required and optional dependencies
- **Maintainer Information**: Package maintainer details
- **Download Statistics**: Download counts and popularity
- **Build Information**: Architecture, build status

## 🎭 Entertainment Services

### Wikipedia Integration
```bash
# Wikipedia article lookup
whois -h whois.akae.re "Linux-WIKIPEDIA"
```
- **Article Content**: Summary and key information
- **Links**: Related articles and external links
- **Categories**: Article categorization

### ACGC Character Database
```bash
# Anime/Comic/Game character information
whois -h whois.akae.re "Miku-ACGC"
```
- **Character Details**: Name, series, description
- **Visual Information**: Character images and artwork
- **Series Information**: Related anime/game/comic series

### Luotianyi Lyrics
```bash
# Random Luotianyi lyrics
whois -h whois.akae.re "LYRIC"
```
- **Random Lyrics**: Random Luotianyi lyric selection
- **Song Information**: Song title and artist details

## 🛠️ Development Tools

### GitHub Integration
```bash
# GitHub user information
whois -h whois.akae.re torvalds-GITHUB

# Repository information
whois -h whois.akae.re torvalds/linux-GITHUB
```

### Response Information
- **User Profiles**: Bio, location, company, follower count
- **Repository Details**: Description, language, stars, forks
- **Activity Statistics**: Commit counts, contribution activity
- **Organization Information**: Organization membership and details

### Built-in Help System
```bash
# Show all available query types
whois -h whois.akae.re "HELP"
```
- **Query Type Reference**: Complete list of supported suffixes
- **Usage Examples**: Example queries for each type
- **Feature Documentation**: Brief description of each service

## 🎯 Intelligent Query Routing

### Overview
The server implements intelligent query routing to provide the best possible results for each query.

### Routing Logic

1. **Query Type Detection**: Automatic identification of 25+ query types
2. **DN42 Detection**: Special handling for DN42 resources
3. **Private IP Handling**: RFC1918 ranges routed to DN42
4. **Smart Fallback**: Automatic fallback to DN42 for failed public queries

### DN42 Detection Rules
- **ASN Range**: AS4242420000-AS4242423999
- **Domains**: *.dn42 top-level domain
- **IP Ranges**: RFC1918 and other private ranges
- **Special Suffixes**: -DN42, -MNT suffixes

### Fallback Mechanism
```rust
1. Try public WHOIS servers (IANA → specific registries)
2. If no results or error → try DN42 WHOIS
3. Return best available result
```

## 🌐 Web Dashboard API

### Endpoints

#### GET `/api/stats`
Returns comprehensive statistics in JSON format.

```json
{
  "total_requests": 12345,
  "requests_per_minute": 8.5,
  "average_response_time_ms": 150.0,
  "total_kb_served": 98765.4,
  "kb_per_minute": 45.2,
  "current_connections": 3,
  "uptime_seconds": 86400,
  "query_type_distribution": {
    "Domain": 3456,
    "IPv4": 2345,
    "ASN": 1234,
    "Geo": 987,
    "BGPTool": 654
  },
  "daily_stats_24h": [...],
  "daily_stats_30d": [...]
}
```

#### GET `/`
Serves the web dashboard with real-time statistics and query testing interface.

### Auto-refresh
- Statistics refresh every 30 seconds
- Charts update automatically
- Theme persistence across sessions

## 📈 Statistics and Monitoring

### Metrics Collected
- **Request Counts**: Total and per-minute rates
- **Response Times**: Average processing times
- **Data Transfer**: Bytes served and transfer rates
- **Connection Tracking**: Active connection counts
- **Query Type Distribution**: Breakdown by query types
- **Historical Data**: 24-hour and 30-day retention

### Persistence
- Statistics saved automatically on server shutdown
- Data loaded on server startup
- JSON format for easy processing

### Chart Types
- **Line Charts**: Request trends over time
- **Bar Charts**: Data transfer patterns
- **Distribution Charts**: Query type breakdowns

## 🔧 Technical Implementation

### Using as a Rust Library

The whois-server can be used as a library in other Rust projects. Simply add it as a dependency:

```toml
[dependencies]
whois-server = { git = "https://github.com/Akaere-NetWorks/whois-server.git" }
```

Then use the simple API:

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

For more details and examples, see [LIBRARY_USAGE.md](LIBRARY_USAGE.md).

### Dependencies
- **reqwest**: HTTP client for external API calls
- **serde**: JSON serialization/deserialization
- **axum**: Web framework for dashboard
- **tokio**: Async runtime
- **Chart.js**: Frontend charting library

### Performance Optimizations
- **Async Processing**: Non-blocking network operations
- **Connection Pooling**: Efficient HTTP client reuse
- **Timeout Handling**: Configurable timeouts for reliability
- **Error Recovery**: Graceful degradation for external service failures

### Security Features
- **Input Validation**: Query sanitization and validation
- **Rate Limiting**: Protection against abuse (configurable)
- **CORS Support**: Secure cross-origin resource sharing
- **User-Agent Headers**: Proper identification in API calls 
