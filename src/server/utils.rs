use anyhow::{Context, Result};
use std::path::Path;
use crate::{log_info};
pub fn create_dump_dir_if_needed(dump_traffic: bool, dump_dir: &str) -> Result<()> {
    if dump_traffic {
        let path = Path::new(dump_dir);
        if !path.exists() {
            log_info!("Creating dumps directory: {}", dump_dir);
            std::fs::create_dir_all(path).context("Failed to create dumps directory")?;
        }
    }
    Ok(())
}
