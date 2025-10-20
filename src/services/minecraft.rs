use std::net::{ SocketAddr, ToSocketAddrs };
use std::time::{ Duration, Instant };
use anyhow::Result;
use tracing::{ debug, error };
use serde::{ Deserialize, Serialize };
use tokio::net::TcpStream;
use tokio::io::{ AsyncReadExt, AsyncWriteExt };

/// Minecraft server status response structure
#[derive(Debug, Deserialize, Serialize)]
struct MinecraftStatus {
    version: MinecraftVersion,
    players: MinecraftPlayers,
    description: serde_json::Value,
    favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    enforces_secure_chat: Option<bool>,
    #[serde(rename = "previewsChat")]
    previews_chat: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MinecraftVersion {
    name: String,
    protocol: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct MinecraftPlayers {
    max: i32,
    online: i32,
    sample: Option<Vec<MinecraftPlayer>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MinecraftPlayer {
    name: String,
    id: String,
}

/// Minecraft server information for display
#[derive(Debug, Clone)]
struct MinecraftServerInfo {
    address: String,
    port: u16,
    online: bool,
    version: String,
    protocol: i32,
    players_online: i32,
    players_max: i32,
    player_list: Vec<String>,
    description: String,
    latency: u64,
    enforces_secure_chat: Option<bool>,
    previews_chat: Option<bool>,
}

/// Minecraft server query service
pub struct MinecraftService {
    timeout: Duration,
}

impl Default for MinecraftService {
    fn default() -> Self {
        Self::new()
    }
}

impl MinecraftService {
    /// Create a new Minecraft service with default 10-second timeout
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }

    /// Create Minecraft service with custom timeout
    #[allow(dead_code)]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Query Minecraft server status
    pub async fn query_minecraft(&self, target: &str) -> Result<String> {
        debug!("Querying Minecraft server: {}", target);

        let (host, port) = self.parse_minecraft_target(target)?;

        match self.get_server_status(&host, port).await {
            Ok(server_info) => {
                let output = self.format_server_info(&server_info);
                debug!(
                    "Minecraft query completed for {}:{}, latency: {}ms",
                    host,
                    port,
                    server_info.latency
                );
                Ok(output)
            }
            Err(e) => {
                error!("Failed to query Minecraft server {}:{}: {}", host, port, e);
                Ok(
                    format!(
                        "Minecraft Server Query Failed for {}:{}\nError: {}\n\nPossible causes:\n- Server is offline or unreachable\n- Server is not running Minecraft\n- Firewall blocking connection\n- Invalid hostname or port\n",
                        host,
                        port,
                        e
                    )
                )
            }
        }
    }

    /// Parse Minecraft target (host:port or just host)
    fn parse_minecraft_target(&self, target: &str) -> Result<(String, u16)> {
        if let Some(colon_pos) = target.rfind(':') {
            let host = target[..colon_pos].to_string();
            let port_str = &target[colon_pos + 1..];

            let port = port_str
                .parse::<u16>()
                .map_err(|_| anyhow::anyhow!("Invalid port number: {}", port_str))?;

            if host.is_empty() {
                return Err(anyhow::anyhow!("Empty hostname"));
            }

            Ok((host, port))
        } else {
            // Default Minecraft port
            Ok((target.to_string(), 25565))
        }
    }

    /// Get server status using Minecraft Server List Ping protocol
    async fn get_server_status(&self, host: &str, port: u16) -> Result<MinecraftServerInfo> {
        let start_time = Instant::now();

        // Resolve hostname to IP address
        let socket_addr = self.resolve_address(host, port).await?;
        debug!("Resolved {}:{} to {}", host, port, socket_addr);

        // Connect to server with timeout
        let mut stream = tokio::time
            ::timeout(self.timeout, TcpStream::connect(socket_addr)).await
            .map_err(|_|
                anyhow::anyhow!("Connection timeout after {} seconds", self.timeout.as_secs())
            )?
            .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

        // Send handshake packet
        self.send_handshake(&mut stream, host, port).await?;

        // Send status request
        self.send_status_request(&mut stream).await?;

        // Read status response
        let status_json = self.read_status_response(&mut stream).await?;

        // Parse JSON response
        let status: MinecraftStatus = serde_json
            ::from_str(&status_json)
            .map_err(|e| anyhow::anyhow!("Failed to parse server response: {}", e))?;

        // Send ping request for latency measurement
        let ping_start = Instant::now();
        self.send_ping(&mut stream).await?;
        self.read_ping_response(&mut stream).await?;
        let ping_latency = ping_start.elapsed().as_millis() as u64;

        let total_latency = start_time.elapsed().as_millis() as u64;

        // Extract player list
        let player_list = status.players.sample
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.name)
            .collect();

        // Format description (can be string or object)
        let description = self.format_description(&status.description);

        Ok(MinecraftServerInfo {
            address: host.to_string(),
            port,
            online: true,
            version: status.version.name,
            protocol: status.version.protocol,
            players_online: status.players.online,
            players_max: status.players.max,
            player_list,
            description,
            latency: std::cmp::min(total_latency, ping_latency),
            enforces_secure_chat: status.enforces_secure_chat,
            previews_chat: status.previews_chat,
        })
    }

    /// Resolve hostname to socket address
    async fn resolve_address(&self, host: &str, port: u16) -> Result<SocketAddr> {
        let addr_str = format!("{}:{}", host, port);

        // Try to resolve the address
        let mut addrs = tokio::task
            ::spawn_blocking(move || { addr_str.to_socket_addrs() }).await
            .map_err(|e| anyhow::anyhow!("DNS resolution task failed: {}", e))?
            .map_err(|e| anyhow::anyhow!("Failed to resolve hostname '{}': {}", host, e))?;

        addrs.next().ok_or_else(|| anyhow::anyhow!("No addresses found for hostname: {}", host))
    }

    /// Send handshake packet (Protocol state: Status)
    async fn send_handshake(&self, stream: &mut TcpStream, host: &str, port: u16) -> Result<()> {
        let mut packet = Vec::new();

        // Packet ID: 0x00 (Handshake)
        packet.push(0x00);

        // Protocol version (use 760 for 1.19.2, widely supported)
        self.write_varint(&mut packet, 760);

        // Server address
        self.write_string(&mut packet, host);

        // Server port
        packet.extend_from_slice(&port.to_be_bytes());

        // Next state: 1 (Status)
        self.write_varint(&mut packet, 1);

        // Send packet with length prefix
        self.send_packet(stream, &packet).await
    }

    /// Send status request packet
    async fn send_status_request(&self, stream: &mut TcpStream) -> Result<()> {
        let packet = vec![0x00]; // Packet ID: 0x00 (Status Request)
        self.send_packet(stream, &packet).await
    }

    /// Read status response packet
    async fn read_status_response(&self, stream: &mut TcpStream) -> Result<String> {
        let packet = self.read_packet(stream).await?;

        if packet.is_empty() || packet[0] != 0x00 {
            return Err(anyhow::anyhow!("Invalid status response packet"));
        }

        // Skip packet ID and read JSON string
        let json_data = &packet[1..];
        let (json_string, _) = self.read_string_from_bytes(json_data)?;

        Ok(json_string)
    }

    /// Send ping packet
    async fn send_ping(&self, stream: &mut TcpStream) -> Result<()> {
        let mut packet = Vec::new();
        packet.push(0x01); // Packet ID: 0x01 (Ping)

        // Add payload (current timestamp)
        let timestamp = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        packet.extend_from_slice(&timestamp.to_be_bytes());

        self.send_packet(stream, &packet).await
    }

    /// Read ping response packet
    async fn read_ping_response(&self, stream: &mut TcpStream) -> Result<()> {
        let packet = self.read_packet(stream).await?;

        if packet.is_empty() || packet[0] != 0x01 {
            return Err(anyhow::anyhow!("Invalid ping response packet"));
        }

        Ok(())
    }

    /// Send packet with length prefix
    async fn send_packet(&self, stream: &mut TcpStream, data: &[u8]) -> Result<()> {
        let mut packet = Vec::new();
        self.write_varint(&mut packet, data.len() as i32);
        packet.extend_from_slice(data);

        stream.write_all(&packet).await.map_err(|e| anyhow::anyhow!("Failed to send packet: {}", e))
    }

    /// Read packet with length prefix
    async fn read_packet(&self, stream: &mut TcpStream) -> Result<Vec<u8>> {
        // Read packet length
        let length = self.read_varint(stream).await? as usize;

        if length == 0 {
            return Ok(Vec::new());
        }

        if length > 1048576 {
            // 1MB limit
            return Err(anyhow::anyhow!("Packet too large: {} bytes", length));
        }

        // Read packet data
        let mut buffer = vec![0u8; length];
        stream
            .read_exact(&mut buffer).await
            .map_err(|e| anyhow::anyhow!("Failed to read packet data: {}", e))?;

        Ok(buffer)
    }

    /// Write VarInt to buffer
    fn write_varint(&self, buffer: &mut Vec<u8>, mut value: i32) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buffer.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// Read VarInt from stream
    async fn read_varint(&self, stream: &mut TcpStream) -> Result<i32> {
        let mut result = 0i32;
        let mut position = 0;

        loop {
            let mut byte = [0u8; 1];
            stream
                .read_exact(&mut byte).await
                .map_err(|e| anyhow::anyhow!("Failed to read varint byte: {}", e))?;

            let byte = byte[0];
            result |= ((byte & 0x7f) as i32) << position;

            if (byte & 0x80) == 0 {
                break;
            }

            position += 7;
            if position >= 32 {
                return Err(anyhow::anyhow!("VarInt too big"));
            }
        }

        Ok(result)
    }

    /// Write string to buffer
    fn write_string(&self, buffer: &mut Vec<u8>, string: &str) {
        let bytes = string.as_bytes();
        self.write_varint(buffer, bytes.len() as i32);
        buffer.extend_from_slice(bytes);
    }

    /// Read string from byte array
    fn read_string_from_bytes(&self, data: &[u8]) -> Result<(String, usize)> {
        if data.is_empty() {
            return Err(anyhow::anyhow!("Empty data for string reading"));
        }

        let mut offset = 0;
        let length = self.read_varint_from_bytes(data, &mut offset)? as usize;

        if offset + length > data.len() {
            return Err(anyhow::anyhow!("String length exceeds available data"));
        }

        let string_data = &data[offset..offset + length];
        let string = String::from_utf8_lossy(string_data).into_owned();

        Ok((string, offset + length))
    }

    /// Read VarInt from byte array
    fn read_varint_from_bytes(&self, data: &[u8], offset: &mut usize) -> Result<i32> {
        let mut result = 0i32;
        let mut position = 0;

        loop {
            if *offset >= data.len() {
                return Err(anyhow::anyhow!("Unexpected end of data while reading varint"));
            }

            let byte = data[*offset];
            *offset += 1;

            result |= ((byte & 0x7f) as i32) << position;

            if (byte & 0x80) == 0 {
                break;
            }

            position += 7;
            if position >= 32 {
                return Err(anyhow::anyhow!("VarInt too big"));
            }
        }

        Ok(result)
    }

    /// Format description from JSON value
    fn format_description(&self, description: &serde_json::Value) -> String {
        match description {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(obj) => {
                // Try to extract text from various formats
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    text.to_string()
                } else {
                    // Fallback to serialized JSON
                    description.to_string()
                }
            }
            _ => description.to_string(),
        }
    }

    /// Format server information for display in RIPE-style format
    fn format_server_info(&self, info: &MinecraftServerInfo) -> String {
        let mut output = String::new();

        // RIPE-style header
        output.push_str("% This is the WHOIS server response for Minecraft server query\n");
        output.push_str("% Information related to Minecraft server status\n");
        output.push_str("%\n");
        output.push_str("% The objects are in RPSL format\n");
        output.push_str("%\n");

        // Server object in RIPE-style
        output.push_str("server:         ");
        output.push_str(&format!("{}:{}\n", info.address, info.port));

        output.push_str("status:         ");
        output.push_str(&format!("{}\n", if info.online { "ONLINE" } else { "OFFLINE" }));

        output.push_str("version:        ");
        output.push_str(&format!("{}\n", info.version));

        output.push_str("protocol:       ");
        output.push_str(&format!("{}\n", info.protocol));

        output.push_str("descr:          ");
        output.push_str(&format!("{}\n", info.description));

        output.push_str("players-online: ");
        output.push_str(&format!("{}\n", info.players_online));

        output.push_str("players-max:    ");
        output.push_str(&format!("{}\n", info.players_max));

        output.push_str("latency:        ");
        output.push_str(&format!("{}ms\n", info.latency));

        if let Some(secure_chat) = info.enforces_secure_chat {
            output.push_str("secure-chat:    ");
            output.push_str(&format!("{}\n", if secure_chat { "enforced" } else { "optional" }));
        }

        if let Some(preview_chat) = info.previews_chat {
            output.push_str("chat-preview:   ");
            output.push_str(&format!("{}\n", if preview_chat { "enabled" } else { "disabled" }));
        }

        // Player list in RIPE-style (if available)
        if !info.player_list.is_empty() {
            for (i, player) in info.player_list.iter().enumerate() {
                if i >= 10 {
                    output.push_str("remarks:        ");
                    output.push_str(
                        &format!("... and {} more players online\n", info.player_list.len() - 10)
                    );
                    break;
                }
                output.push_str("player:         ");
                output.push_str(&format!("{}\n", player));
            }
        } else if info.players_online > 0 {
            output.push_str("remarks:        ");
            output.push_str("Player list hidden by server configuration\n");
        }

        // Source information
        output.push_str("source:         AKAERE-NETWORKS-AGENT\n");

        output.push('\n');
        output.push_str("% Information retrieved using Minecraft Server List Ping protocol\n");
        output.push_str("% Query processed by WHOIS server\n");

        output
    }

    /// Check if a query string is a Minecraft query
    #[allow(dead_code)]
    pub fn is_minecraft_query(query: &str) -> bool {
        let upper_query = query.to_uppercase();
        upper_query.ends_with("-MINECRAFT") || upper_query.ends_with("-MC")
    }

    /// Parse Minecraft query to extract target
    pub fn parse_minecraft_query(query: &str) -> Option<String> {
        let upper_query = query.to_uppercase();

        if upper_query.ends_with("-MINECRAFT") {
            Some(query[..query.len() - 10].to_string())
        } else if upper_query.ends_with("-MC") {
            Some(query[..query.len() - 3].to_string())
        } else {
            None
        }
    }
}

/// Process Minecraft server query with -MINECRAFT or -MC suffix
pub async fn process_minecraft_query(query: &str) -> Result<String> {
    let minecraft_service = MinecraftService::new();

    if let Some(target) = MinecraftService::parse_minecraft_query(query) {
        debug!("Processing Minecraft query for target: {}", target);
        return minecraft_service.query_minecraft(&target).await;
    }

    error!("Invalid Minecraft query format: {}", query);
    Ok(
        format!("Invalid Minecraft query format. Use: target-MINECRAFT or target-MC\nTarget format: hostname:port or hostname (default port 25565)\nQuery: {}\nExamples:\n  - mc.hypixel.net-MC\n  - play.cubecraft.net:25565-MINECRAFT\n  - 192.168.1.100-MC\n", query)
    )
}

/// Minecraft user profile information from Mojang API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftUserProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub properties: Vec<MinecraftUserProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftUserProperty {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub signature: Option<String>,
}

/// Minecraft UUID response from Mojang API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftUuidResponse {
    pub id: String,
    pub name: String,
}

/// Minecraft user service for querying player information
pub struct MinecraftUserService {
    client: reqwest::Client,
}

impl Default for MinecraftUserService {
    fn default() -> Self {
        Self::new()
    }
}

impl MinecraftUserService {
    /// Create a new Minecraft user service
    pub fn new() -> Self {
        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("WhoisServer/1.0 Minecraft User API Client")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Query Minecraft user information by username
    pub async fn query_user_info(&self, username: &str) -> Result<String> {
        debug!("Querying Minecraft user info for: {}", username);

        // First, get UUID from username
        let uuid = match self.get_uuid_from_username(username).await {
            Ok(uuid) => uuid,
            Err(e) => {
                return Ok(format!("Minecraft User Not Found: {}\nError: {}\n", username, e));
            }
        };

        // Then get full profile information
        match self.get_user_profile(&uuid.id).await {
            Ok(profile) => Ok(self.format_user_info(&uuid, &profile)),
            Err(e) => {
                // Return basic info even if profile fetch fails
                Ok(self.format_basic_user_info(&uuid, &format!("Profile fetch failed: {}", e)))
            }
        }
    }

    /// Get UUID from username using Mojang API
    async fn get_uuid_from_username(&self, username: &str) -> Result<MinecraftUuidResponse> {
        let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", username);

        let response = self.client.get(&url).send().await?;

        if response.status() == 204 {
            return Err(anyhow::anyhow!("Player not found"));
        }

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API request failed: {}", response.status()));
        }

        let uuid_response: MinecraftUuidResponse = response.json().await?;
        Ok(uuid_response)
    }

    /// Get user profile from UUID using Mojang API
    async fn get_user_profile(&self, uuid: &str) -> Result<MinecraftUserProfile> {
        let url = format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", uuid);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Profile request failed: {}", response.status()));
        }

        let profile: MinecraftUserProfile = response.json().await?;
        Ok(profile)
    }

    /// Format user information for WHOIS display
    fn format_user_info(
        &self,
        uuid_info: &MinecraftUuidResponse,
        profile: &MinecraftUserProfile
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!("Minecraft User Information: {}\n", uuid_info.name));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("username: {}\n", uuid_info.name));
        output.push_str(&format!("uuid: {}\n", self.format_uuid(&uuid_info.id)));
        output.push_str(&format!("uuid-short: {}\n", uuid_info.id));

        // Add profile properties
        for property in &profile.properties {
            match property.name.as_str() {
                "textures" => {
                    output.push_str(&format!("has-skin: {}\n", "yes"));
                    if property.signature.is_some() {
                        output.push_str(&format!("skin-signed: {}\n", "yes"));
                    }
                }
                _ => {
                    output.push_str(
                        &format!("property-{}: {}\n", property.name.to_lowercase(), "present")
                    );
                }
            }
        }

        // Add useful URLs
        output.push_str(&format!("namemc-url: https://namemc.com/profile/{}\n", uuid_info.id));
        output.push_str(&format!("skin-url: https://crafatar.com/skins/{}\n", uuid_info.id));
        output.push_str(&format!("avatar-url: https://crafatar.com/avatars/{}\n", uuid_info.id));

        output.push_str("% Note: Player information is public data from Mojang API\n");

        output
    }

    /// Format basic user information when profile fetch fails
    fn format_basic_user_info(&self, uuid_info: &MinecraftUuidResponse, error: &str) -> String {
        let mut output = String::new();

        output.push_str(&format!("Minecraft User Information: {}\n", uuid_info.name));
        output.push_str("=".repeat(60).as_str());
        output.push('\n');

        output.push_str(&format!("username: {}\n", uuid_info.name));
        output.push_str(&format!("uuid: {}\n", self.format_uuid(&uuid_info.id)));
        output.push_str(&format!("uuid-short: {}\n", uuid_info.id));
        output.push_str(&format!("profile-status: {}\n", error));

        output.push_str(&format!("namemc-url: https://namemc.com/profile/{}\n", uuid_info.id));
        output.push_str(&format!("skin-url: https://crafatar.com/skins/{}\n", uuid_info.id));
        output.push_str(&format!("avatar-url: https://crafatar.com/avatars/{}\n", uuid_info.id));

        output
    }

    /// Format UUID with dashes
    fn format_uuid(&self, uuid: &str) -> String {
        if uuid.len() == 32 {
            format!(
                "{}-{}-{}-{}-{}",
                &uuid[0..8],
                &uuid[8..12],
                &uuid[12..16],
                &uuid[16..20],
                &uuid[20..32]
            )
        } else {
            uuid.to_string()
        }
    }

    /// Check if a query string is a Minecraft user query
    pub fn is_minecraft_user_query(query: &str) -> bool {
        query.to_uppercase().ends_with("-MCU")
    }

    /// Parse Minecraft user query
    pub fn parse_minecraft_user_query(query: &str) -> Option<String> {
        if !Self::is_minecraft_user_query(query) {
            return None;
        }

        let clean_query = &query[..query.len() - 4]; // Remove "-MCU"
        Some(clean_query.to_string())
    }
}

/// Process Minecraft user query with -MCU suffix
pub async fn process_minecraft_user_query(query: &str) -> Result<String> {
    let user_service = MinecraftUserService::new();

    if let Some(username) = MinecraftUserService::parse_minecraft_user_query(query) {
        debug!("Processing Minecraft user query for: {}", username);

        if username.is_empty() {
            return Ok(
                "Invalid Minecraft user query. Please provide a username.\nExample: Notch-MCU\n".to_string()
            );
        }

        // Validate username format (Minecraft usernames are 3-16 characters, alphanumeric and underscores)
        if
            username.len() < 3 ||
            username.len() > 16 ||
            !username.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            return Ok(
                format!("Invalid Minecraft username format. Usernames must be 3-16 characters, alphanumeric and underscores only.\nQuery: {}\n", username)
            );
        }

        user_service.query_user_info(&username).await
    } else {
        error!("Invalid Minecraft user query format: {}", query);
        Ok(
            format!("Invalid Minecraft user query format. Use: <username>-MCU\nExample: Notch-MCU\nQuery: {}\n", query)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minecraft_query_detection() {
        assert!(MinecraftService::is_minecraft_query("mc.hypixel.net-MINECRAFT"));
        assert!(MinecraftService::is_minecraft_query("mc.hypixel.net-MC"));
        assert!(MinecraftService::is_minecraft_query("play.cubecraft.net:25565-minecraft"));
        assert!(MinecraftService::is_minecraft_query("192.168.1.100-mc"));

        assert!(!MinecraftService::is_minecraft_query("mc.hypixel.net"));
        assert!(!MinecraftService::is_minecraft_query("mc.hypixel.net-SSL"));
        assert!(!MinecraftService::is_minecraft_query("MINECRAFT-mc.hypixel.net"));
    }

    #[test]
    fn test_minecraft_query_parsing() {
        assert_eq!(
            MinecraftService::parse_minecraft_query("mc.hypixel.net-MINECRAFT"),
            Some("mc.hypixel.net".to_string())
        );

        assert_eq!(
            MinecraftService::parse_minecraft_query("play.cubecraft.net:25565-MC"),
            Some("play.cubecraft.net:25565".to_string())
        );

        assert_eq!(
            MinecraftService::parse_minecraft_query("192.168.1.100-mc"),
            Some("192.168.1.100".to_string())
        );

        assert_eq!(MinecraftService::parse_minecraft_query("mc.hypixel.net"), None);
    }

    #[test]
    fn test_minecraft_target_parsing() {
        let service = MinecraftService::new();

        // Test hostname with port
        assert_eq!(service.parse_minecraft_target("mc.hypixel.net:25565").unwrap(), (
            "mc.hypixel.net".to_string(),
            25565,
        ));

        // Test hostname without port (should default to 25565)
        assert_eq!(service.parse_minecraft_target("mc.hypixel.net").unwrap(), (
            "mc.hypixel.net".to_string(),
            25565,
        ));

        // Test IP with port
        assert_eq!(service.parse_minecraft_target("192.168.1.100:25566").unwrap(), (
            "192.168.1.100".to_string(),
            25566,
        ));

        // Test invalid port
        assert!(service.parse_minecraft_target("mc.hypixel.net:invalid").is_err());

        // Test empty hostname
        assert!(service.parse_minecraft_target(":25565").is_err());
    }

    #[tokio::test]
    async fn test_minecraft_service_creation() {
        let service = MinecraftService::new();
        assert_eq!(service.timeout, Duration::from_secs(10));

        let custom_service = MinecraftService::with_timeout(Duration::from_secs(5));
        assert_eq!(custom_service.timeout, Duration::from_secs(5));
    }
}
