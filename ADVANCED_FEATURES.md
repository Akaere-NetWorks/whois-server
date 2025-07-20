# 🛠️ Advanced Features Documentation

This document provides detailed technical information about the advanced features implemented in the WHOIS server.

## 📋 Table of Contents

- [Platform-Aware DN42 Integration](#platform-aware-dn42-integration)
- [IRR Explorer Integration](#irr-explorer-integration)
- [Looking Glass Services](#looking-glass-services)
- [RADB Direct Access](#radb-direct-access)
- [RPKI Validation](#rpki-validation)
- [MANRS Integration](#manrs-integration)
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

## 🔐 RPKI Validation

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

## 🎯 Intelligent Query Routing

### Overview
The server implements intelligent query routing to provide the best possible results for each query.

### Routing Logic

1. **Query Type Detection**: Automatic identification of 11+ query types
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