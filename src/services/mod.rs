// Core modules (always compiled)
pub mod desc;
pub mod help;
pub mod iana_cache;
pub mod rdap;
pub mod utils;
pub mod whois;

// Feature-gated modules
#[cfg(feature = "bgp")]
pub mod bgptool;
#[cfg(feature = "bgp")]
pub mod irr;
#[cfg(feature = "bgp")]
pub mod looking_glass;
#[cfg(feature = "bgp")]
pub mod rpki;
#[cfg(feature = "bgp-manrs")]
pub mod manrs;
#[cfg(feature = "peeringdb")]
pub mod peeringdb;
#[cfg(feature = "geo")]
pub mod geo;
#[cfg(feature = "dns")]
pub mod dns;
#[cfg(feature = "ping")]
pub mod ping;
#[cfg(feature = "traceroute")]
pub mod traceroute;
#[cfg(feature = "ntp")]
pub mod ntp;
#[cfg(feature = "ssl")]
pub mod ssl;
#[cfg(feature = "crt")]
pub mod crt;
#[cfg(feature = "cfstatus")]
pub mod cfstatus;
#[cfg(feature = "email")]
pub mod email;
#[cfg(feature = "steam")]
pub mod steam;
#[cfg(feature = "imdb")]
pub mod imdb;
#[cfg(feature = "minecraft")]
pub mod minecraft;
#[cfg(feature = "acgc")]
pub mod acgc;
#[cfg(feature = "lyric")]
pub mod lyric;
#[cfg(feature = "meal")]
pub mod meal;
#[cfg(feature = "wikipedia")]
pub mod wikipedia;
#[cfg(feature = "github")]
pub mod github;
#[cfg(feature = "icp")]
pub mod icp;
#[cfg(feature = "pen")]
pub mod pen;
#[cfg(feature = "pixiv")]
pub mod pixiv;

#[cfg(any(
    feature = "pkg-alma", feature = "pkg-aosc", feature = "pkg-aur",
    feature = "pkg-cargo", feature = "pkg-debian", feature = "pkg-epel",
    feature = "pkg-npm", feature = "pkg-pypi", feature = "pkg-nixos",
    feature = "pkg-ubuntu", feature = "pkg-opensuse", feature = "pkg-openwrt",
    feature = "pkg-modrinth", feature = "pkg-curseforge",
))]
pub mod packages;

// Core re-exports (always compiled)
pub use desc::*;
pub use whois::*;
// pub use iana_cache::*; // Used via explicit imports in modules
pub use rdap::process_rdap_query;

// Feature-gated re-exports
#[cfg(feature = "bgp")]
pub use bgptool::*;
#[cfg(feature = "bgp")]
pub use irr::*;
#[cfg(feature = "bgp")]
pub use looking_glass::*;
#[cfg(feature = "bgp")]
pub use rpki::*;
#[cfg(feature = "bgp-manrs")]
pub use manrs::*;
#[cfg(feature = "peeringdb")]
pub use peeringdb::*;
#[cfg(feature = "geo")]
pub use geo::*;
#[cfg(feature = "dns")]
pub use dns::*;
#[cfg(feature = "ping")]
pub use ping::*;
#[cfg(feature = "traceroute")]
pub use traceroute::*;
#[cfg(feature = "ntp")]
pub use ntp::*;
#[cfg(feature = "ssl")]
pub use ssl::*;
#[cfg(feature = "crt")]
pub use crt::*;
#[cfg(feature = "cfstatus")]
pub use cfstatus::*;
#[cfg(feature = "email")]
pub use email::*;
#[cfg(feature = "steam")]
pub use steam::*;
#[cfg(feature = "imdb")]
pub use imdb::*;
#[cfg(feature = "minecraft")]
pub use minecraft::*;
#[cfg(feature = "acgc")]
pub use acgc::*;
#[cfg(feature = "lyric")]
pub use lyric::*;
#[cfg(feature = "meal")]
pub use meal::*;
#[cfg(feature = "wikipedia")]
pub use wikipedia::*;
#[cfg(feature = "github")]
pub use github::*;
#[cfg(feature = "icp")]
pub use icp::*;
#[cfg(feature = "pen")]
pub use pen::*;
// pub use pixiv::*; // Pixiv implementation used via explicit imports

#[cfg(any(
    feature = "pkg-alma", feature = "pkg-aosc", feature = "pkg-aur",
    feature = "pkg-cargo", feature = "pkg-debian", feature = "pkg-epel",
    feature = "pkg-npm", feature = "pkg-pypi", feature = "pkg-nixos",
    feature = "pkg-ubuntu", feature = "pkg-opensuse", feature = "pkg-openwrt",
    feature = "pkg-modrinth", feature = "pkg-curseforge",
))]
pub use packages::*;
