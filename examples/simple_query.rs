// Minimal example of using whois-server as a library
use whois_server::query;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Simple query - just like using the whois command
    let result = query("example.com").await?;
    println!("{}", result);

    Ok(())
}
