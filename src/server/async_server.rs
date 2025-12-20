use anyhow::{Context, Result};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use crate::{log_error, log_info};

use super::connection::handle_connection;
use crate::core::StatsState;

pub async fn run_async_server(
    addr: &str,
    max_connections: usize,
    timeout: u64,
    dump_traffic: bool,
    dump_dir: &str,
    stats: StatsState,
    enable_color: bool,
) -> Result<()> {
    // Start server
    let listener = TcpListener::bind(&addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    let (tx, mut rx) = mpsc::channel::<()>(max_connections);

    // Handle connections
    loop {
        tokio::select! {
            _ = rx.recv() => {
                // A connection completed, continue accepting new connections
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        log_info!("Accepted connection from {}", addr);
                        let tx_clone = tx.clone();
                        let stats_clone = stats.clone();

                        // Set timeout
                        let timeout = Duration::from_secs(timeout);
                        let dump_traffic = dump_traffic;
                        let dump_dir = dump_dir.to_string();

                        // Handle connection
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, addr, timeout, dump_traffic, &dump_dir, stats_clone, enable_color).await {
                                log_error!("Connection handling error: {}", e);
                            }

                            // Notify completion
                            let _ = tx_clone.send(()).await;
                        });
                    }
                    Err(e) => {
                        log_error!("Failed to accept connection: {}", e);
                    }
                }
            }
        }
    }
}
