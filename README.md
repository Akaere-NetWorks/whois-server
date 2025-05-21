<div align="center">

# 🌐 WHOIS Server

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![RFC 3912](https://img.shields.io/badge/RFC-3912-green.svg)](https://datatracker.ietf.org/doc/html/rfc3912)
[![DN42](https://img.shields.io/badge/DN42-Compatible-blueviolet)](https://dn42.eu/)

**A high-performance WHOIS server with automatic DN42 query detection capabilities.**

*Deployed at [whois.akae.re](https://akae.re) - Try it now!*

</div>

## 📑 Table of Contents

- [🌐 WHOIS Server](#-whois-server)
  - [📑 Table of Contents](#-table-of-contents)
  - [✨ Features](#-features)
  - [🌍 Public Instance](#-public-instance)
  - [🚀 Installation](#-installation)
  - [🔧 Usage](#-usage)
    - [Running the server](#running-the-server)
    - [Command-line options](#command-line-options)
    - [Testing with WHOIS clients](#testing-with-whois-clients)
  - [🔍 Query Types and Forwarding Rules](#-query-types-and-forwarding-rules)
  - [📜 License](#-license)

## ✨ Features

- **Standard WHOIS protocol** support (RFC 3912)
- **Automatic query type detection** (domain, IP address, ASN)
- **Smart forwarding** to the appropriate WHOIS server
- **DN42 support** - automatic detection of private IP addresses and DN42 networks (ASNs starting with AS42424)
- **IPv4 and IPv6 support**
- **High performance** - asynchronous processing of multiple connections
- **Configurable** - timeout and concurrent connection limits
- **Robust error handling** - graceful handling of network issues

## 🌍 Public Instance

A public instance of this WHOIS server is deployed at **whois.akae.re**. You can query it using:

```bash
whois -h whois.akae.re example.com
```

Or to query an ASN:

```bash
whois -h whois.akae.re AS213605
```

DN42-specific queries are automatically routed to the DN42 WHOIS server:

```bash
whois -h whois.akae.re AS4242420000
```

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
# With default settings (port 43)
cargo run --release

# With custom port
cargo run --release -- --port 4343

# With debug output enabled
cargo run --release -- --debug

# With specific listen address
cargo run --release -- --host 127.0.0.1 

# Run in blocking mode (better for some Windows environments)
cargo run --release -- --use-blocking
```

### Command-line options

```
Options:
  -h, --host <HOST>              Listen address [default: 0.0.0.0]
  -p, --port <PORT>              Listen port [default: 43]
  -d, --debug                    Enable debug output
  -t, --trace                    Enable trace output (extremely verbose)
  --max-connections <N>          Maximum concurrent connections [default: 100]
  --timeout <SECONDS>            Connection timeout in seconds [default: 10]
  --dump-traffic                 Write raw queries and responses to files for debugging
  --dump-dir <DIR>               Dump traffic directory [default: dumps]
  --use-blocking                 Use blocking (non-async) network operations
  --help                         Print help
  --version                      Print version
```

### Testing with WHOIS clients

```bash
# Linux/Mac
whois -h localhost -p 43 example.com

# Windows (using PowerShell)
(Invoke-WebRequest -Uri "http://localhost:43" -Method Post -Body "example.com").Content

# Windows (using telnet)
telnet localhost 43
# Then type your query (e.g., example.com) and press Enter
```

## 🔍 Query Types and Forwarding Rules

| Query Type | Treatment | Example |
|------------|-----------|---------|
| **Domain queries** | Forwarded through IANA WHOIS server to identify and route to the appropriate registrar's WHOIS server | `example.com` |
| **Public IP addresses** | Forwarded through IANA WHOIS server to identify and route to the appropriate RIR | `8.8.8.8` |
| **Private IP addresses** | Forwarded to DN42 WHOIS server | `10.10.10.10` |
| **Regular ASNs** | Forwarded through IANA WHOIS server | `AS15169` |
| **DN42 ASNs** | Forwarded to DN42 WHOIS server | `AS4242420000` |

## 📜 License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 