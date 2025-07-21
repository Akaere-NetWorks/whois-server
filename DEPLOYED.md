# whois.akae.re Deployment Information

This document describes the official deployment instance of the project, whois.akae.re.

## Deployment Address

- **Main Domain**: whois.akae.re
- **Port**: 43 (Standard WHOIS port)
- **Command**: `whois -h whois.akae.re <query>`

## Service Features

whois.akae.re is the official deployment instance of this project, providing the following features:

- WHOIS service compliant with RFC 3912 standard
- Support for regular domain names, IP addresses, and AS number queries
- Platform-aware DN42 network query support (automatic detection and forwarding)
- Advanced query types: IRR Explorer, Looking Glass, BGP Tools, RPKI, MANRS
- Network services: DNS resolution, traceroute analysis, SSL certificate analysis
- Security services: Certificate Transparency search, RPKI validation
- Gaming services: Minecraft server status queries
- Geo-location services with multiple data sources
- Email search and contact information lookup
- IPv4 and IPv6 dual-stack support
- Real-time web dashboard and statistics
- High availability and low latency

## Usage Examples

### Query Domain Information

```bash
$ whois -h whois.akae.re akae.re
% Akaere NetWorks Whois Server
% The objects are in RPSL format
% Please report any issues to noc@akae.re

Domain Name: akae.re
Registry Domain ID: D503300000040559064-AGRS
Registrar WHOIS Server: whois.gandi.net
Registrar URL: http://www.gandi.net/
...
```

### Query IP Address

```bash
$ whois -h whois.akae.re 8.8.8.8
% Akaere NetWorks Whois Server
% The objects are in RPSL format
% Please report any issues to noc@akae.re

NetRange:       8.0.0.0 - 8.255.255.255
CIDR:           8.0.0.0/8
NetName:        LVLT-ORG-8-8
NetHandle:      NET-8-0-0-0-1
...
```

### Query AS Number

```bash
$ whois -h whois.akae.re AS213605
% Akaere NetWorks Whois Server
% The objects are in RPSL format
% Please report any issues to noc@akae.re

aut-num:        AS213605
as-name:        Pysio-NetWork
org:            ORG-LA1994-RIPE
...
```

### Query DN42 Network Resources

```bash
$ whois -h whois.akae.re AS4242420000
% Akaere NetWorks Whois Server
% The objects are in RPSL format
% Please report any issues to noc@akae.re

aut-num:            AS4242420000
as-name:            DNFREE-AS
descr:              DN42 Free (reserved for future use)
...
```

### Advanced Query Examples

```bash
# RPKI Validation
$ whois -h whois.akae.re 192.0.2.0/24-AS213605-RPKI

# MANRS Compliance Check
$ whois -h whois.akae.re AS213605-MANRS

# IRR Explorer Analysis
$ whois -h whois.akae.re 203.0.113.0/24-IRR

# Looking Glass (BIRD-style)
$ whois -h whois.akae.re 1.1.1.0-LG

# Geo-location
$ whois -h whois.akae.re 8.8.8.8-GEO

# DNS Resolution
$ whois -h whois.akae.re example.com-DNS

# Traceroute Analysis
$ whois -h whois.akae.re 8.8.8.8-TRACEROUTE

# SSL Certificate Analysis
$ whois -h whois.akae.re example.com-SSL

# Certificate Transparency Search
$ whois -h whois.akae.re example.com-CRT

# Minecraft Server Status
$ whois -h whois.akae.re play.hypixel.net-MC
```

## Technical Specifications

whois.akae.re is deployed in the following environment:

- **Operating Environment**: Linux Ubuntu 22.04 LTS
- **Hardware Configuration**: 2 CPU cores, 4GB RAM
- **Network Connection**: 1Gbps, dual-stack IPv4/IPv6
- **Location**: Hong Kong (HK) Data Center, China
- **Maintenance Window**: First Sunday of each month, 02:00-04:00 UTC

## Contact Information

- **Operations Email**: noc@akae.re
- **Website**: https://akae.re
- **Maintainer**: Akaere Networks

## Service Level Agreement (SLA)

- **Availability Target**: 99.9% (no more than 43 minutes of unplanned downtime per month)
- **Maximum Response Time**: < 200ms
- **Incident Response Time**: < 2 hours

---

*Last updated: July 20, 2025* 