<div align="center">

# 🌐 WHOIS Server

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![RFC 3912](https://img.shields.io/badge/RFC-3912-green.svg)](https://datatracker.ietf.org/doc/html/rfc3912)
[![DN42](https://img.shields.io/badge/DN42-Compatible-blueviolet)](https://dn42.eu/)

**A high-performance WHOIS server with DN42 support, geo-location services, and comprehensive query capabilities.**

*Deployed at [whois.akae.re](https://whois.akae.re) - Try it now!*

</div>

## 📑 Table of Contents

- [🌐 WHOIS Server](#-whois-server)
  - [📑 Table of Contents](#-table-of-contents)
  - [✨ Features](#-features)
  - [🌍 Public Instance](#-public-instance)
  - [🖥️ Web Dashboard](#️-web-dashboard)
  - [🚀 Installation](#-installation)
  - [🔧 Usage](#-usage)
    - [Running the server](#running-the-server)
    - [Command-line options](#command-line-options)
    - [Testing with WHOIS clients](#testing-with-whois-clients)
  - [🔍 Query Types and Features](#-query-types-and-features)
    - [Standard WHOIS Queries](#standard-whois-queries)
    - [Enhanced Query Types](#enhanced-query-types)
    - [Geo-location Services](#geo-location-services)
    - [Network Intelligence & Advanced Features](#network-intelligence--advanced-features)
  - [🛠️ Advanced Query Features](#️-advanced-query-features)
    - [IRR Explorer Integration](#irr-explorer-integration--irr-suffix)
    - [Looking Glass Services](#looking-glass-services--lg-suffix)
    - [RADB Direct Access](#radb-direct-access--radb-suffix)
  - [📊 Statistics and Monitoring](#-statistics-and-monitoring)
  - [🏗️ Architecture](#️-architecture)
  - [📜 License](#-license)

## ✨ Features

- **🚀 High Performance** - Asynchronous Rust implementation with configurable connection limits
- **🌐 RFC 3912 Compliant** - Full WHOIS protocol support
- **🔍 Smart Query Detection** - Automatic identification of domains, IP addresses, ASNs, and special query types
- **🌟 DN42 Integration** - Seamless support for DN42 networks and .dn42 domains
- **📍 Geo-location Services** - Built-in IP geolocation using multiple data sources
- **🔧 BGP Tools Integration** - Network analysis and BGP information queries
- **📧 Email Search** - Contact information lookup capabilities
- **🛡️ IRR Explorer Integration** - Comprehensive routing registry analysis with RPKI validation
- **🔭 Looking Glass Services** - RIPE RIS BGP data in BIRD routing daemon format
- **📊 RADB Direct Access** - Routing Assets Database queries for AS-SET and route objects
- **📈 Real-time Statistics** - Comprehensive usage tracking and monitoring
- **🌐 Web Dashboard** - Modern web interface for statistics and testing
- **⚡ Dual Operation Modes** - Both async and blocking network operations
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
whois -h whois.akae.re AS213605-RADB

# IRR Explorer analysis
whois -h whois.akae.re 192.0.2.0/24-IRR

# Looking Glass (BIRD-style routing data)
whois -h whois.akae.re AS213605-LG
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

## 🚀 Installation

Ensure you have Rust and Cargo installed, then:

```bash
# Clone the repository
git clone https://github.com/yourusername/whois-server.git
cd whois-server

# Build in release mode
cargo build --release

# The executable will be available at target/release/whois-server
```

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

# Run in blocking mode (better for some environments)
cargo run --release -- --use-blocking

# Enable traffic dumping for debugging
cargo run --release -- --dump-traffic --dump-dir ./logs
```

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
      --use-blocking             Use blocking (non-async) network operations
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
| **-RADB** | `AS213605-RADB` | Query RADB (Routing Assets Database) directly |
| **-IRR** | `192.0.2.0/24-IRR` | IRR Explorer - comprehensive routing registry analysis |
| **-LG** | `AS213605-LG` | Looking Glass - RIPE RIS BGP routing data in BIRD format |

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
whois -h whois.akae.re AS213605-LG
```

### RADB Direct Access (`-RADB` suffix)

Direct queries to the Routing Assets Database (RADB) for AS-SET expansions, route objects, and routing policies. This is particularly useful for network operators managing routing policies and filters.

**Example:**
```bash
whois -h whois.akae.re AS-SET:AS-EXAMPLE-RADB
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

The server is built with a modular Rust architecture:

```
src/
├── main.rs          # Application entry point and initialization
├── config.rs        # Configuration and command-line parsing
├── query.rs         # Query type detection and analysis
├── whois.rs         # WHOIS protocol client implementations  
├── web.rs           # Web dashboard and API endpoints
├── stats.rs         # Statistics collection and persistence
├── email.rs         # Email search functionality
├── bgptool.rs       # BGP tools integration
├── irr.rs           # IRR Explorer integration
├── looking_glass.rs # RIPE RIS Looking Glass services
├── utils.rs         # Utility functions
├── server/          # Server implementations
│   ├── mod.rs       # Server module exports
│   ├── async_server.rs     # Async TCP server
│   ├── blocking_server.rs  # Blocking TCP server
│   ├── connection.rs       # Connection handling logic
│   └── utils.rs     # Server utility functions
└── geo/             # Geo-location services
    ├── mod.rs       # Main geo module
    ├── services.rs  # Service orchestration
    ├── types.rs     # Data type definitions
    ├── formatters.rs # Output formatting
    ├── ripe_api.rs  # RIPE database integration
    ├── ipinfo_api.rs # IPInfo service integration
    ├── constants.rs # Geographic constants
    └── utils.rs     # Geographic utility functions
```

### Key Components

- **Query Engine** - Intelligent query parsing and type detection with 11+ query types
- **Multi-protocol Support** - RFC 3912 WHOIS + custom extensions and advanced features
- **Dual Server Architecture** - Both async (Tokio) and blocking server implementations
- **Web Interface** - Axum-based REST API and dashboard with real-time updates
- **Statistics Engine** - Real-time metrics collection with 24h/30d historical data
- **Advanced Network Tools** - IRR Explorer, Looking Glass, BGP Tools integration
- **External Integrations** - RIPE, IPInfo, IRR Explorer, RADB, and DN42 data sources
- **Intelligent Routing** - Smart query routing with fallback mechanisms

## 📜 License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 