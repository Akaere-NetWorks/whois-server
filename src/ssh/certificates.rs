// WHOIS Server - SSH Certificate Management
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result};
use russh_keys::{encode_pkcs8_pem, key, load_secret_key};
use std::fs;
use std::path::{Path, PathBuf};
use crate::{log_info};
/// Manages SSH server certificates and keys
pub struct SshCertificateManager {
    cache_dir: PathBuf,
    host_key_path: PathBuf,
}

impl SshCertificateManager {
    /// Create a new certificate manager
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Self {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        let host_key_path = cache_dir.join("ssh_host_key");

        Self {
            cache_dir,
            host_key_path,
        }
    }

    /// Initialize the certificate manager, creating directories and keys as needed
    pub async fn initialize(&self) -> Result<()> {
        // Create cache directory if it doesn't exist
        if !self.cache_dir.exists() {
            log_info!("Creating SSH cache directory: {:?}", self.cache_dir);
            fs::create_dir_all(&self.cache_dir).with_context(|| {
                format!("Failed to create SSH cache directory: {:?}", self.cache_dir)
            })?;
        }

        // Generate host key if it doesn't exist
        if !self.host_key_path.exists() {
            log_info!("Generating new SSH host key: {:?}", self.host_key_path);
            self.generate_host_key()
                .await
                .with_context(|| "Failed to generate SSH host key")?;
        } else {
            log_info!("Using existing SSH host key: {:?}", self.host_key_path);
        }

        Ok(())
    }

    /// Generate a new SSH host key
    async fn generate_host_key(&self) -> Result<()> {
        // Generate an Ed25519 key pair
        let key_pair = key::KeyPair::generate_ed25519()
            .ok_or_else(|| anyhow::anyhow!("Failed to generate Ed25519 key pair"))?;

        // Encode the private key to PEM format
        let mut pem_data = Vec::new();
        encode_pkcs8_pem(&key_pair, &mut pem_data)
            .with_context(|| "Failed to encode private key to PEM format")?;

        fs::write(&self.host_key_path, &pem_data)
            .with_context(|| format!("Failed to write private key to {:?}", self.host_key_path))?;

        // Set appropriate permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.host_key_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.host_key_path, perms)?;
        }

        log_info!("Successfully generated and saved SSH host key");
        Ok(())
    }

    /// Load the SSH host key
    pub async fn load_host_key(&self) -> Result<key::KeyPair> {
        if !self.host_key_path.exists() {
            return Err(anyhow::anyhow!(
                "SSH host key does not exist: {:?}",
                self.host_key_path
            ));
        }

        let key_pair = load_secret_key(&self.host_key_path, None).with_context(|| {
            format!("Failed to load SSH host key from {:?}", self.host_key_path)
        })?;

        Ok(key_pair)
    }

    /// Get the public key fingerprint for logging/display purposes
    pub async fn get_public_key_fingerprint(&self) -> Result<String> {
        let key_pair = self.load_host_key().await?;
        let public_key = key_pair.clone_public_key()?;

        // Calculate SHA256 fingerprint
        let fingerprint = public_key.fingerprint();
        Ok(fingerprint)
    }
}
