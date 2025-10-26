// WHOIS Server - SSH Module
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SSH server module providing WHOIS services over SSH protocol
//!
//! This module implements an SSH server that listens on port 2222 and provides
//! WHOIS query functionality directly through SSH sessions. Features include:
//! - Fixed SSH server certificates stored in ./cache/ssh
//! - Connection history tracking with LMDB (100 records, 30 days retention)
//! - Direct WHOIS query processing without command prefixes

pub mod certificates;
pub mod handler;
pub mod history;
pub mod server;

#[allow(unused_imports)]
pub use certificates::SshCertificateManager;
#[allow(unused_imports)]
pub use handler::WhoisSshHandler;
#[allow(unused_imports)]
pub use history::SshConnectionHistory;
pub use server::SshServer;
