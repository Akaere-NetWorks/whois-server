use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use anyhow::Result;
use tracing::{debug, warn, error};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use rand::random;

/// DNS message header
#[derive(Debug)]
struct DnsHeader {
    id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

/// DNS question
#[derive(Debug)]
struct DnsQuestion {
    name: String,
    qtype: u16,
    qclass: u16,
}

/// DNS resource record
#[derive(Debug, Clone)]
struct DnsRecord {
    name: String,
    rtype: u16,
    class: u16,
    ttl: u32,
    data: Vec<u8>,
}

/// DNS response containing all sections
#[derive(Debug)]
struct DnsResponse {
    answers: Vec<DnsRecord>,
    authority: Vec<DnsRecord>,
    additional: Vec<DnsRecord>,
}

/// DNS resolver service for recursive domain and reverse DNS lookups
pub struct DnsService {
    root_servers: Vec<SocketAddr>,
}

impl DnsHeader {
    fn new(id: u16) -> Self {
        Self {
            id,
            flags: 0x0100, // Standard query with recursion desired
            qdcount: 1,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.flags.to_be_bytes());
        bytes.extend_from_slice(&self.qdcount.to_be_bytes());
        bytes.extend_from_slice(&self.ancount.to_be_bytes());
        bytes.extend_from_slice(&self.nscount.to_be_bytes());
        bytes.extend_from_slice(&self.arcount.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 12 {
            return Err(anyhow::anyhow!("DNS header too short"));
        }
        
        Ok(Self {
            id: u16::from_be_bytes([bytes[0], bytes[1]]),
            flags: u16::from_be_bytes([bytes[2], bytes[3]]),
            qdcount: u16::from_be_bytes([bytes[4], bytes[5]]),
            ancount: u16::from_be_bytes([bytes[6], bytes[7]]),
            nscount: u16::from_be_bytes([bytes[8], bytes[9]]),
            arcount: u16::from_be_bytes([bytes[10], bytes[11]]),
        })
    }
}

impl DnsService {
    /// Create a new DNS service with recursive resolution
    pub async fn new() -> Result<Self> {
        // Root DNS servers (a few of them)
        let root_servers = vec![
            "198.41.0.4:53".parse()?,      // a.root-servers.net
            "199.9.14.201:53".parse()?,    // b.root-servers.net  
            "192.33.4.12:53".parse()?,     // c.root-servers.net
            "199.7.91.13:53".parse()?,     // d.root-servers.net
        ];
        
        debug!("DNS recursive resolver initialized with {} root servers", root_servers.len());
        Ok(Self { root_servers })
    }

    /// Perform DNS query for domain names
    pub async fn query_dns(&self, domain: &str) -> Result<String> {
        debug!("Performing recursive DNS query for domain: {}", domain);

        let mut output = String::new();

        // Query different record types
        let record_types = [
            (1, "A"),      // A record
            (28, "AAAA"),  // AAAA record  
            (15, "MX"),    // MX record
            (16, "TXT"),   // TXT record
            (2, "NS"),     // NS record
            (6, "SOA"),    // SOA record
        ];

        for (qtype, type_name) in record_types {
            match self.resolve_recursive(domain, qtype).await {
                Ok(records) => {
                    if !records.is_empty() {
                        output.push_str(&format!("\n{} Records for {}:\n", type_name, domain));
                        for record in records {
                            let formatted = self.format_record(&record, type_name);
                            output.push_str(&formatted);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to resolve {} records for {}: {}", type_name, domain, e);
                }
            }
        }

        if output.is_empty() {
            output = format!("No DNS records found for domain: {}\n", domain);
        } else {
            output = format!("Recursive DNS Resolution Results for: {}\n{}", domain, output);
        }

        debug!("DNS query completed for {}, result length: {} bytes", domain, output.len());
        Ok(output)
    }

    /// Perform reverse DNS lookup for IP addresses
    pub async fn query_rdns(&self, ip: IpAddr) -> Result<String> {
        debug!("Performing recursive reverse DNS query for IP: {}", ip);

        let ptr_name = match ip {
            IpAddr::V4(ipv4) => self.create_ipv4_ptr_name(ipv4),
            IpAddr::V6(ipv6) => self.create_ipv6_ptr_name(ipv6),
        };

        debug!("Generated PTR name: {}", ptr_name);

        match self.resolve_recursive(&ptr_name, 12).await { // 12 = PTR record type
            Ok(records) => {
                let mut output = format!("Recursive Reverse DNS Results for {}:\n\nPTR Records:\n", ip);
                for record in records {
                    let formatted = self.format_record(&record, "PTR");
                    output.push_str(&formatted);
                }
                if output.lines().count() <= 3 {
                    output = format!("No reverse DNS record found for IP: {}\n", ip);
                }
                debug!("rDNS query completed for {}, result length: {} bytes", ip, output.len());
                Ok(output)
            }
            Err(e) => {
                warn!("Failed to query PTR records for {}: {}", ip, e);
                Ok(format!("Reverse DNS lookup failed for {}: {}\n", ip, e))
            }
        }
    }


    /// Create IPv4 PTR name (e.g., 1.1.1.1 -> 1.1.1.1.in-addr.arpa)
    fn create_ipv4_ptr_name(&self, ip: Ipv4Addr) -> String {
        let octets = ip.octets();
        format!("{}.{}.{}.{}.in-addr.arpa", 
            octets[3], octets[2], octets[1], octets[0])
    }

    /// Create IPv6 PTR name (e.g., 2001:db8::1 -> 1.0.0.0...ip6.arpa)
    fn create_ipv6_ptr_name(&self, ip: Ipv6Addr) -> String {
        let segments = ip.segments();
        let mut nibbles = Vec::new();
        
        for segment in segments.iter().rev() {
            let bytes = segment.to_be_bytes();
            for byte in bytes.iter().rev() {
                nibbles.push(format!("{:x}", byte & 0x0f));
                nibbles.push(format!("{:x}", (byte & 0xf0) >> 4));
            }
        }
        
        format!("{}.ip6.arpa", nibbles.join("."))
    }

    /// Recursive DNS resolution
    async fn resolve_recursive(&self, domain: &str, qtype: u16) -> Result<Vec<DnsRecord>> {
        debug!("Starting recursive resolution for {} (type {})", domain, qtype);
        
        // Start with root servers
        let mut nameservers = self.root_servers.clone();
        let mut depth = 0;
        const MAX_DEPTH: usize = 10;

        loop {
            if depth >= MAX_DEPTH {
                return Err(anyhow::anyhow!("Maximum recursion depth reached"));
            }

            depth += 1;
            debug!("Recursion depth: {}, querying {} servers", depth, nameservers.len());

            let mut best_response = None;
            
            // Try each nameserver
            for ns in &nameservers {
                match self.query_server(*ns, domain, qtype).await {
                    Ok(response) => {
                        debug!("Got response from {}: {} answers, {} authority, {} additional", 
                               ns, response.answers.len(), response.authority.len(), response.additional.len());
                        
                        // If we got answers, return them
                        if !response.answers.is_empty() {
                            return Ok(response.answers);
                        }
                        
                        // If we got authority records, use them for next iteration
                        if !response.authority.is_empty() || !response.additional.is_empty() {
                            best_response = Some(response);
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to query {}: {}", ns, e);
                        continue;
                    }
                }
            }

            // Extract new nameservers from authority/additional sections
            if let Some(response) = best_response {
                nameservers = self.extract_nameservers(&response)?;
                if nameservers.is_empty() {
                    return Err(anyhow::anyhow!("No more nameservers to try"));
                }
            } else {
                return Err(anyhow::anyhow!("All nameservers failed"));
            }
        }
    }

    /// Query a specific DNS server
    async fn query_server(&self, server: SocketAddr, domain: &str, qtype: u16) -> Result<DnsResponse> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        
        // Build DNS query
        let query_id = random::<u16>();
        let mut packet = Vec::new();
        
        // Header
        let header = DnsHeader::new(query_id);
        packet.extend_from_slice(&header.to_bytes());
        
        // Question
        packet.extend_from_slice(&self.encode_domain_name(domain));
        packet.extend_from_slice(&qtype.to_be_bytes());  // QTYPE
        packet.extend_from_slice(&1u16.to_be_bytes());   // QCLASS (IN)
        
        // Send query with timeout
        debug!("Sending DNS query to {} for {} (type {})", server, domain, qtype);
        socket.send_to(&packet, server).await?;
        
        let mut buffer = vec![0; 512];
        let (len, _) = timeout(Duration::from_secs(5), socket.recv_from(&mut buffer)).await??;
        buffer.truncate(len);
        
        // Parse response
        self.parse_dns_response(&buffer)
    }

    /// Encode domain name to DNS wire format
    fn encode_domain_name(&self, domain: &str) -> Vec<u8> {
        let mut encoded = Vec::new();
        
        for label in domain.split('.') {
            if label.is_empty() {
                continue;
            }
            encoded.push(label.len() as u8);
            encoded.extend_from_slice(label.as_bytes());
        }
        encoded.push(0); // Root label
        
        encoded
    }

    /// Parse DNS response
    fn parse_dns_response(&self, buffer: &[u8]) -> Result<DnsResponse> {
        if buffer.len() < 12 {
            return Err(anyhow::anyhow!("Response too short"));
        }

        let header = DnsHeader::from_bytes(buffer)?;
        debug!("Parsed DNS header: {} answers, {} authority, {} additional", 
               header.ancount, header.nscount, header.arcount);

        let mut offset = 12;
        
        // Skip questions
        for _ in 0..header.qdcount {
            offset = self.skip_name(buffer, offset)?;
            offset += 4; // QTYPE + QCLASS
        }

        // Parse answers
        let mut answers = Vec::new();
        for _ in 0..header.ancount {
            let (record, new_offset) = self.parse_record(buffer, offset)?;
            answers.push(record);
            offset = new_offset;
        }

        // Parse authority
        let mut authority = Vec::new();
        for _ in 0..header.nscount {
            let (record, new_offset) = self.parse_record(buffer, offset)?;
            authority.push(record);
            offset = new_offset;
        }

        // Parse additional
        let mut additional = Vec::new();
        for _ in 0..header.arcount {
            let (record, new_offset) = self.parse_record(buffer, offset)?;
            additional.push(record);
            offset = new_offset;
        }

        Ok(DnsResponse {
            answers,
            authority,
            additional,
        })
    }

    /// Parse a DNS resource record
    fn parse_record(&self, buffer: &[u8], offset: usize) -> Result<(DnsRecord, usize)> {
        let (name, mut offset) = self.parse_name(buffer, offset)?;
        
        if offset + 10 > buffer.len() {
            return Err(anyhow::anyhow!("Record too short"));
        }

        let rtype = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        let class = u16::from_be_bytes([buffer[offset + 2], buffer[offset + 3]]);
        let ttl = u32::from_be_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]]);
        let rdlength = u16::from_be_bytes([buffer[offset + 8], buffer[offset + 9]]) as usize;
        offset += 10;

        if offset + rdlength > buffer.len() {
            return Err(anyhow::anyhow!("Record data too short"));
        }

        let data = buffer[offset..offset + rdlength].to_vec();
        offset += rdlength;

        Ok((DnsRecord {
            name,
            rtype,
            class,
            ttl,
            data,
        }, offset))
    }

    /// Parse domain name from DNS message
    fn parse_name(&self, buffer: &[u8], mut offset: usize) -> Result<(String, usize)> {
        let mut name_parts = Vec::new();
        let mut jumped = false;
        let original_offset = offset;

        loop {
            if offset >= buffer.len() {
                return Err(anyhow::anyhow!("Unexpected end of buffer while parsing name"));
            }

            let len = buffer[offset];
            
            if len == 0 {
                offset += 1;
                break;
            } else if len & 0xC0 == 0xC0 {
                // Compression pointer
                if !jumped {
                    offset += 2;
                }
                let pointer = ((len as usize & 0x3F) << 8) | buffer[offset - 1] as usize;
                if pointer >= buffer.len() {
                    return Err(anyhow::anyhow!("Invalid compression pointer"));
                }
                offset = pointer;
                jumped = true;
            } else {
                // Regular label
                offset += 1;
                if offset + len as usize > buffer.len() {
                    return Err(anyhow::anyhow!("Label extends beyond buffer"));
                }
                let label = String::from_utf8_lossy(&buffer[offset..offset + len as usize]);
                name_parts.push(label.into_owned());
                offset += len as usize;
            }
        }

        let final_offset = if jumped { original_offset + 2 } else { offset };
        Ok((name_parts.join("."), final_offset))
    }

    /// Skip over a domain name in DNS message
    fn skip_name(&self, buffer: &[u8], mut offset: usize) -> Result<usize> {
        loop {
            if offset >= buffer.len() {
                return Err(anyhow::anyhow!("Unexpected end while skipping name"));
            }

            let len = buffer[offset];
            
            if len == 0 {
                return Ok(offset + 1);
            } else if len & 0xC0 == 0xC0 {
                return Ok(offset + 2);
            } else {
                offset += 1 + len as usize;
            }
        }
    }

    /// Extract nameservers from DNS response
    fn extract_nameservers(&self, response: &DnsResponse) -> Result<Vec<SocketAddr>> {
        let mut nameservers = Vec::new();

        // Look for A records in additional section that correspond to NS records
        for ns_record in &response.authority {
            if ns_record.rtype == 2 { // NS record
                let ns_name = self.parse_name_from_data(&ns_record.data)?;
                
                // Find corresponding A record in additional section
                for additional in &response.additional {
                    if additional.rtype == 1 && additional.name == ns_name { // A record
                        if additional.data.len() >= 4 {
                            let ip = Ipv4Addr::new(
                                additional.data[0],
                                additional.data[1], 
                                additional.data[2],
                                additional.data[3]
                            );
                            nameservers.push(SocketAddr::from((ip, 53)));
                        }
                    }
                }
            }
        }

        // If no A records found, try to resolve NS names (simplified)
        if nameservers.is_empty() {
            // For simplicity, use some well-known DNS servers as fallback
            nameservers.extend_from_slice(&[
                "8.8.8.8:53".parse().unwrap(),
                "1.1.1.1:53".parse().unwrap(),
            ]);
        }

        Ok(nameservers)
    }

    /// Parse domain name from record data
    fn parse_name_from_data(&self, data: &[u8]) -> Result<String> {
        let (name, _) = self.parse_name(data, 0)?;
        Ok(name)
    }

    /// Format DNS record for display
    fn format_record(&self, record: &DnsRecord, record_type: &str) -> String {
        match record.rtype {
            1 => { // A record
                if record.data.len() >= 4 {
                    let ip = Ipv4Addr::new(record.data[0], record.data[1], record.data[2], record.data[3]);
                    format!("  {} (TTL: {})\n", ip, record.ttl)
                } else {
                    format!("  Invalid A record (TTL: {})\n", record.ttl)
                }
            }
            28 => { // AAAA record
                if record.data.len() >= 16 {
                    let mut addr = [0u8; 16];
                    addr.copy_from_slice(&record.data[0..16]);
                    let ip = Ipv6Addr::from(addr);
                    format!("  {} (TTL: {})\n", ip, record.ttl)
                } else {
                    format!("  Invalid AAAA record (TTL: {})\n", record.ttl)
                }
            }
            15 => { // MX record
                if record.data.len() >= 2 {
                    let preference = u16::from_be_bytes([record.data[0], record.data[1]]);
                    match self.parse_name_from_data(&record.data[2..]) {
                        Ok(exchange) => format!("  {} {} (TTL: {})\n", preference, exchange, record.ttl),
                        Err(_) => format!("  Invalid MX record (TTL: {})\n", record.ttl),
                    }
                } else {
                    format!("  Invalid MX record (TTL: {})\n", record.ttl)
                }
            }
            16 => { // TXT record
                let text = String::from_utf8_lossy(&record.data);
                format!("  \"{}\" (TTL: {})\n", text, record.ttl)
            }
            2 | 12 => { // NS or PTR record
                match self.parse_name_from_data(&record.data) {
                    Ok(name) => format!("  {} (TTL: {})\n", name, record.ttl),
                    Err(_) => format!("  Invalid {} record (TTL: {})\n", record_type, record.ttl),
                }
            }
            _ => {
                format!("  {} record with {} bytes data (TTL: {})\n", record_type, record.data.len(), record.ttl)
            }
        }
    }

    /// Detect if a query string is a domain name
    pub fn is_domain_name(query: &str) -> bool {
        // Basic domain validation
        if query.is_empty() || query.len() > 253 {
            return false;
        }

        // Must contain at least one dot
        if !query.contains('.') {
            return false;
        }

        // Check if it's an IP address
        if query.parse::<IpAddr>().is_ok() {
            return false;
        }

        // Check for valid domain characters
        let parts: Vec<&str> = query.split('.').collect();
        if parts.len() < 2 {
            return false;
        }

        for part in parts {
            if part.is_empty() || part.len() > 63 {
                return false;
            }
            
            // Check for valid domain characters (letters, numbers, hyphens)
            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return false;
            }
            
            // Cannot start or end with hyphen
            if part.starts_with('-') || part.ends_with('-') {
                return false;
            }
        }

        true
    }

    /// Parse IP address from query string
    pub fn parse_ip_address(query: &str) -> Option<IpAddr> {
        query.parse::<IpAddr>().ok()
    }
}

/// Process DNS query with -DNS suffix
pub async fn process_dns_query(query: &str) -> Result<String> {
    let dns_service = DnsService::new().await?;
    
    // Remove -DNS suffix if present
    let clean_query = if query.to_uppercase().ends_with("-DNS") {
        &query[..query.len() - 4]
    } else {
        query
    };

    debug!("Processing DNS query for: {}", clean_query);

    // Check if it's an IP address (for rDNS)
    if let Some(ip) = DnsService::parse_ip_address(clean_query) {
        debug!("Detected IP address, performing reverse DNS lookup");
        return dns_service.query_rdns(ip).await;
    }

    // Check if it's a domain name (for DNS)
    if DnsService::is_domain_name(clean_query) {
        debug!("Detected domain name, performing DNS lookup");
        return dns_service.query_dns(clean_query).await;
    }

    // Neither IP nor valid domain
    error!("Invalid DNS query format: {}", clean_query);
    Ok(format!("Invalid DNS query format. Please provide a valid domain name or IP address.\nQuery: {}\n", clean_query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_name_detection() {
        assert!(DnsService::is_domain_name("example.com"));
        assert!(DnsService::is_domain_name("sub.example.com"));
        assert!(DnsService::is_domain_name("test.co.uk"));
        
        assert!(!DnsService::is_domain_name("1.1.1.1"));
        assert!(!DnsService::is_domain_name("2001:db8::1"));
        assert!(!DnsService::is_domain_name("localhost"));
        assert!(!DnsService::is_domain_name(""));
        assert!(!DnsService::is_domain_name("ex ample.com"));
    }

    #[test]
    fn test_ip_parsing() {
        assert!(DnsService::parse_ip_address("1.1.1.1").is_some());
        assert!(DnsService::parse_ip_address("2001:db8::1").is_some());
        assert!(DnsService::parse_ip_address("example.com").is_none());
    }

    #[tokio::test]
    async fn test_dns_query_format() {
        // Test the query parsing logic
        let result = process_dns_query("example.com-DNS").await;
        assert!(result.is_ok());
        
        let result = process_dns_query("1.1.1.1-DNS").await;
        assert!(result.is_ok());
    }

    #[tokio::test] 
    async fn test_dns_service_creation() {
        let result = DnsService::new().await;
        assert!(result.is_ok());
    }
}