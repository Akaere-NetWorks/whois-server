use anyhow::{ Result, anyhow };
use serde::{ Deserialize, Serialize };
use std::time::Duration;
use tracing::debug;

// RIPE STAT Looking Glass API
const RIPE_STAT_API_BASE: &str = "https://stat.ripe.net";

#[derive(Debug, Deserialize, Serialize)]
struct LookingGlassResponse {
    data: LookingGlassData,
    data_call_status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LookingGlassData {
    rrcs: Vec<RrcData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RrcData {
    rrc: String,
    location: String,
    peers: Vec<PeerData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PeerData {
    asn_origin: String,
    as_path: String,
    community: String,
    #[serde(rename = "largeCommunity")]
    large_community: String,
    #[serde(rename = "extendedCommunity")]
    extended_community: String,
    last_updated: String,
    prefix: String,
    peer: String,
    origin: String,
    next_hop: String,
    latest_time: String,
}

/// Process Looking Glass queries ending with -LG (async version)
pub async fn process_looking_glass_query(resource: &str) -> Result<String> {
    debug!("Processing Looking Glass query for: {}", resource);

    let url = format!("{}/data/looking-glass/data.json?resource={}", RIPE_STAT_API_BASE, resource);
    debug!("Requesting URL: {}", url);

    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("API request failed with status: {}", response.status()));
    }

    let lg_response: LookingGlassResponse = response.json().await?;

    if lg_response.data_call_status != "supported" {
        return Err(anyhow!("Looking Glass data call not supported"));
    }

    format_bird_output(&lg_response.data, resource)
}

/// Format Looking Glass response in BIRD-style format
fn format_bird_output(data: &LookingGlassData, resource: &str) -> Result<String> {
    let mut output = String::new();

    // BIRD-style header
    output.push_str(&format!("% RIPE STAT Looking Glass data for {}\n", resource));
    output.push_str("% Data from RIPE NCC Route Information Service (RIS)\n");
    output.push_str("% Output in BIRD routing daemon style\n\n");

    if data.rrcs.is_empty() {
        output.push_str("% No routing data found\n");
        return Ok(output);
    }

    // Group routes by prefix for better organization
    let mut routes_by_prefix: std::collections::HashMap<
        String,
        Vec<&PeerData>
    > = std::collections::HashMap::new();

    for rrc in &data.rrcs {
        for peer in &rrc.peers {
            routes_by_prefix.entry(peer.prefix.clone()).or_insert_with(Vec::new).push(peer);
        }
    }

    for (prefix, peers) in routes_by_prefix {
        output.push_str(&format!("# Routes for prefix {}\n", prefix));

        for peer in peers {
            // BIRD-style route format
            output.push_str(&format!("route {} via {} {{\n", prefix, peer.next_hop));
            output.push_str(&format!("    # Peer: {} (AS{})\n", peer.peer, peer.asn_origin));
            output.push_str(&format!("    # AS-Path: {}\n", peer.as_path));
            output.push_str(&format!("    # Origin: {}\n", peer.origin));

            if !peer.community.is_empty() {
                output.push_str(&format!("    # Communities: {}\n", peer.community));
            }

            if !peer.large_community.is_empty() {
                output.push_str(&format!("    # Large Communities: {}\n", peer.large_community));
            }

            if !peer.extended_community.is_empty() {
                output.push_str(
                    &format!("    # Extended Communities: {}\n", peer.extended_community)
                );
            }

            output.push_str(&format!("    # Last Updated: {}\n", peer.last_updated));
            output.push_str(&format!("    # Latest Time: {}\n", peer.latest_time));

            // BIRD-style attributes
            output.push_str(
                &format!("    bgp_path.len = {};\n", peer.as_path.split_whitespace().count())
            );
            output.push_str(&format!("    bgp_origin = {};\n", peer.origin.to_uppercase()));
            output.push_str(&format!("    bgp_next_hop = {};\n", peer.next_hop));

            // Add communities as BIRD attributes if present
            if !peer.community.is_empty() {
                let communities: Vec<&str> = peer.community.split_whitespace().collect();
                for community in communities {
                    if community.contains(':') {
                        output.push_str(
                            &format!(
                                "    bgp_community.add(({},{}));\n",
                                community.split(':').nth(0).unwrap_or("0"),
                                community.split(':').nth(1).unwrap_or("0")
                            )
                        );
                    }
                }
            }

            output.push_str("}}\n\n");
        }
    }

    // Summary statistics
    let total_routes = data.rrcs
        .iter()
        .map(|rrc| rrc.peers.len())
        .sum::<usize>();
    let total_rrcs = data.rrcs.len();

    output.push_str(
        &format!("# Summary: {} routes from {} RRC collectors\n", total_routes, total_rrcs)
    );

    // List all RRC locations
    output.push_str("# RRC Locations:\n");
    for rrc in &data.rrcs {
        let peer_count = rrc.peers.len();
        output.push_str(&format!("#   {}: {} ({} peers)\n", rrc.rrc, rrc.location, peer_count));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bird_output() {
        let test_data = LookingGlassData {
            rrcs: vec![RrcData {
                rrc: "RRC00".to_string(),
                location: "Amsterdam, Netherlands".to_string(),
                peers: vec![PeerData {
                    asn_origin: "1205".to_string(),
                    as_path: "34854 6939 1853 1853 1205".to_string(),
                    community: "34854:1000".to_string(),
                    large_community: "".to_string(),
                    extended_community: "".to_string(),
                    last_updated: "2025-05-31T23:16:01".to_string(),
                    prefix: "140.78.0.0/16".to_string(),
                    peer: "2.56.11.1".to_string(),
                    origin: "IGP".to_string(),
                    next_hop: "2.56.11.1".to_string(),
                    latest_time: "2025-06-09T09:11:57".to_string(),
                }],
            }],
        };

        let result = format_bird_output(&test_data, "140.78.0.0/16").unwrap();

        assert!(result.contains("% RIPE STAT Looking Glass data"));
        assert!(result.contains("route 140.78.0.0/16 via 2.56.11.1"));
        assert!(result.contains("bgp_path.len = 5"));
        assert!(result.contains("bgp_origin = IGP"));
        assert!(result.contains("bgp_community.add((34854,1000))"));
    }
}
