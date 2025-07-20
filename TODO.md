# TODO

## Feature Roadmap

### 2. DNS Resolution within WHOIS
- [ ] Integrate DNS resolver functionality into WHOIS queries
- [ ] Add DNS record type detection (A, AAAA, MX, NS, etc.)
- [ ] Implement reverse DNS lookup capabilities
- [ ] Add DNS query results to WHOIS response formatting
- [ ] Support for DNS-over-HTTPS (DoH) and DNS-over-TLS (DoT)
- [ ] Add DNS resolution statistics and monitoring

### 3. Traceroute Integration within WHOIS
- [ ] Implement traceroute functionality for IP addresses
- [ ] Add cross-platform traceroute support (Windows/Unix)
- [ ] Integrate traceroute results into WHOIS responses
- [ ] Add hop-by-hop analysis with ASN information
- [ ] Implement timeout and retry logic for traceroute
- [ ] Add traceroute visualization in web dashboard
- [ ] Support for both IPv4 and IPv6 traceroute

## Implementation Notes

### Caching Architecture
- Utilize existing LMDB storage infrastructure
- Implement cache layers for different data types
- Add cache warming strategies for frequently queried data

### DNS Integration
- Leverage existing async infrastructure for non-blocking DNS queries
- Consider integration with existing geo-location services
- Add DNS query type detection to core query processing

### Traceroute Implementation
- Use platform-specific implementations (similar to DN42 backend strategy)
- Integrate with BGP tools for enhanced path analysis
- Consider rate limiting to prevent abuse

## Priority
1. **High**: IANA WHOIS caching (improves performance significantly)
2. **Medium**: DNS resolution (enhances query capabilities)
3. **Medium**: Traceroute integration (adds network diagnostic value)