// WHOIS Server - SSH Server Implementation
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{ Context, Result };
use russh::server;
use russh_keys::key;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{ info, warn, error, debug };

use super::certificates::SshCertificateManager;
use super::handler::WhoisSshHandler;
use super::history::SshConnectionHistory;

/// SSH server configuration
#[derive(Debug, Clone)]
pub struct SshServerConfig {
    pub listen_addr: String,
    pub port: u16,
    pub cache_dir: String,
}

impl Default for SshServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 2222,
            cache_dir: "./cache/ssh".to_string(),
        }
    }
}

/// SSH server for WHOIS services
pub struct SshServer {
    config: SshServerConfig,
    cert_manager: SshCertificateManager,
    history: Arc<SshConnectionHistory>,
    host_key: Option<Arc<key::KeyPair>>,
}

impl SshServer {
    /// Create a new SSH server
    pub fn new(config: SshServerConfig) -> Result<Self> {
        let cert_manager = SshCertificateManager::new(&config.cache_dir);

        // Initialize SSH connection history storage
        let history_db_path = Path::new(&config.cache_dir).join("history.lmdb");
        debug!("Initializing SSH connection history at path: {:?}", history_db_path);
        let history = Arc::new(
            SshConnectionHistory::new(&history_db_path).with_context(||
                format!("Failed to initialize SSH connection history at {:?}", history_db_path)
            )?
        );

        Ok(Self {
            config,
            cert_manager,
            history,
            host_key: None,
        })
    }

    /// Initialize the SSH server
    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize certificate manager
        self.cert_manager
            .initialize().await
            .with_context(|| "Failed to initialize SSH certificate manager")?;

        // Load host key
        let host_key = self.cert_manager
            .load_host_key().await
            .with_context(|| "Failed to load SSH host key")?;

        self.host_key = Some(Arc::new(host_key));

        // Log host key fingerprint
        match self.cert_manager.get_public_key_fingerprint().await {
            Ok(fingerprint) => {
                info!("SSH server host key fingerprint: {}", fingerprint);
            }
            Err(e) => {
                warn!("Failed to get host key fingerprint: {}", e);
            }
        }

        Ok(())
    }

    /// Start the SSH server
    pub async fn start(&self) -> Result<()> {
        let host_key = self.host_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSH server not initialized - host key not loaded"))?;

        let bind_addr = format!("{}:{}", self.config.listen_addr, self.config.port);
        info!("Starting SSH server on {}", bind_addr);

        let listener = TcpListener::bind(&bind_addr).await.with_context(||
            format!("Failed to bind SSH server to {}", bind_addr)
        )?;

        info!("SSH server listening on {}", bind_addr);

        // Create server configuration
        let server_config = Arc::new(server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)), // 1 hour timeout
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![host_key.as_ref().clone()],
            ..Default::default()
        });

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    info!("SSH connection from {}", client_addr);

                    let history = Arc::clone(&self.history);
                    let host_key = Arc::clone(host_key);
                    let config = Arc::clone(&server_config);

                    tokio::spawn(async move {
                        if
                            let Err(e) = Self::handle_connection(
                                stream,
                                client_addr,
                                history,
                                host_key,
                                config
                            ).await
                        {
                            error!("SSH connection error from {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept SSH connection: {}", e);
                }
            }
        }
    }

    /// Handle a single SSH connection
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        client_addr: SocketAddr,
        history: Arc<SshConnectionHistory>,
        host_key: Arc<key::KeyPair>,
        config: Arc<server::Config>
    ) -> Result<()> {
        let mut handler = WhoisSshHandler::new(history, host_key);
        handler.set_client_addr(client_addr);

        let _session = server
            ::run_stream(config, stream, handler).await
            .with_context(|| format!("SSH session failed for {}", client_addr))?;

        debug!("SSH session completed for {}", client_addr);
        Ok(())
    }

    /// Get SSH server statistics
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> Result<SshServerStats> {
        let total_records = self.history
            .get_total_records()
            .with_context(|| "Failed to get SSH history record count")?;

        Ok(SshServerStats {
            total_connections: total_records,
            active_connections: 0, // TODO: Track active connections
            cache_directory: self.config.cache_dir.clone(),
        })
    }
}

/// SSH server statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SshServerStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub cache_directory: String,
}
