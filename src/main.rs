/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2024 Akaere Networks
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
mod email;
mod query;
mod server;
mod utils;
mod whois;

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

use config::Cli;
use server::{create_dump_dir_if_needed, run_async_server, run_blocking_server};

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
    
    // Create dump directory if needed
    create_dump_dir_if_needed(args.dump_traffic, &args.dump_dir)?;
    
    // Create server address
    let addr = format!("{}:{}", args.host, args.port);
    info!("Starting WHOIS server on {}", addr);
    
    if args.use_blocking {
        info!("Using blocking TCP connections (non-async)");
        run_blocking_server(&addr, args.timeout, args.dump_traffic, &args.dump_dir)?;
        return Ok(());
    }
    
    // Start async server
    run_async_server(&addr, args.max_connections, args.timeout, args.dump_traffic, &args.dump_dir).await
}
