use crate::{log_error, log_info};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

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

    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Handle connections
    loop {
        let permit = semaphore.clone().acquire_owned().await?;

        match listener.accept().await {
            Ok((stream, addr)) => {
                log_info!("Accepted connection from {}", addr);
                let stats_clone = stats.clone();

                // Set timeout
                let timeout = Duration::from_secs(timeout);
                let dump_dir = dump_dir.to_string();

                // Handle connection
                tokio::spawn(async move {
                    let _permit = permit; // Hold permit for connection lifetime
                    if let Err(e) = handle_connection(
                        stream,
                        addr,
                        timeout,
                        dump_traffic,
                        &dump_dir,
                        stats_clone,
                        enable_color,
                    )
                    .await
                    {
                        log_error!("Connection handling error: {}", e);
                    }
                });
            }
            Err(e) => {
                log_error!("Failed to accept connection: {}", e);
                drop(permit); // Release permit on accept failure
            }
        }
    }
}
