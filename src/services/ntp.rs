// WHOIS Server - NTP Time Synchronization Test Service
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! NTP time synchronization test service
//!
//! Connects to NTP servers and retrieves time information for testing purposes.
//! Does not actually synchronize the system clock.

use anyhow::Result;
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// NTP packet structure (48 bytes)
#[repr(C)]
struct NtpPacket {
    li_vn_mode: u8,       // Leap Indicator, Version Number, Mode
    stratum: u8,          // Stratum level
    poll: i8,             // Maximum poll interval
    precision: i8,        // Precision of the system clock
    root_delay: u32,      // Total round-trip delay
    root_dispersion: u32, // Maximum error
    ref_id: u32,          // Reference identifier
    ref_timestamp: u64,   // Reference timestamp
    orig_timestamp: u64,  // Origin timestamp
    rx_timestamp: u64,    // Receive timestamp
    tx_timestamp: u64,    // Transmit timestamp
}

impl NtpPacket {
    fn new() -> Self {
        NtpPacket {
            li_vn_mode: 0x1B, // LI=0 (no warning), VN=3 (NTPv3), Mode=3 (client)
            stratum: 0,
            poll: 0,
            precision: 0,
            root_delay: 0,
            root_dispersion: 0,
            ref_id: 0,
            ref_timestamp: 0,
            orig_timestamp: 0,
            rx_timestamp: 0,
            tx_timestamp: 0,
        }
    }

    fn to_bytes(&self) -> [u8; 48] {
        let mut bytes = [0u8; 48];
        bytes[0] = self.li_vn_mode;
        bytes[1] = self.stratum;
        bytes[2] = self.poll as u8;
        bytes[3] = self.precision as u8;

        // Convert u32/u64 fields to network byte order (big-endian)
        bytes[4..8].copy_from_slice(&self.root_delay.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.root_dispersion.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.ref_id.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.ref_timestamp.to_be_bytes());
        bytes[24..32].copy_from_slice(&self.orig_timestamp.to_be_bytes());
        bytes[32..40].copy_from_slice(&self.rx_timestamp.to_be_bytes());
        bytes[40..48].copy_from_slice(&self.tx_timestamp.to_be_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 48 {
            return None;
        }

        Some(NtpPacket {
            li_vn_mode: bytes[0],
            stratum: bytes[1],
            poll: bytes[2] as i8,
            precision: bytes[3] as i8,
            root_delay: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            root_dispersion: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            ref_id: u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            ref_timestamp: u64::from_be_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
            orig_timestamp: u64::from_be_bytes([
                bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30],
                bytes[31],
            ]),
            rx_timestamp: u64::from_be_bytes([
                bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38],
                bytes[39],
            ]),
            tx_timestamp: u64::from_be_bytes([
                bytes[40], bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46],
                bytes[47],
            ]),
        })
    }
}

/// Convert NTP timestamp to Unix timestamp with microsecond precision
fn ntp_to_unix_micros(ntp_timestamp: u64) -> i64 {
    const NTP_EPOCH_OFFSET: u64 = 2208988800; // Seconds between 1900 and 1970
    let seconds = (ntp_timestamp >> 32) as i64;
    let fraction = (ntp_timestamp & 0xFFFFFFFF) as f64;
    let micros = (fraction / 4294967296.0 * 1_000_000.0) as i64;
    (seconds - NTP_EPOCH_OFFSET as i64) * 1_000_000 + micros
}

/// Format timestamp as human-readable string
fn format_timestamp(unix_timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    let datetime = DateTime::<Utc>::from_timestamp(unix_timestamp, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Query NTP server and return time information
pub fn query_ntp_server(server: &str) -> Result<String> {
    debug!("Querying NTP server: {}", server);

    // Resolve server address (default to port 123)
    let addr = if server.contains(':') {
        server
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve NTP server address"))?
    } else {
        format!("{}:123", server)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve NTP server address"))?
    };

    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;

    // Record local time before sending request (microseconds)
    let t1 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as i64;

    // Create and send NTP request
    let request = NtpPacket::new();
    let request_bytes = request.to_bytes();
    socket.send_to(&request_bytes, addr)?;

    debug!("Sent NTP request to {}", addr);

    // Receive response
    let mut response_bytes = [0u8; 48];
    let (size, from) = socket.recv_from(&mut response_bytes)?;

    // Record local time after receiving response (microseconds)
    let t4 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as i64;

    if size < 48 {
        return Err(anyhow::anyhow!("Invalid NTP response size"));
    }

    debug!("Received NTP response from {}", from);

    // Parse response
    let response = NtpPacket::from_bytes(&response_bytes)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse NTP response"))?;

    // Extract timestamps (in microseconds)
    let t2 = ntp_to_unix_micros(response.rx_timestamp); // Server receive time
    let t3 = ntp_to_unix_micros(response.tx_timestamp); // Server transmit time

    // Calculate offset and delay (in microseconds)
    // Offset: ((T2 - T1) + (T3 - T4)) / 2
    // Delay: (T4 - T1) - (T3 - T2)
    let offset_micros = ((t2 - t1) + (t3 - t4)) / 2;
    let delay_micros = (t4 - t1) - (t3 - t2);

    // Convert to milliseconds for display
    let offset_ms = offset_micros as f64 / 1000.0;
    let delay_ms = delay_micros as f64 / 1000.0;

    // Get stratum description
    let stratum_desc = match response.stratum {
        0 => "Unspecified or invalid",
        1 => "Primary reference (e.g., GPS, atomic clock)",
        2..=15 => "Secondary reference (via NTP)",
        16..=255 => "Reserved",
    };

    // Format output
    let mut output = String::new();
    output.push_str("% NTP Time Synchronization Test\n");
    output.push_str(&format!("% Server: {}\n", server));
    output.push_str(&format!("% Resolved to: {}\n", addr));
    output.push_str("%\n");
    output.push_str("% Server Information:\n");
    output.push_str(&format!(
        "stratum:         {} ({})\n",
        response.stratum, stratum_desc
    ));
    output.push_str(&format!(
        "precision:       2^{} seconds\n",
        response.precision
    ));
    output.push_str(&format!(
        "root-delay:      {} ms\n",
        (response.root_delay as f64 / 65536.0 * 1000.0) as u32
    ));
    output.push_str(&format!(
        "root-dispersion: {} ms\n",
        (response.root_dispersion as f64 / 65536.0 * 1000.0) as u32
    ));
    output.push_str("%\n");
    output.push_str("% Time Information:\n");
    output.push_str(&format!(
        "server-time:     {}\n",
        format_timestamp(t3 / 1_000_000)
    ));
    output.push_str(&format!(
        "local-time:      {}\n",
        format_timestamp(t4 / 1_000_000)
    ));
    output.push_str("%\n");
    output.push_str("% Synchronization Metrics:\n");
    output.push_str(&format!(
        "offset:          {:.3} ms ({:.6} seconds)\n",
        offset_ms,
        offset_ms / 1000.0
    ));
    output.push_str(&format!("round-trip:      {:.3} ms\n", delay_ms));
    output.push_str("%\n");

    if offset_ms.abs() > 1000.0 {
        output.push_str(&format!(
            "% ⚠ WARNING: Clock offset is {:.3} seconds\n",
            offset_ms / 1000.0
        ));
        output.push_str("% Your local clock may need adjustment\n");
    } else if offset_ms.abs() > 100.0 {
        output.push_str(&format!(
            "% ⚠ Clock offset is significant: {:.1}ms\n",
            offset_ms
        ));
    } else if offset_ms.abs() > 10.0 {
        output.push_str(&format!(
            "% ✓ Clock is synchronized (offset: {:.1}ms)\n",
            offset_ms
        ));
    } else {
        output.push_str(&format!(
            "% ✓ Excellent synchronization! (offset: {:.2}ms)\n",
            offset_ms
        ));
    }

    output.push_str("%\n");
    output.push_str("% Note: This is a test query only. System time was not modified.\n");

    Ok(output)
}

/// Handle NTP query
pub async fn handle_ntp_query(server: &str) -> Result<String> {
    if server.is_empty() {
        return Ok("% NTP Time Synchronization Test\n\
             % Error: No server specified\n\
             %\n\
             % Usage: <server>-NTP\n\
             %\n\
             % Examples:\n\
             %   pool.ntp.org-NTP\n\
             %   time.google.com-NTP\n\
             %   time.cloudflare.com-NTP\n\
             %   ntp.aliyun.com-NTP\n\
             %   cn.pool.ntp.org-NTP\n\
             %\n\
             % Run 'whois help' for more information\n"
            .to_string());
    }

    match query_ntp_server(server) {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!("NTP query failed for {}: {}", server, e);
            Ok(format!(
                "% NTP Time Synchronization Test\n\
                 % Server: {}\n\
                 % Error: {}\n\
                 %\n\
                 % Possible reasons:\n\
                 % - Server is unreachable\n\
                 % - Firewall blocking UDP port 123\n\
                 % - Invalid server address\n\
                 % - Server is not responding\n\
                 %\n\
                 % Try these public NTP servers:\n\
                 %   pool.ntp.org\n\
                 %   time.google.com\n\
                 %   time.cloudflare.com\n\
                 %   ntp.aliyun.com\n\
                 %   cn.pool.ntp.org\n",
                server, e
            ))
        }
    }
}
