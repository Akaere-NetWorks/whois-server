use std::sync::Arc;
use std::io::{ BufRead, BufReader, Write };
use std::net::{ TcpStream, ToSocketAddrs };
use std::time::Duration;
use anyhow::Result;
use tracing::{ debug, error };
use rustls::{ ClientConfig, ClientConnection, StreamOwned };
use x509_parser::prelude::*;
use sha1::{ Sha1, Digest };
use sha2::Sha256;
use chrono::DateTime;

/// SSL certificate information structure
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub version: u32,
    pub not_before: String,
    pub not_after: String,
    pub signature_algorithm: String,
    pub public_key_algorithm: String,
    pub subject_alternative_names: Vec<String>,
    pub key_usage: Vec<String>,
    #[allow(dead_code)]
    pub extended_key_usage: Vec<String>,
    pub fingerprint_sha1: String,
    pub fingerprint_sha256: String,
    pub is_ca: bool,
    pub is_self_signed: bool,
    pub chain_length: usize,
}

/// SSL service for certificate retrieval and analysis
pub struct SslService {
    timeout: Duration,
}

impl Default for SslService {
    fn default() -> Self {
        Self::new()
    }
}

impl SslService {
    /// Create a new SSL service with default timeout
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }

    /// Create SSL service with custom timeout
    #[allow(dead_code)]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Query SSL certificate information for a domain
    pub async fn query_ssl_certificate(&self, domain: &str, port: Option<u16>) -> Result<String> {
        let port = port.unwrap_or(443);
        debug!("Querying SSL certificate for {}:{}", domain, port);

        match self.get_certificate_info(domain, port).await {
            Ok(cert_info) => {
                let output = self.format_certificate_info(&cert_info, domain, port);
                debug!("SSL certificate query completed for {}", domain);
                Ok(output)
            }
            Err(e) => {
                error!("Failed to retrieve SSL certificate for {}: {}", domain, e);
                Ok(format!("SSL Certificate Query Failed for {}:{}\nError: {}\n", domain, port, e))
            }
        }
    }

    /// Retrieve certificate information from domain
    async fn get_certificate_info(&self, domain: &str, port: u16) -> Result<CertificateInfo> {
        // Create SSL client configuration with custom verifier
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
            .with_no_client_auth();

        let server_name = rustls::ServerName::try_from(domain)?;
        let conn = ClientConnection::new(Arc::new(config), server_name)?;

        // Connect to the server
        let addr = format!("{}:{}", domain, port);
        let tcp_stream = TcpStream::connect_timeout(
            &addr
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| { anyhow::anyhow!("Unable to resolve domain: {}", domain) })?,
            self.timeout
        )?;

        tcp_stream.set_read_timeout(Some(self.timeout))?;
        tcp_stream.set_write_timeout(Some(self.timeout))?;

        let mut tls_stream = StreamOwned::new(conn, tcp_stream);

        // Perform TLS handshake by sending a basic HTTP request
        let request = format!("HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", domain);
        tls_stream.write_all(request.as_bytes())?;

        // Read response to ensure handshake completion
        let mut reader = BufReader::new(&mut tls_stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        // Get peer certificates
        let peer_certs = tls_stream.conn
            .peer_certificates()
            .ok_or_else(|| anyhow::anyhow!("No peer certificates available"))?;

        if peer_certs.is_empty() {
            return Err(anyhow::anyhow!("No certificates in chain"));
        }

        // Parse the first certificate (leaf certificate)
        let cert_der = &peer_certs[0];
        let cert_info = self.parse_certificate(cert_der.as_ref(), peer_certs.len())?;

        Ok(cert_info)
    }

    /// Parse DER-encoded certificate
    fn parse_certificate(&self, cert_der: &[u8], chain_length: usize) -> Result<CertificateInfo> {
        let (_, cert) = X509Certificate::from_der(cert_der)?;

        // Extract basic information
        let subject = cert.subject().to_string();
        let issuer = cert.issuer().to_string();
        let serial_number = format!("{:X}", cert.serial);
        let version = cert.version().0;

        // Format dates
        let not_before = self.format_asn1_time(&cert.validity().not_before)?;
        let not_after = self.format_asn1_time(&cert.validity().not_after)?;

        // Signature algorithm
        let signature_algorithm = cert.signature_algorithm.algorithm.to_string();

        // Public key algorithm
        let public_key_algorithm = cert.public_key().algorithm.algorithm.to_string();

        // Subject Alternative Names
        let mut san_list = Vec::new();
        for ext in cert.extensions() {
            if ext.oid == x509_parser::oid_registry::OID_X509_EXT_SUBJECT_ALT_NAME {
                if let Ok((_, san)) = SubjectAlternativeName::from_der(ext.value) {
                    for name in san.general_names {
                        match name {
                            GeneralName::DNSName(dns) => san_list.push(format!("DNS: {}", dns)),
                            GeneralName::IPAddress(ip) => {
                                let ip_str = match ip.len() {
                                    4 => format!("IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]),
                                    16 => {
                                        let mut ipv6_parts = Vec::new();
                                        for chunk in ip.chunks(2) {
                                            ipv6_parts.push(
                                                format!(
                                                    "{:02x}{:02x}",
                                                    chunk[0],
                                                    chunk.get(1).unwrap_or(&0)
                                                )
                                            );
                                        }
                                        format!("IP: {}", ipv6_parts.join(":"))
                                    }
                                    _ => format!("IP: {:?}", ip),
                                };
                                san_list.push(ip_str);
                            }
                            GeneralName::RFC822Name(email) =>
                                san_list.push(format!("Email: {}", email)),
                            GeneralName::URI(uri) => san_list.push(format!("URI: {}", uri)),
                            _ => {}
                        }
                    }
                }
                break;
            }
        }

        // Key Usage
        let mut key_usage = Vec::new();
        for ext in cert.extensions() {
            if ext.oid == x509_parser::oid_registry::OID_X509_EXT_KEY_USAGE {
                if let Ok((_, ku)) = KeyUsage::from_der(ext.value) {
                    if ku.digital_signature() {
                        key_usage.push("Digital Signature".to_string());
                    }
                    if ku.non_repudiation() {
                        key_usage.push("Non Repudiation".to_string());
                    }
                    if ku.key_encipherment() {
                        key_usage.push("Key Encipherment".to_string());
                    }
                    if ku.data_encipherment() {
                        key_usage.push("Data Encipherment".to_string());
                    }
                    if ku.key_agreement() {
                        key_usage.push("Key Agreement".to_string());
                    }
                    if ku.key_cert_sign() {
                        key_usage.push("Key Cert Sign".to_string());
                    }
                    if ku.crl_sign() {
                        key_usage.push("CRL Sign".to_string());
                    }
                    if ku.encipher_only() {
                        key_usage.push("Encipher Only".to_string());
                    }
                    if ku.decipher_only() {
                        key_usage.push("Decipher Only".to_string());
                    }
                }
                break;
            }
        }

        // Extended Key Usage
        let extended_key_usage = Vec::new(); // Simplified for now

        // Generate fingerprints
        let fingerprint_sha1 = self.generate_fingerprint(cert_der, "SHA1")?;
        let fingerprint_sha256 = self.generate_fingerprint(cert_der, "SHA256")?;

        // Check if CA certificate - simplified approach
        let is_ca = false; // Will be set to true if basic constraints extension indicates CA

        // Check if self-signed (simplified check)
        let is_self_signed = cert.subject() == cert.issuer();

        Ok(CertificateInfo {
            subject,
            issuer,
            serial_number,
            version,
            not_before,
            not_after,
            signature_algorithm,
            public_key_algorithm,
            subject_alternative_names: san_list,
            key_usage,
            extended_key_usage,
            fingerprint_sha1,
            fingerprint_sha256,
            is_ca,
            is_self_signed,
            chain_length,
        })
    }

    /// Format ASN.1 time to readable string
    fn format_asn1_time(&self, time: &ASN1Time) -> Result<String> {
        let timestamp = time.timestamp();

        // Convert timestamp to DateTime<Utc>
        let datetime = DateTime::from_timestamp(timestamp, 0).ok_or_else(||
            anyhow::anyhow!("Invalid timestamp: {}", timestamp)
        )?;

        // Format as readable string with both timestamp and UTC time
        Ok(format!("{} ({})", datetime.format("%Y-%m-%d %H:%M:%S UTC"), timestamp))
    }

    /// Generate certificate fingerprint
    fn generate_fingerprint(&self, cert_der: &[u8], algorithm: &str) -> Result<String> {
        match algorithm {
            "SHA1" => {
                let mut hasher = Sha1::new();
                hasher.update(cert_der);
                let result = hasher.finalize();
                Ok(
                    result
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(":")
                )
            }
            "SHA256" => {
                let mut hasher = Sha256::new();
                hasher.update(cert_der);
                let result = hasher.finalize();
                Ok(
                    result
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(":")
                )
            }
            _ => Err(anyhow::anyhow!("Unsupported hash algorithm: {}", algorithm)),
        }
    }

    /// Format certificate information for display
    fn format_certificate_info(&self, cert: &CertificateInfo, domain: &str, port: u16) -> String {
        let mut output = String::new();

        output.push_str(&format!("SSL Certificate Information for {}:{}\n", domain, port));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("Subject: {}\n", cert.subject));
        output.push_str(&format!("Issuer: {}\n", cert.issuer));
        output.push_str(&format!("Serial Number: {}\n", cert.serial_number));
        output.push_str(&format!("Version: {}\n", cert.version));
        output.push('\n');

        output.push_str("Validity Period:\n");
        output.push_str(&format!("  Not Before: {}\n", cert.not_before));
        output.push_str(&format!("  Not After: {}\n", cert.not_after));
        output.push('\n');

        output.push_str("Algorithms:\n");
        output.push_str(&format!("  Signature Algorithm: {}\n", cert.signature_algorithm));
        output.push_str(&format!("  Public Key Algorithm: {}\n", cert.public_key_algorithm));
        output.push('\n');

        if !cert.subject_alternative_names.is_empty() {
            output.push_str("Subject Alternative Names:\n");
            for san in &cert.subject_alternative_names {
                output.push_str(&format!("  {}\n", san));
            }
            output.push('\n');
        }

        if !cert.key_usage.is_empty() {
            output.push_str("Key Usage:\n");
            for usage in &cert.key_usage {
                output.push_str(&format!("  {}\n", usage));
            }
            output.push('\n');
        }

        output.push_str("Certificate Properties:\n");
        output.push_str(&format!("  Is CA Certificate: {}\n", cert.is_ca));
        output.push_str(&format!("  Is Self-Signed: {}\n", cert.is_self_signed));
        output.push_str(&format!("  Certificate Chain Length: {}\n", cert.chain_length));
        output.push('\n');

        output.push_str("Fingerprints:\n");
        output.push_str(&format!("  SHA1: {}\n", cert.fingerprint_sha1));
        output.push_str(&format!("  SHA256: {}\n", cert.fingerprint_sha256));

        output
    }

    /// Check if a query string is a valid domain for SSL lookup
    pub fn is_ssl_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-SSL")
    }

    /// Parse SSL query to extract domain and optional port
    pub fn parse_ssl_query(query: &str) -> Option<(String, Option<u16>)> {
        if !Self::is_ssl_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 4]; // Remove "-SSL"

        // Check for port specification
        if let Some(colon_pos) = clean_query.rfind(':') {
            let domain = clean_query[..colon_pos].to_string();
            if let Ok(port) = clean_query[colon_pos + 1..].parse::<u16>() {
                return Some((domain, Some(port)));
            }
        }

        Some((clean_query.to_string(), None))
    }
}

/// Custom certificate verifier that accepts all certificates
/// This is needed to analyze certificates that might be invalid/expired
struct AcceptAllVerifier;

impl rustls::client::ServerCertVerifier for AcceptAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

/// Process SSL certificate query with -SSL suffix
pub async fn process_ssl_query(query: &str) -> Result<String> {
    let ssl_service = SslService::new();

    if let Some((domain, port)) = SslService::parse_ssl_query(query) {
        debug!("Processing SSL query for domain: {}, port: {:?}", domain, port);
        return ssl_service.query_ssl_certificate(&domain, port).await;
    }

    error!("Invalid SSL query format: {}", query);
    Ok(format!("Invalid SSL query format. Use: domain-SSL or domain:port-SSL\nQuery: {}\n", query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_query_detection() {
        assert!(SslService::is_ssl_query("example.com-SSL"));
        assert!(SslService::is_ssl_query("example.com-ssl"));
        assert!(SslService::is_ssl_query("sub.example.com:8443-SSL"));

        assert!(!SslService::is_ssl_query("example.com"));
        assert!(!SslService::is_ssl_query("example.com-GEO"));
        assert!(!SslService::is_ssl_query("SSL-example.com"));
    }

    #[test]
    fn test_ssl_query_parsing() {
        assert_eq!(
            SslService::parse_ssl_query("example.com-SSL"),
            Some(("example.com".to_string(), None))
        );

        assert_eq!(
            SslService::parse_ssl_query("example.com:8443-SSL"),
            Some(("example.com".to_string(), Some(8443)))
        );

        assert_eq!(
            SslService::parse_ssl_query("sub.domain.com:443-SSL"),
            Some(("sub.domain.com".to_string(), Some(443)))
        );

        assert_eq!(SslService::parse_ssl_query("example.com"), None);
    }

    #[tokio::test]
    async fn test_ssl_service_creation() {
        let service = SslService::new();
        assert_eq!(service.timeout, Duration::from_secs(10));

        let custom_service = SslService::with_timeout(Duration::from_secs(5));
        assert_eq!(custom_service.timeout, Duration::from_secs(5));
    }
}
