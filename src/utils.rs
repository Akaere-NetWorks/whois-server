use std::fs::File;
use std::io::Write;
use tracing::{debug, error};

// Helper function to dump content to a file
pub fn dump_to_file(filename: &str, content: &str) {
    match File::create(filename) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content.as_bytes()) {
                error!("Failed to write to dump file {}: {}", filename, e);
            } else {
                debug!("Wrote {} bytes to {}", content.len(), filename);
            }
        },
        Err(e) => error!("Failed to create dump file {}: {}", filename, e),
    }
} 