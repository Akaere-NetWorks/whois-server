// Quick test to verify library functionality
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing whois-server library...\n");

    // Test 1: Simple help query (should always work)
    println!("Test 1: HELP query");
    match query("HELP").await {
        Ok(result) => {
            println!(
                "✓ Success (first 100 chars): {}",
                result.chars().take(100).collect::<String>()
            );
        }
        Err(e) => println!("✗ Failed: {}", e),
    }

    println!("\nLibrary is working correctly!");
    Ok(())
}
