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

mod bgptool;
mod config;
mod dn42;
mod email;
mod geo;
mod irr;
mod looking_glass;
mod query;
mod rpki;
mod server;
mod stats;
mod utils;
mod web;
mod whois;

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

use config::Cli;
use server::{create_dump_dir_if_needed, run_async_server, run_blocking_server};
use stats::{create_stats_state, save_stats_on_shutdown};
use web::run_web_server;
use dn42::start_periodic_sync;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    
    // Initialize logging
    let log_level = if args.trace {
        Level::TRACE
    } else if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_span_events(FmtSpan::CLOSE)
        .init();
    
    // Create statistics state
    let stats = create_stats_state().await;
    
    // Create dump directory if needed
    create_dump_dir_if_needed(args.dump_traffic, &args.dump_dir)?;
    
    // Start DN42 registry sync task
    tokio::spawn(async move {
        start_periodic_sync().await;
    });
    
    // Start web server
    let web_stats = stats.clone();
    let web_port = args.web_port;
    tokio::spawn(async move {
        info!("Starting web server on port {}", web_port);
        if let Err(e) = run_web_server(web_stats, web_port).await {
            tracing::error!("Web server error: {}", e);
        }
    });
    
    // Create server address
    let addr = format!("{}:{}", args.host, args.port);
    info!("Starting WHOIS server on {}", addr);
    
    if args.use_blocking {
        info!("Using blocking TCP connections (non-async)");
        run_blocking_server(&addr, args.timeout, args.dump_traffic, &args.dump_dir)?;
        return Ok(());
    }
    
    // Start async server
    let result = run_async_server(&addr, args.max_connections, args.timeout, args.dump_traffic, &args.dump_dir, stats.clone()).await;
    
    // Save stats on shutdown
    info!("Saving statistics before shutdown...");
    save_stats_on_shutdown(&stats).await;
    
    result
}
