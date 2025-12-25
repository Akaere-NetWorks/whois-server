use clap::Parser;

// WHOIS server constants
pub const DEFAULT_WHOIS_SERVER: &str = "whois.ripe.net";
pub const DEFAULT_WHOIS_PORT: u16 = 43;
pub const TIMEOUT_SECONDS: u64 = 10;
// DN42 registry configuration
pub const DN42_REGISTRY_PATH: &str = "./cache/dn42-registry";
pub const DN42_LMDB_PATH: &str = "./cache/dn42-lmdb";

// PeeringDB cache configuration
pub const PEERINGDB_LMDB_PATH: &str = "./cache/peeringdb-lmdb";
pub const PEERINGDB_CACHE_TTL: u64 = 86400; // 1 day in seconds

// ICP filing cache configuration
pub const ICP_LMDB_PATH: &str = "./cache/icp-lmdb";
pub const ICP_CACHE_TTL: u64 = 86400; // 1 day in seconds

// Statistics LMDB configuration
pub const STATS_LMDB_PATH: &str = "./cache/stats-lmdb";

// Internet Routing Registry (IRR) servers
pub const RADB_WHOIS_SERVER: &str = "whois.radb.net";
pub const RADB_WHOIS_PORT: u16 = 43;
pub const ALTDB_WHOIS_SERVER: &str = "whois.altdb.net";
pub const ALTDB_WHOIS_PORT: u16 = 43;
pub const AFRINIC_WHOIS_SERVER: &str = "whois.afrinic.net";
pub const AFRINIC_WHOIS_PORT: u16 = 43;
pub const APNIC_WHOIS_SERVER: &str = "whois.apnic.net";
pub const APNIC_WHOIS_PORT: u16 = 43;
pub const ARIN_WHOIS_SERVER: &str = "rr.arin.net";
pub const ARIN_WHOIS_PORT: u16 = 43;
pub const BELL_WHOIS_SERVER: &str = "whois.in.bell.ca";
pub const BELL_WHOIS_PORT: u16 = 43;
pub const JPIRR_WHOIS_SERVER: &str = "jpirr.nic.ad.jp";
pub const JPIRR_WHOIS_PORT: u16 = 43;
pub const LACNIC_WHOIS_SERVER: &str = "irr.lacnic.net";
pub const LACNIC_WHOIS_PORT: u16 = 43;
pub const LEVEL3_WHOIS_SERVER: &str = "rr.level3.net";
pub const LEVEL3_WHOIS_PORT: u16 = 43;
pub const NTTCOM_WHOIS_SERVER: &str = "rr.ntt.net";
pub const NTTCOM_WHOIS_PORT: u16 = 43;
pub const RIPE_WHOIS_SERVER: &str = "whois.ripe.net";
pub const RIPE_WHOIS_PORT: u16 = 43;
pub const TC_WHOIS_SERVER: &str = "whois.bgp.net.br";
pub const TC_WHOIS_PORT: u16 = 43;

//RIPE NCC Routing Information Service (RIS) Whois
pub const RIS_WHOIS_SERVER: &str = "riswhois.ripe.net";
pub const RIS_WHOIS_PORT: u16 = 43;

// Server identification banner
pub const SERVER_BANNER: &str = "% Akaere NetWorks Whois Server";

// Pixiv image proxy configuration
pub fn pixiv_proxy_enabled() -> bool {
    std::env::var("PIXIV_PROXY_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false)
}

// Private IP range definitions
pub const PRIVATE_IPV4_RANGES: &[&str] = &[
    "10.0.0.0/8",      // RFC1918
    "172.16.0.0/12",   // RFC1918
    "192.168.0.0/16",  // RFC1918
    "169.254.0.0/16",  // Link-local addresses
    "192.0.2.0/24",    // Documentation examples (TEST-NET-1)
    "198.51.100.0/24", // Documentation examples (TEST-NET-2)
    "203.0.113.0/24",  // Documentation examples (TEST-NET-3)
    "100.64.0.0/10",   // CGNAT (Carrier-grade NAT)
    "127.0.0.0/8",     // Localhost
];

pub const PRIVATE_IPV6_RANGES: &[&str] = &[
    "fc00::/7",      // Unique Local Addresses
    "fd00::/8",      // Unique Local Addresses (subset)
    "fe80::/10",     // Link-local addresses
    "::1/128",       // Localhost
    "2001:db8::/32", // Documentation addresses
];

#[derive(Parser)]
#[command(author, version, about = "A simple WHOIS server")]
pub struct Cli {
    /// Listen address
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    pub host: String,

    /// Listen port
    #[arg(short, long, default_value_t = 43)]
    pub port: u16,

    /// Enable debug output
    #[arg(short, long)]
    pub debug: bool,

    /// Enable trace output (extremely verbose)
    #[arg(short, long)]
    pub trace: bool,

    /// Maximum concurrent connections
    #[arg(long, default_value_t = 100)]
    pub max_connections: usize,

    /// Connection timeout in seconds
    #[arg(long, default_value_t = 10)]
    pub timeout: u64,

    /// Write raw queries and responses to files for debugging
    #[arg(long)]
    pub dump_traffic: bool,

    /// Dump traffic directory (default: ./dumps)
    #[arg(long, default_value = "dumps")]
    pub dump_dir: String,

    /// Web dashboard port
    #[arg(long, default_value_t = 9999)]
    pub web_port: u16,

    /// Enable WHOIS-COLOR protocol support
    #[arg(long, default_value_t = true)]
    pub enable_color: bool,

    /// Enable SSH server
    #[arg(long)]
    pub enable_ssh: bool,

    /// SSH server port
    #[arg(long, default_value_t = 2222)]
    pub ssh_port: u16,

    /// SSH cache directory
    #[arg(long, default_value = "./cache/ssh")]
    pub ssh_cache_dir: String,
}
