/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

mod config;
mod core;
mod dn42;
mod plugins;
mod server;
mod services;
mod ssh;
mod storage;
mod web;

use anyhow::Result;
use clap::Parser;

use core::logger::init_from_args;

use config::Cli;
use core::{create_stats_state, get_patches_count, init_patches, save_stats_on_shutdown};
use dn42::{
    dn42_manager_maintenance, get_dn42_platform_info, initialize_dn42_manager, is_dn42_online_mode,
    start_periodic_sync,
};
use server::{create_dump_dir_if_needed, run_async_server};
use services::pen::start_pen_periodic_update;
use ssh::{SshServer, server::SshServerConfig};
use tokio::time::{Duration, interval};
use web::run_web_server;
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    let args = Cli::parse();

    // Initialize systemd-style logger
    init_from_args(args.debug, args.trace, false)
        .map_err(|e| anyhow::anyhow!("Failed to initialize logger: {}", e))?;

    // Create statistics state
    let stats = create_stats_state().await;

    // Create dump directory if needed
    create_dump_dir_if_needed(args.dump_traffic, &args.dump_dir)?;

    // Initialize patch system
    log_init_start!("Response Patches Loader");
    match init_patches("./patches") {
        Ok(_count) => {
            let (files, rules) = get_patches_count();
            log_init_ok_with_details!("Response Patches Loader", &format!("{} files, {} rules", files, rules));
        }
        Err(e) => {
            log_init_warn!("Response Patches Loader", &format!("continuing without patches: {}", e));
        }
    }

    // Initialize DN42 manager (platform-aware)
    log_init_start!("DN42 System");
    if let Err(e) = initialize_dn42_manager().await {
        log_init_failed!("DN42 System", &format!("manager initialization failed: {}", e));
    } else {
        let platform_info = get_dn42_platform_info().await.unwrap_or("Unknown");
        let is_online = is_dn42_online_mode().await.unwrap_or(false);
        log_init_ok_with_details!("DN42 System", &format!("Platform: {}, Mode: {}", platform_info, if is_online { "Online" } else { "Git" }));
    }

    // Start DN42 sync task (Git mode) or maintenance task (Online mode)
    tokio::spawn(async move {
        if let Ok(is_online) = is_dn42_online_mode().await {
            if is_online {
                log_info!("Starting DN42 online mode maintenance task (every hour)");
                let mut maintenance_interval = interval(Duration::from_secs(3600)); // 1 hour
                maintenance_interval.tick().await; // Skip the first tick

                loop {
                    maintenance_interval.tick().await;
                    log_info!("Running scheduled DN42 online maintenance");
                    if let Err(e) = dn42_manager_maintenance().await {
                        log_error!("DN42 online maintenance failed: {}", e);
                    }
                }
            } else {
                log_info!("Starting DN42 git mode periodic sync");
                start_periodic_sync().await;
            }
        } else {
            log_error!("Failed to determine DN42 mode, falling back to git sync");
            start_periodic_sync().await;
        }
    });

    // Load plugins
    log_init_start!("Plugin System");
    let plugin_registry = match plugins::load_all_plugins().await {
        Ok(registry) => {
            let plugin_count = registry.len();
            log_init_ok_with_details!("Plugin System", &format!("{} plugin(s) loaded", plugin_count));

            // Store plugin registry for query detection (Arc for thread-safe sharing)
            let shared_registry = std::sync::Arc::new(registry);
            core::query::set_plugin_registry(shared_registry.clone());

            Some(shared_registry)
        }
        Err(e) => {
            log_init_warn!("Plugin System", &format!("continuing without plugins: {}", e));
            None
        }
    };

    // Start PEN (Private Enterprise Numbers) periodic update task
    tokio::spawn(async move {
        log_task_start!("PEN Periodic Update Service");
        start_pen_periodic_update().await;
    });

    // Start web server
    let web_stats = stats.clone();
    let web_port = args.web_port;
    tokio::spawn(async move {
        log_task_start!(&format!("Web Server on port {}", web_port));
        if let Err(e) = run_web_server(web_stats, web_port).await {
            log_error!("Web server error: {}", e);
        }
    });

    // Start SSH server if enabled
    if args.enable_ssh {
        let ssh_config = SshServerConfig {
            listen_addr: args.host.clone(),
            port: args.ssh_port,
            cache_dir: args.ssh_cache_dir.clone(),
        };

        tokio::spawn(async move {
            log_task_start!(&format!("SSH Server on port {}", ssh_config.port));
            let mut ssh_server = match SshServer::new(ssh_config) {
                Ok(server) => server,
                Err(e) => {
                    log_error!("Failed to create SSH server: {}", e);
                    return;
                }
            };

            if let Err(e) = ssh_server.initialize().await {
                log_error!("Failed to initialize SSH server: {}", e);
                return;
            }

            if let Err(e) = ssh_server.start().await {
                log_error!("SSH server error: {}", e);
            }
        });
    }

    // Create server address
    let addr = format!("{}:{}", args.host, args.port);
    log_task_start!(&format!("WHOIS Server on {}", addr));

    // Start async server
    let result = run_async_server(
        &addr,
        args.max_connections,
        args.timeout,
        args.dump_traffic,
        &args.dump_dir,
        stats.clone(),
        args.enable_color,
    )
    .await;

    // Save stats on shutdown
    log_info!("Saving statistics before shutdown...");
    save_stats_on_shutdown(&stats).await;

    // Cleanup plugins
    if let Some(registry) = plugin_registry {
        log_info!("Cleaning up plugins...");
        for suffix in registry.get_all_suffixes() {
            if let Some(plugin) = registry.get_plugin(&suffix) {
                plugin.call_cleanup();
            }
        }
    }

    result
}
