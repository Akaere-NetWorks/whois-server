// WHOIS Server - SSH Session Handler
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use chrono::{DateTime, Utc};
use russh::{Channel, ChannelId, CryptoVec, server};
use russh_keys::key;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use super::history::{SshConnectionHistory, SshConnectionRecord};
use crate::core::process_query;

/// ANSI escape sequence parsing state
#[derive(Debug, Clone, PartialEq)]
enum EscapeState {
    Normal,
    Escape,
    Csi,
}

/// SSH session data
#[derive(Debug, Clone)]
struct SshSession {
    start_time: DateTime<Utc>,
    queries_count: u32,
    username: Option<String>,
    current_line: String,
    cursor_pos: usize,
    command_history: Vec<String>,
    history_index: Option<usize>,
    escape_state: EscapeState,
    escape_buffer: Vec<u8>,
}

/// WHOIS SSH server handler
pub struct WhoisSshHandler {
    /// SSH connection history manager
    history: Arc<SshConnectionHistory>,
    /// Active sessions
    sessions: Arc<Mutex<HashMap<ChannelId, SshSession>>>,
    /// Client address
    client_addr: Option<SocketAddr>,
    /// Server host key
    #[allow(dead_code)]
    host_key: Arc<key::KeyPair>,
}

impl WhoisSshHandler {
    /// Create a new WHOIS SSH handler
    pub fn new(history: Arc<SshConnectionHistory>, host_key: Arc<key::KeyPair>) -> Self {
        Self {
            history,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_addr: None,
            host_key,
        }
    }

    /// Set the client address
    pub fn set_client_addr(&mut self, addr: SocketAddr) {
        self.client_addr = Some(addr);
    }

    /// Process a WHOIS query and return the response
    async fn process_whois_query(&self, query: &str) -> String {
        let query = query.trim();

        if query.is_empty() {
            return "Error: Empty query\r\n".to_string();
        }

        // Special handling for history command
        if query.eq_ignore_ascii_case("history") {
            return self.get_connection_history().await;
        }

        // Detect query type and process
        let query_type = crate::core::analyze_query(query);
        debug!(
            "Processing SSH WHOIS query: {} (type: {:?})",
            query, query_type
        );

        // Use the existing query handling logic from the main server
        match process_query(query, &query_type, None).await {
            Ok(response) => {
                // Add CRLF line endings for proper terminal display
                response.replace('\n', "\r\n") + "\r\n"
            }
            Err(e) => {
                error!("Error processing SSH WHOIS query '{}': {}", query, e);
                format!("Error: {}\r\n", e)
            }
        }
    }

    /// Get connection history for the current client IP
    async fn get_connection_history(&self) -> String {
        let client_ip = match self.client_addr {
            Some(addr) => addr.ip(),
            None => {
                return "Error: Unable to determine client IP\r\n".to_string();
            }
        };

        match self.history.get_history_for_ip(&client_ip) {
            Ok(records) => {
                if records.is_empty() {
                    "No connection history found for your IP address.\r\n".to_string()
                } else {
                    let mut response = format!(
                        "Connection history for {} ({} records):\r\n\r\n",
                        client_ip,
                        records.len()
                    );

                    for (i, record) in records.iter().enumerate() {
                        response.push_str(&format!(
                            "{}. {} - {} queries, {}s duration, reason: {}\r\n",
                            i + 1,
                            record.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                            record.queries_count,
                            record.session_duration_seconds,
                            record.disconnect_reason
                        ));
                    }

                    response.push_str("\r\n");
                    response
                }
            }
            Err(e) => {
                error!("Failed to retrieve connection history: {}", e);
                "Error: Failed to retrieve connection history\r\n".to_string()
            }
        }
    }
}

#[async_trait::async_trait]
impl server::Handler for WhoisSshHandler {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<server::Msg>,
        _session: &mut server::Session,
    ) -> Result<bool, Self::Error> {
        debug!("SSH channel opened: {:?}", channel.id());

        // Initialize session data
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            channel.id(),
            SshSession {
                start_time: Utc::now(),
                queries_count: 0,
                username: None,
                current_line: String::new(),
                cursor_pos: 0,
                command_history: Vec::new(),
                history_index: None,
                escape_state: EscapeState::Normal,
                escape_buffer: Vec::new(),
            },
        );

        Ok(true)
    }

    async fn auth_password(
        &mut self,
        user: &str,
        _password: &str,
    ) -> Result<server::Auth, Self::Error> {
        // Accept only "whois" username for SSH connections
        if user != "whois" {
            info!("SSH authentication failed: invalid username '{}'", user);
            return Ok(server::Auth::Reject {
                proceed_with_methods: None,
            });
        }

        info!("SSH authentication successful: user={}", user);

        // Store username for session tracking
        let mut sessions = self.sessions.lock().await;
        for session_data in sessions.values_mut() {
            session_data.username = Some(user.to_string());
        }

        Ok(server::Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _public_key: &key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        // Accept only "whois" username for SSH connections
        if user != "whois" {
            info!(
                "SSH public key authentication failed: invalid username '{}'",
                user
            );
            return Ok(server::Auth::Reject {
                proceed_with_methods: None,
            });
        }

        info!("SSH public key authentication successful: user={}", user);

        // Store username for session tracking
        let mut sessions = self.sessions.lock().await;
        for session_data in sessions.values_mut() {
            session_data.username = Some(user.to_string());
        }

        Ok(server::Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        for &byte in data {
            self.handle_byte(channel, byte, session).await?;
        }
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        debug!("SSH channel closed: {:?}", channel);

        // Record session in history
        if let Some(client_addr) = self.client_addr {
            let mut sessions = self.sessions.lock().await;
            if let Some(session_data) = sessions.remove(&channel) {
                let duration = Utc::now().signed_duration_since(session_data.start_time);

                let record = SshConnectionRecord {
                    timestamp: session_data.start_time,
                    ip_address: client_addr.ip(),
                    username: session_data.username,
                    queries_count: session_data.queries_count,
                    session_duration_seconds: duration.num_seconds().max(0) as u64,
                    disconnect_reason: "Channel closed".to_string(),
                };

                if let Err(e) = self.history.add_record(record) {
                    error!("Failed to record SSH session history: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        debug!("SSH channel EOF: {:?}", channel);
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        debug!("SSH PTY request for channel: {:?}", channel);
        // Accept PTY request
        session.request_success();
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        debug!("SSH shell request for channel: {:?}", channel);
        // Accept shell request and send welcome message
        session.request_success();

        let welcome_msg = "┌─────────────────────────────────────────────────────────────┐\r\n\
            │              Akaere NetWorks WHOIS SSH Server               │\r\n\
            │                     whois.akae.re                           │\r\n\
            └─────────────────────────────────────────────────────────────┘\r\n\
            \r\n\
            Welcome! Type WHOIS queries directly:\r\n\
            Examples: example.com, 8.8.8.8, AS15169, example.com-GEO\r\n\
            \r\n\
            Special commands:\r\n\
            • 'history'    - View your connection history\r\n\
            • 'help'       - Show all available query types\r\n\
            • 'clear/cls'  - Clear the screen\r\n\
            • 'exit/quit'  - Disconnect from server\r\n\
            \r\n\
            Controls:\r\n\
            • Ctrl+C       - Cancel current input\r\n\
            • Ctrl+D       - Exit when input is empty\r\n\
            • Arrow keys   - Navigate command history\r\n\
            \r\n\
            © 2025 Akaere Networks | Licensed under AGPL-3.0-or-later\r\n\
            \r\n\
            whois> "
            .to_string();

        session.data(channel, CryptoVec::from_slice(welcome_msg.as_bytes()));
        Ok(())
    }
}

impl WhoisSshHandler {
    async fn handle_byte(
        &mut self,
        channel: ChannelId,
        byte: u8,
        session: &mut server::Session,
    ) -> Result<(), anyhow::Error> {
        let mut sessions = self.sessions.lock().await;
        let session_data = match sessions.get_mut(&channel) {
            Some(s) => s,
            None => {
                return Ok(());
            }
        };

        // Handle escape sequences
        match session_data.escape_state {
            EscapeState::Normal => {
                match byte {
                    // Enter key - process command
                    b'\r' | b'\n' => {
                        let command = session_data.current_line.trim().to_string();
                        session.data(channel, CryptoVec::from_slice(b"\r\n"));

                        if !command.is_empty() {
                            // Check for exit commands
                            if command.eq_ignore_ascii_case("exit")
                                || command.eq_ignore_ascii_case("quit")
                                || command.eq_ignore_ascii_case("bye")
                            {
                                session.data(channel, CryptoVec::from_slice(b"Goodbye!\r\n"));
                                session.close(channel);
                                return Ok(());
                            }

                            // Check for clear command
                            if command.eq_ignore_ascii_case("clear")
                                || command.eq_ignore_ascii_case("cls")
                            {
                                // Clear screen using ANSI escape sequences
                                session.data(channel, CryptoVec::from_slice(b"\x1B[2J\x1B[H"));

                                // Reset session state
                                session_data.current_line.clear();
                                session_data.cursor_pos = 0;
                                session_data.history_index = None;

                                session.data(channel, CryptoVec::from_slice(b"whois> "));
                                return Ok(());
                            }

                            // Add to history
                            session_data.command_history.push(command.clone());
                            if session_data.command_history.len() > 100 {
                                session_data.command_history.remove(0);
                            }
                            session_data.history_index = None;
                            session_data.queries_count += 1;

                            // Clear current line
                            session_data.current_line.clear();
                            session_data.cursor_pos = 0;

                            // Process command
                            drop(sessions); // Release lock before async operation
                            let response = self.process_whois_query(&command).await;
                            session.data(channel, CryptoVec::from_slice(response.as_bytes()));
                        } else {
                            session_data.current_line.clear();
                            session_data.cursor_pos = 0;
                        }

                        // Send new prompt
                        session.data(channel, CryptoVec::from_slice(b"whois> "));
                    }

                    // Backspace
                    b'\x08' | b'\x7f' => {
                        if session_data.cursor_pos > 0 {
                            session_data
                                .current_line
                                .remove(session_data.cursor_pos - 1);
                            session_data.cursor_pos -= 1;

                            // Move cursor back, clear to end of line, rewrite line
                            session.data(channel, CryptoVec::from_slice(b"\x08\x1B[K"));
                            let remaining = &session_data.current_line[session_data.cursor_pos..];
                            if !remaining.is_empty() {
                                session.data(channel, CryptoVec::from_slice(remaining.as_bytes()));
                                // Move cursor back to correct position
                                let move_back = format!("\x1B[{}D", remaining.len());
                                session.data(channel, CryptoVec::from_slice(move_back.as_bytes()));
                            }
                        }
                    }

                    // Escape sequence start
                    b'\x1b' => {
                        session_data.escape_state = EscapeState::Escape;
                        session_data.escape_buffer.clear();
                        session_data.escape_buffer.push(byte);
                    }

                    // Ctrl+C
                    b'\x03' => {
                        if session_data.current_line.is_empty() {
                            // If no current input, offer to exit
                            session.data(
                                channel,
                                CryptoVec::from_slice(
                                    b"^C\r\nType 'exit' to quit, or continue with queries.\r\nwhois> "
                                )
                            );
                        } else {
                            // Clear current line
                            session.data(channel, CryptoVec::from_slice(b"^C\r\n"));
                            session_data.current_line.clear();
                            session_data.cursor_pos = 0;
                            session_data.history_index = None;
                            session.data(channel, CryptoVec::from_slice(b"whois> "));
                        }
                    }

                    // Ctrl+D (EOF)
                    b'\x04' => {
                        if session_data.current_line.is_empty() {
                            session.data(channel, CryptoVec::from_slice(b"exit\r\n"));
                            session.close(channel);
                        }
                    }

                    // Ctrl+A (beginning of line)
                    b'\x01' => {
                        if session_data.cursor_pos > 0 {
                            let move_back = format!("\x1B[{}D", session_data.cursor_pos);
                            session.data(channel, CryptoVec::from_slice(move_back.as_bytes()));
                            session_data.cursor_pos = 0;
                        }
                    }

                    // Ctrl+E (end of line)
                    b'\x05' => {
                        let line_len = session_data.current_line.len();
                        if session_data.cursor_pos < line_len {
                            let move_forward = line_len - session_data.cursor_pos;
                            let move_cmd = format!("\x1B[{move_forward}C");
                            session.data(channel, CryptoVec::from_slice(move_cmd.as_bytes()));
                            session_data.cursor_pos = line_len;
                        }
                    }

                    // Ctrl+L (clear screen)
                    b'\x0c' => {
                        session.data(channel, CryptoVec::from_slice(b"\x1B[2J\x1B[H"));
                        let prompt_and_line = format!("whois> {}", session_data.current_line);
                        session.data(channel, CryptoVec::from_slice(prompt_and_line.as_bytes()));
                        if session_data.cursor_pos < session_data.current_line.len() {
                            let move_back =
                                session_data.current_line.len() - session_data.cursor_pos;
                            let move_cmd = format!("\x1B[{move_back}D");
                            session.data(channel, CryptoVec::from_slice(move_cmd.as_bytes()));
                        }
                    }

                    // Tab (for potential completion in the future)
                    b'\t' => {
                        // For now, ignore tab
                    }

                    // Regular printable characters
                    32..=126 => {
                        let ch = byte as char;
                        session_data
                            .current_line
                            .insert(session_data.cursor_pos, ch);
                        session_data.cursor_pos += 1;

                        // Echo the character
                        session.data(channel, CryptoVec::from_slice(&[byte]));

                        // If we inserted in the middle, redraw the rest of the line
                        let remaining = &session_data.current_line[session_data.cursor_pos..];
                        if !remaining.is_empty() {
                            session.data(channel, CryptoVec::from_slice(remaining.as_bytes()));
                            // Move cursor back to correct position
                            let move_back = format!("\x1B[{}D", remaining.len());
                            session.data(channel, CryptoVec::from_slice(move_back.as_bytes()));
                        }
                    }

                    // Ignore other control characters
                    _ => {}
                }
            }

            EscapeState::Escape => {
                session_data.escape_buffer.push(byte);
                match byte {
                    b'[' => {
                        session_data.escape_state = EscapeState::Csi;
                    }
                    _ => {
                        // Unknown escape sequence, ignore
                        session_data.escape_state = EscapeState::Normal;
                        session_data.escape_buffer.clear();
                    }
                }
            }

            EscapeState::Csi => {
                session_data.escape_buffer.push(byte);
                match byte {
                    // Arrow keys and other CSI sequences
                    b'A'..=b'Z' | b'a'..=b'z' => {
                        let escape_buffer = session_data.escape_buffer.clone();
                        session_data.escape_state = EscapeState::Normal;
                        session_data.escape_buffer.clear();
                        drop(sessions); // Release lock before calling handle_csi_sequence
                        self.handle_csi_sequence(channel, &escape_buffer, session)
                            .await?;
                        return Ok(()); // Early return to avoid re-acquiring lock
                    }
                    // Continue building the sequence
                    b'0'..=b'9' | b';' | b'?' => {
                        // Continue
                    }
                    _ => {
                        // Invalid sequence, ignore
                        session_data.escape_state = EscapeState::Normal;
                        session_data.escape_buffer.clear();
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_csi_sequence(
        &mut self,
        channel: ChannelId,
        sequence: &[u8],
        session: &mut server::Session,
    ) -> Result<(), anyhow::Error> {
        let mut sessions = self.sessions.lock().await;
        let session_data = match sessions.get_mut(&channel) {
            Some(s) => s,
            None => {
                return Ok(());
            }
        };

        if sequence.len() >= 3 && sequence[0] == b'\x1b' && sequence[1] == b'[' {
            match sequence[sequence.len() - 1] {
                // Up arrow - previous command in history
                b'A' => {
                    if !session_data.command_history.is_empty() {
                        let new_index = match session_data.history_index {
                            None => session_data.command_history.len() - 1,
                            Some(idx) => {
                                if idx > 0 {
                                    idx - 1
                                } else {
                                    0
                                }
                            }
                        };

                        if let Some(cmd) = session_data.command_history.get(new_index) {
                            // Clear current line
                            session.data(channel, CryptoVec::from_slice(b"\r\x1B[K"));
                            session.data(channel, CryptoVec::from_slice(b"whois> "));

                            // Display command from history
                            session.data(channel, CryptoVec::from_slice(cmd.as_bytes()));

                            session_data.current_line = cmd.clone();
                            session_data.cursor_pos = cmd.len();
                            session_data.history_index = Some(new_index);
                        }
                    }
                }

                // Down arrow - next command in history
                b'B' => {
                    if let Some(idx) = session_data.history_index {
                        if idx + 1 < session_data.command_history.len() {
                            let new_index = idx + 1;
                            if let Some(cmd) = session_data.command_history.get(new_index) {
                                // Clear current line
                                session.data(channel, CryptoVec::from_slice(b"\r\x1B[K"));
                                session.data(channel, CryptoVec::from_slice(b"whois> "));

                                // Display command from history
                                session.data(channel, CryptoVec::from_slice(cmd.as_bytes()));

                                session_data.current_line = cmd.clone();
                                session_data.cursor_pos = cmd.len();
                                session_data.history_index = Some(new_index);
                            }
                        } else {
                            // Clear line (go beyond history)
                            session.data(channel, CryptoVec::from_slice(b"\r\x1B[K"));
                            session.data(channel, CryptoVec::from_slice(b"whois> "));

                            session_data.current_line.clear();
                            session_data.cursor_pos = 0;
                            session_data.history_index = None;
                        }
                    }
                }

                // Right arrow - move cursor right
                b'C' => {
                    if session_data.cursor_pos < session_data.current_line.len() {
                        session.data(channel, CryptoVec::from_slice(b"\x1B[C"));
                        session_data.cursor_pos += 1;
                    }
                }

                // Left arrow - move cursor left
                b'D' => {
                    if session_data.cursor_pos > 0 {
                        session.data(channel, CryptoVec::from_slice(b"\x1B[D"));
                        session_data.cursor_pos -= 1;
                    }
                }

                // Home key
                b'H' => {
                    if session_data.cursor_pos > 0 {
                        let move_back = format!("\x1B[{}D", session_data.cursor_pos);
                        session.data(channel, CryptoVec::from_slice(move_back.as_bytes()));
                        session_data.cursor_pos = 0;
                    }
                }

                // End key
                b'F' => {
                    let line_len = session_data.current_line.len();
                    if session_data.cursor_pos < line_len {
                        let move_forward = line_len - session_data.cursor_pos;
                        let move_cmd = format!("\x1B[{move_forward}C");
                        session.data(channel, CryptoVec::from_slice(move_cmd.as_bytes()));
                        session_data.cursor_pos = line_len;
                    }
                }

                // Delete key
                b'~' if sequence.len() >= 4 && sequence[2] == b'3' => {
                    if session_data.cursor_pos < session_data.current_line.len() {
                        session_data.current_line.remove(session_data.cursor_pos);

                        // Clear to end of line and redraw
                        session.data(channel, CryptoVec::from_slice(b"\x1B[K"));
                        let remaining = &session_data.current_line[session_data.cursor_pos..];
                        if !remaining.is_empty() {
                            session.data(channel, CryptoVec::from_slice(remaining.as_bytes()));
                            // Move cursor back to correct position
                            let move_back = format!("\x1B[{}D", remaining.len());
                            session.data(channel, CryptoVec::from_slice(move_back.as_bytes()));
                        }
                    }
                }

                _ => {
                    // Unknown sequence, ignore
                }
            }
        }

        Ok(())
    }
}
