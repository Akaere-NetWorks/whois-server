use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RipeStatResponse {
    pub data: Option<RipeStatData>,
    pub status: String,
    #[allow(dead_code)]
    pub messages: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct RipeStatData {
    #[allow(dead_code)]
    pub prefixes: Option<Vec<GeoPrefix>>,
    pub located_resources: Option<Vec<LocatedResource>>,
    #[allow(dead_code)]
    pub unknown_resources: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeoPrefix {
    #[allow(dead_code)]
    pub prefix: String,
    #[allow(dead_code)]
    pub country: Option<String>,
    #[allow(dead_code)]
    pub city: Option<String>,
    #[allow(dead_code)]
    pub latitude: Option<f64>,
    #[allow(dead_code)]
    pub longitude: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocatedResource {
    pub resource: String,
    pub locations: Option<Vec<GeoLocation>>,
    #[allow(dead_code)]
    pub unknown_percentage: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeoLocation {
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[allow(dead_code)]
    pub resources: Option<Vec<String>>,
    #[allow(dead_code)]
    pub covered_percentage: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IpinfoResponse {
    pub ip: String,
    pub asn: Option<String>,
    pub as_name: Option<String>,
    pub as_domain: Option<String>,
    #[allow(dead_code)]
    pub country_code: Option<String>,
    pub country: Option<String>,
    #[allow(dead_code)]
    pub continent_code: Option<String>,
    pub continent: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    #[allow(dead_code)]
    pub latitude: Option<String>,
    #[allow(dead_code)]
    pub longitude: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RirGeoResponse {
    pub data: Option<RirGeoData>,
    pub status: String,
    #[allow(dead_code)]
    pub messages: Option<Vec<Vec<String>>>,
    #[allow(dead_code)]
    pub see_also: Option<Vec<String>>,
    #[allow(dead_code)]
    pub version: String,
    #[allow(dead_code)]
    pub data_call_name: String,
    #[allow(dead_code)]
    pub data_call_status: String,
    #[allow(dead_code)]
    pub cached: bool,
    #[allow(dead_code)]
    pub query_id: String,
    #[allow(dead_code)]
    pub process_time: u32,
    #[allow(dead_code)]
    pub server_id: String,
    #[allow(dead_code)]
    pub build_version: String,
    #[allow(dead_code)]
    pub status_code: u16,
    #[allow(dead_code)]
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct RirGeoData {
    pub located_resources: Option<Vec<RirGeoResource>>,
    #[allow(dead_code)]
    pub result_time: String,
    #[allow(dead_code)]
    pub parameters: RirGeoParameters,
    #[allow(dead_code)]
    pub earliest_time: String,
    #[allow(dead_code)]
    pub latest_time: String,
}

#[derive(Debug, Deserialize)]
pub struct RirGeoResource {
    pub resource: String,
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct RirGeoParameters {
    #[allow(dead_code)]
    pub resource: String,
    #[allow(dead_code)]
    pub query_time: String,
    #[allow(dead_code)]
    pub cache: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrefixesResponse {
    pub data: Option<PrefixesData>,
    pub status: String,
    #[allow(dead_code)]
    pub messages: Option<Vec<Vec<String>>>,
    #[allow(dead_code)]
    pub see_also: Option<Vec<String>>,
    #[allow(dead_code)]
    pub version: Option<String>,
    #[allow(dead_code)]
    pub data_call_name: Option<String>,
    #[allow(dead_code)]
    pub data_call_status: Option<String>,
    #[allow(dead_code)]
    pub cached: Option<bool>,
    #[allow(dead_code)]
    pub query_id: Option<String>,
    #[allow(dead_code)]
    pub process_time: Option<u32>,
    #[allow(dead_code)]
    pub server_id: Option<String>,
    #[allow(dead_code)]
    pub build_version: Option<String>,
    #[allow(dead_code)]
    pub status_code: Option<u16>,
    #[allow(dead_code)]
    pub time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrefixesData {
    pub prefixes: Option<Vec<PrefixInfo>>,
    #[allow(dead_code)]
    pub query_starttime: Option<String>,
    #[allow(dead_code)]
    pub query_endtime: Option<String>,
    #[allow(dead_code)]
    pub resource: Option<String>,
    #[allow(dead_code)]
    pub latest_time: Option<String>,
    #[allow(dead_code)]
    pub earliest_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrefixInfo {
    pub prefix: String,
    #[allow(dead_code)]
    pub timelines: Option<Vec<Timeline>>,
}

#[derive(Debug, Deserialize)]
pub struct Timeline {
    #[allow(dead_code)]
    pub starttime: Option<String>,
    #[allow(dead_code)]
    pub endtime: Option<String>,
} 