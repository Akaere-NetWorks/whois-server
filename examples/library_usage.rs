// Example of using whois-server as a library
// This demonstrates how to integrate whois-server functionality into your own application

use whois_server::{ColorScheme, query, query_with_color};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== WHOIS Server Library Usage Examples ===\n");

    // Example 1: Simple domain query
    println!("1. Querying domain 'example.com':");
    match query("example.com").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 2: ASN query
    println!("2. Querying ASN 'AS13335':");
    match query("AS13335").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 3: IP geolocation query
    println!("3. Querying IP geolocation '1.1.1.1-GEO':");
    match query("1.1.1.1-GEO").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 4: DN42 domain query
    println!("4. Querying DN42 domain 'example.dn42':");
    match query("example.dn42").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 5: BGP Tools query
    println!("5. Querying BGP info '1.1.1.0-BGPTOOL':");
    match query("1.1.1.0-BGPTOOL").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 6: DNS query
    println!("6. Querying DNS 'google.com-DNS':");
    match query("google.com-DNS").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 7: IRR Explorer query
    println!("7. Querying IRR Explorer '192.0.2.0/24-IRR':");
    match query("192.0.2.0/24-IRR").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 8: Package repository query (Cargo)
    println!("8. Querying Cargo package 'tokio-CARGO':");
    match query("tokio-CARGO").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 9: GitHub query
    println!("9. Querying GitHub user 'torvalds-GITHUB':");
    match query("torvalds-GITHUB").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 10: Query with color scheme
    println!("10. Querying with RIPE color scheme 'example.com':");
    match query_with_color("example.com", Some(ColorScheme::Ripe)).await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 11: Help query
    println!("11. Getting help:");
    match query("HELP").await {
        Ok(result) => println!("{}\n", result),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    println!("=== All examples completed ===");
    Ok(())
}
