#![allow(non_snake_case)]

use super::ipinfo_api::query_ipinfo_api;
use super::meituan::MeituanCombinedResponse;
use super::types::{
    BilibiliIpResponse, IpApiResponse, IpinfoResponse, PrefixesResponse, RipeStatResponse,
    RirGeoResponse,
};
use super::utils::{extract_ip_from_prefix, truncate_string};
use anyhow::Result;

use std::fmt::Write;

use crate::log_debug;
/// Format RIR geo location response
pub fn format_rir_geo_response(resource: &str, response: &RirGeoResponse) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% RIPE NCC STAT RIR Geographic Query\n");
    formatted.push_str("% Data from RIR Statistics\n");
    writeln!(formatted, "% Query: {}", resource).unwrap();
    formatted.push('\n');

    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No RIR geographic data available\n");
            return Ok(formatted);
        }
    };

    // Display located resources
    if let Some(located) = &data.located_resources {
        if !located.is_empty() {
            formatted.push_str("RIR Geographic Location Results\n");
            formatted.push_str("===============================\n\n");
            formatted.push_str("Resource                    | Country Code\n");
            formatted.push_str("----------------------------|-------------\n");

            for item in located {
                writeln!(
                    formatted,
                    "{:<27} | {}",
                    truncate_string(&item.resource, 27),
                    item.location
                )
                .unwrap();
            }
            formatted.push('\n');

            // Summary
            writeln!(formatted, "% Total located resources: {}", located.len()).unwrap();
        }
    } else {
        formatted.push_str("% No located resources found\n");
    }

    // Show messages if any
    if let Some(messages) = &response.messages
        && !messages.is_empty()
    {
        formatted.push_str("\n% API Messages:\n");
        for message in messages {
            for msg_part in message {
                writeln!(formatted, "% {}", msg_part).unwrap();
            }
        }
    }

    Ok(formatted)
}

/// Format ultimate geo location response from all available APIs
pub fn format_ultimate_geo_response(
    resource: &str,
    ripe_result: Result<RipeStatResponse>,
    ipinfo_result: Result<IpinfoResponse>,
    ipapi_result: Result<IpApiResponse>,
    bilibili_result: Result<BilibiliIpResponse>,
    meituan_result: Result<MeituanCombinedResponse>,
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Ultimate Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT, IPinfo, IP-API, BiliBili, and Meituan\n");
    writeln!(formatted, "% Query: {}", resource).unwrap();
    formatted.push('\n');

    // RIPE NCC STAT section
    formatted.push_str("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
    match ripe_result {
        Ok(ripe_response) => {
            if let Some(data) = &ripe_response.data {
                if let Some(located) = &data.located_resources {
                    if !located.is_empty() {
                        for item in located {
                            writeln!(formatted, "Resource:  {}", item.resource).unwrap();

                            if let Some(locations) = &item.locations {
                                for location in locations {
                                    let country = location.country.as_deref().unwrap_or("N/A");
                                    let city = location.city.as_deref().unwrap_or("N/A");

                                    writeln!(formatted, "Country:   {}", country).unwrap();
                                    writeln!(formatted, "City:      {}", city).unwrap();

                                    if let (Some(lat), Some(lon)) =
                                        (location.latitude, location.longitude)
                                    {
                                        writeln!(formatted, "Location:  {:.4}, {:.4}", lat, lon)
                                            .unwrap();
                                    } else {
                                        formatted.push_str("Location:  N/A\n");
                                    }
                                }
                            } else {
                                formatted.push_str("Country:   N/A\n");
                                formatted.push_str("City:      N/A\n");
                                formatted.push_str("Location:  N/A\n");
                            }
                        }
                    } else {
                        formatted.push_str("% No location data available\n");
                    }
                } else {
                    formatted.push_str("% No location data available\n");
                }
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match &ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");

            writeln!(formatted, "Resource: {}", ipinfo_response.ip).unwrap();
            writeln!(formatted, "Country:  {}", country).unwrap();
            writeln!(formatted, "City:     {}", city).unwrap();
            writeln!(formatted, "ASN:      {}", asn).unwrap();
            writeln!(formatted, "AS Name:  {}", as_name).unwrap();

            if let Some(continent) = &ipinfo_response.continent {
                writeln!(formatted, "Continent: {}", continent).unwrap();
            }
            if let Some(region) = &ipinfo_response.region {
                writeln!(formatted, "Region:    {}", region).unwrap();
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                writeln!(formatted, "AS Domain: {}", as_domain).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // IP-API section
    formatted.push_str("=== IP-API ===\n");
    match &ipapi_result {
        Ok(ipapi_response) => {
            writeln!(formatted, "Resource:  {}", ipapi_response.ip).unwrap();
            writeln!(
                formatted,
                "Country:   {} ({})",
                ipapi_response.country.as_deref().unwrap_or("N/A"),
                ipapi_response.country_code.as_deref().unwrap_or("N/A")
            )
            .unwrap();
            writeln!(
                formatted,
                "Region:    {} ({})",
                ipapi_response.region_name.as_deref().unwrap_or("N/A"),
                ipapi_response.region.as_deref().unwrap_or("N/A")
            )
            .unwrap();
            writeln!(
                formatted,
                "City:      {}",
                ipapi_response.city.as_deref().unwrap_or("N/A")
            )
            .unwrap();

            if let (Some(lat), Some(lon)) = (ipapi_response.lat, ipapi_response.lon) {
                writeln!(formatted, "Location:  {:.4}, {:.4}", lat, lon).unwrap();
            }

            if let Some(zip) = &ipapi_response.zip {
                writeln!(formatted, "ZIP:       {}", zip).unwrap();
            }
            if let Some(timezone) = &ipapi_response.timezone {
                writeln!(formatted, "Timezone:  {}", timezone).unwrap();
            }
            if let Some(isp) = &ipapi_response.isp {
                writeln!(formatted, "ISP:       {}", isp).unwrap();
            }
            if let Some(org) = &ipapi_response.org {
                writeln!(formatted, "Org:       {}", org).unwrap();
            }
            if let Some(asn_info) = &ipapi_response.asn_info {
                writeln!(formatted, "ASN Info:  {}", asn_info).unwrap();
            }

            // Additional flags
            if let Some(mobile) = ipapi_response.mobile {
                writeln!(formatted, "Mobile:    {}", mobile).unwrap();
            }
            if let Some(proxy) = ipapi_response.proxy {
                writeln!(formatted, "Proxy:     {}", proxy).unwrap();
            }
            if let Some(hosting) = ipapi_response.hosting {
                writeln!(formatted, "Hosting:   {}", hosting).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // BiliBili section
    formatted.push_str("=== BiliBili ===\n");
    match &bilibili_result {
        Ok(bilibili_response) => {
            if let Some(data) = &bilibili_response.data {
                writeln!(formatted, "Address:   {}", data.addr).unwrap();
                writeln!(formatted, "Country:   {}", data.country).unwrap();
                writeln!(formatted, "Province:  {}", data.province).unwrap();
                writeln!(formatted, "City:      {}", data.city).unwrap();
                writeln!(formatted, "ISP:       {}", data.isp).unwrap();
                writeln!(
                    formatted,
                    "Location:  {}, {}",
                    data.latitude, data.longitude
                )
                .unwrap();
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // Meituan section
    formatted.push_str("=== Meituan ===\n");
    match &meituan_result {
        Ok(meituan_response) => {
            writeln!(formatted, "Resource:  {}", meituan_response.ip).unwrap();
            writeln!(formatted, "Country:   {}", meituan_response.country).unwrap();
            writeln!(formatted, "Province:  {}", meituan_response.province).unwrap();
            writeln!(formatted, "City:      {}", meituan_response.city).unwrap();
            writeln!(formatted, "District:  {}", meituan_response.district).unwrap();
            writeln!(formatted, "AD Code:   {}", meituan_response.adcode).unwrap();
            writeln!(
                formatted,
                "Location:  {:.2}, {:.2}",
                meituan_response.lat, meituan_response.lng
            )
            .unwrap();
            writeln!(formatted, "Source:    {}", meituan_response.fromwhere).unwrap();

            // Add city details if available
            if let Some(city_details) = &meituan_response.city_details {
                formatted.push_str("\n--- City Details ---\n");
                writeln!(formatted, "City ID:      {}", city_details.id).unwrap();
                writeln!(formatted, "City (EN):    {}", city_details.city_pinyin).unwrap();
                writeln!(formatted, "Open City:    {}", city_details.open_city_name).unwrap();
                writeln!(formatted, "Area Name:    {}", city_details.area_name).unwrap();
                writeln!(formatted, "Detail:       {}", city_details.detail).unwrap();
                writeln!(formatted, "Is Open:      {}", city_details.is_open).unwrap();
                writeln!(formatted, "Is Foreign:   {}", city_details.is_foreign).unwrap();
                writeln!(formatted, "DP City ID:   {}", city_details.dp_city_id).unwrap();
                writeln!(formatted, "Area Code:    {}", city_details.area).unwrap();
                writeln!(formatted, "Parent Area:  {}", city_details.parent_area).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    Ok(formatted)
}

/// Format enhanced geo location response from multiple APIs (4 APIs - legacy)
#[allow(dead_code)]
pub fn format_enhanced_geo_response(
    resource: &str,
    ripe_result: Result<RipeStatResponse>,
    ipinfo_result: Result<IpinfoResponse>,
    ipapi_result: Result<IpApiResponse>,
    bilibili_result: Result<BilibiliIpResponse>,
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Enhanced Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT, IPinfo, IP-API, and BiliBili\n");
    writeln!(formatted, "% Query: {}", resource).unwrap();
    formatted.push('\n');

    // RIPE NCC STAT section
    formatted.push_str("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
    match ripe_result {
        Ok(ripe_response) => {
            if let Some(data) = &ripe_response.data {
                if let Some(located) = &data.located_resources {
                    if !located.is_empty() {
                        // Use existing RIPE formatting logic
                        let mut rows = Vec::new();
                        let mut max_resource_len = 8;
                        let mut max_country_len = 7;
                        let mut max_city_len = 4;

                        for item in located {
                            if let Some(locations) = &item.locations {
                                for location in locations {
                                    let country = location.country.as_deref().unwrap_or("N/A");
                                    let city = location.city.as_deref().unwrap_or("N/A");
                                    let lat = location
                                        .latitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());
                                    let lon = location
                                        .longitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());

                                    max_resource_len =
                                        std::cmp::max(max_resource_len, item.resource.len());
                                    max_country_len = std::cmp::max(max_country_len, country.len());
                                    max_city_len = std::cmp::max(max_city_len, city.len());

                                    rows.push((
                                        item.resource.clone(),
                                        country.to_string(),
                                        city.to_string(),
                                        lat,
                                        lon,
                                    ));
                                }
                            } else {
                                max_resource_len =
                                    std::cmp::max(max_resource_len, item.resource.len());
                                rows.push((
                                    item.resource.clone(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                ));
                            }
                        }

                        writeln!(
                            formatted,
                            "{:<width1$} | {:<width2$} | {:<width3$} | Latitude  | Longitude",
                            "Resource",
                            "Country",
                            "City",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        )
                        .unwrap();

                        writeln!(
                            formatted,
                            "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-----------|----------",
                            "",
                            "",
                            "",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        )
                        .unwrap();

                        for (resource, country, city, lat, lon) in rows {
                            writeln!(
                                formatted,
                                "{:<width1$} | {:<width2$} | {:<width3$} | {:<9} | {}",
                                resource,
                                country,
                                city,
                                lat,
                                lon,
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                            .unwrap();
                        }
                    } else {
                        formatted.push_str("% No location data available\n");
                    }
                } else {
                    formatted.push_str("% No location data available\n");
                }
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match &ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");

            writeln!(formatted, "Resource: {}", ipinfo_response.ip).unwrap();
            writeln!(formatted, "Country:  {}", country).unwrap();
            writeln!(formatted, "City:     {}", city).unwrap();
            writeln!(formatted, "ASN:      {}", asn).unwrap();
            writeln!(formatted, "AS Name:  {}", as_name).unwrap();

            if let Some(continent) = &ipinfo_response.continent {
                writeln!(formatted, "Continent: {}", continent).unwrap();
            }
            if let Some(region) = &ipinfo_response.region {
                writeln!(formatted, "Region:    {}", region).unwrap();
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                writeln!(formatted, "AS Domain: {}", as_domain).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // IP-API section
    formatted.push_str("=== IP-API ===\n");
    match &ipapi_result {
        Ok(ipapi_response) => {
            writeln!(formatted, "Resource:  {}", ipapi_response.ip).unwrap();
            writeln!(
                formatted,
                "Country:   {} ({})",
                ipapi_response.country.as_deref().unwrap_or("N/A"),
                ipapi_response.country_code.as_deref().unwrap_or("N/A")
            )
            .unwrap();
            writeln!(
                formatted,
                "Region:    {} ({})",
                ipapi_response.region_name.as_deref().unwrap_or("N/A"),
                ipapi_response.region.as_deref().unwrap_or("N/A")
            )
            .unwrap();
            writeln!(
                formatted,
                "City:      {}",
                ipapi_response.city.as_deref().unwrap_or("N/A")
            )
            .unwrap();

            if let (Some(lat), Some(lon)) = (ipapi_response.lat, ipapi_response.lon) {
                writeln!(formatted, "Location:  {:.4}, {:.4}", lat, lon).unwrap();
            }

            if let Some(zip) = &ipapi_response.zip {
                writeln!(formatted, "ZIP:       {}", zip).unwrap();
            }
            if let Some(timezone) = &ipapi_response.timezone {
                writeln!(formatted, "Timezone:  {}", timezone).unwrap();
            }
            if let Some(isp) = &ipapi_response.isp {
                writeln!(formatted, "ISP:       {}", isp).unwrap();
            }
            if let Some(org) = &ipapi_response.org {
                writeln!(formatted, "Org:       {}", org).unwrap();
            }
            if let Some(asn_info) = &ipapi_response.asn_info {
                writeln!(formatted, "ASN Info:  {}", asn_info).unwrap();
            }

            // Additional flags
            if let Some(mobile) = ipapi_response.mobile {
                writeln!(formatted, "Mobile:    {}", mobile).unwrap();
            }
            if let Some(proxy) = ipapi_response.proxy {
                writeln!(formatted, "Proxy:     {}", proxy).unwrap();
            }
            if let Some(hosting) = ipapi_response.hosting {
                writeln!(formatted, "Hosting:   {}", hosting).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // BiliBili section
    formatted.push_str("=== BiliBili ===\n");
    match &bilibili_result {
        Ok(bilibili_response) => {
            if let Some(data) = &bilibili_response.data {
                writeln!(formatted, "Address:   {}", data.addr).unwrap();
                writeln!(formatted, "Country:   {}", data.country).unwrap();
                writeln!(formatted, "Province:  {}", data.province).unwrap();
                writeln!(formatted, "City:      {}", data.city).unwrap();
                writeln!(formatted, "ISP:       {}", data.isp).unwrap();
                writeln!(
                    formatted,
                    "Location:  {}, {}",
                    data.latitude, data.longitude
                )
                .unwrap();
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    Ok(formatted)
}

/// Format combined geo location response from both APIs (legacy function for compatibility)
#[allow(dead_code)]
pub fn format_combined_geo_response(
    resource: &str,
    ripe_result: Result<RipeStatResponse>,
    ipinfo_result: Result<IpinfoResponse>,
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT (MaxMind GeoLite2) and IPinfo\n");
    writeln!(formatted, "% Query: {}", resource).unwrap();
    formatted.push('\n');

    // RIPE NCC STAT section
    formatted.push_str("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
    match ripe_result {
        Ok(ripe_response) => {
            if let Some(data) = &ripe_response.data {
                if let Some(located) = &data.located_resources {
                    if !located.is_empty() {
                        // Collect all data first to calculate column widths
                        let mut rows = Vec::new();
                        let mut max_resource_len = 8; // "Resource"
                        let mut max_country_len = 7; // "Country"
                        let mut max_city_len = 4; // "City"

                        for item in located {
                            if let Some(locations) = &item.locations {
                                for location in locations {
                                    let country = location.country.as_deref().unwrap_or("N/A");
                                    let city = location.city.as_deref().unwrap_or("N/A");
                                    let lat = location
                                        .latitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());
                                    let lon = location
                                        .longitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());

                                    max_resource_len =
                                        std::cmp::max(max_resource_len, item.resource.len());
                                    max_country_len = std::cmp::max(max_country_len, country.len());
                                    max_city_len = std::cmp::max(max_city_len, city.len());

                                    rows.push((
                                        item.resource.clone(),
                                        country.to_string(),
                                        city.to_string(),
                                        lat,
                                        lon,
                                    ));
                                }
                            } else {
                                max_resource_len =
                                    std::cmp::max(max_resource_len, item.resource.len());
                                rows.push((
                                    item.resource.clone(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                ));
                            }
                        }

                        // Dynamic header
                        writeln!(
                            formatted,
                            "{:<width1$} | {:<width2$} | {:<width3$} | Latitude  | Longitude",
                            "Resource",
                            "Country",
                            "City",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        )
                        .unwrap();

                        // Dynamic separator
                        writeln!(
                            formatted,
                            "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-----------|----------",
                            "",
                            "",
                            "",
                            width1 = max_resource_len,
                            width2 = max_country_len,
                            width3 = max_city_len
                        )
                        .unwrap();

                        // Data rows
                        for (resource, country, city, lat, lon) in rows {
                            writeln!(
                                formatted,
                                "{:<width1$} | {:<width2$} | {:<width3$} | {:<9} | {}",
                                resource,
                                country,
                                city,
                                lat,
                                lon,
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                            .unwrap();
                        }
                    } else {
                        formatted.push_str("% No location data available\n");
                    }
                } else {
                    formatted.push_str("% No location data available\n");
                }
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");

            // Calculate dynamic column widths
            let resource_width = std::cmp::max(8, ipinfo_response.ip.len());
            let country_width = std::cmp::max(7, country.len());
            let city_width = std::cmp::max(4, city.len());
            let asn_width = std::cmp::max(3, asn.len());

            // Dynamic header
            writeln!(
                formatted,
                "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | AS Name",
                "Resource",
                "Country",
                "City",
                "ASN",
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            )
            .unwrap();

            // Dynamic separator
            writeln!(
                formatted,
                "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-{:-<width4$}-|----------",
                "",
                "",
                "",
                "",
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            )
            .unwrap();

            // Data row with dynamic widths
            writeln!(
                formatted,
                "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | {}",
                ipinfo_response.ip,
                country,
                city,
                asn,
                as_name,
                width1 = resource_width,
                width2 = country_width,
                width3 = city_width,
                width4 = asn_width
            )
            .unwrap();

            // Additional info if available
            if let Some(continent) = &ipinfo_response.continent {
                writeln!(formatted, "% Continent: {}", continent).unwrap();
            }
            if let Some(region) = &ipinfo_response.region {
                writeln!(formatted, "% Region: {}", region).unwrap();
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                writeln!(formatted, "% AS Domain: {}", as_domain).unwrap();
            }
        }
        Err(e) => {
            writeln!(formatted, "% Error: {}", e).unwrap();
        }
    }

    formatted.push('\n');

    Ok(formatted)
}

/// Format prefixes response with country and AS name information from IPinfo
pub async fn format_prefixes_response(
    asn: &str,
    response: &PrefixesResponse,
    client: &reqwest::Client,
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% ASN Announced Prefixes Query\n");
    formatted.push_str("% Data from RIPE NCC STAT\n");
    writeln!(formatted, "% Query: {}", asn).unwrap();
    formatted.push('\n');

    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No prefixes data available\n");
            return Ok(formatted);
        }
    };

    if let Some(prefixes) = &data.prefixes {
        if !prefixes.is_empty() {
            log_debug!(
                "Processing {} prefixes with concurrent IPinfo queries",
                prefixes.len()
            );

            // Create semaphore to limit concurrent requests to 32
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(32));

            // Collect prefix information with country and AS name data using concurrent queries
            let mut tasks = Vec::new();

            for prefix_info in prefixes {
                let prefix = prefix_info.prefix.clone();
                let ip_addr = extract_ip_from_prefix(&prefix);
                let client = client.clone();
                let permit = semaphore.clone();

                let task = tokio::spawn(async move {
                    // Acquire semaphore permit to limit concurrency
                    let _permit = permit.acquire().await.ok();

                    log_debug!(
                        "Querying IPinfo for IP: {} (from prefix: {})",
                        ip_addr,
                        prefix
                    );

                    let (country, as_name) = match query_ipinfo_api(&client, &ip_addr).await {
                        Ok(ipinfo_response) => {
                            log_debug!(
                                "IPinfo response for {}: as_name={:?}, country={:?}",
                                ip_addr,
                                ipinfo_response.as_name,
                                ipinfo_response.country
                            );
                            let country = ipinfo_response
                                .country
                                .as_deref()
                                .unwrap_or("N/A")
                                .to_string();
                            let as_name = ipinfo_response
                                .as_name
                                .as_deref()
                                .unwrap_or("N/A")
                                .to_string();
                            (country, as_name)
                        }
                        Err(e) => {
                            log_debug!("IPinfo query failed for {}: {}", ip_addr, e);
                            ("N/A".to_string(), "N/A".to_string())
                        }
                    };

                    (prefix, country, as_name)
                });

                tasks.push(task);
            }

            // Wait for all tasks to complete and collect results
            let mut prefix_data = Vec::new();
            for task in tasks {
                match task.await {
                    Ok(result) => prefix_data.push(result),
                    Err(e) => {
                        log_debug!("Task join error: {}", e);
                        // Continue with other results
                    }
                }
            }

            log_debug!(
                "Completed IPinfo queries for {} prefixes",
                prefix_data.len()
            );

            // Calculate adaptive column widths (single-pass)
            let (prefix_width, country_width, as_name_width) = prefix_data
                .iter()
                .fold((6, 7, 7), |(p, c, a), (pp, cc, aa)| {
                    (p.max(pp.len()), c.max(cc.len()), a.max(aa.len()))
                });

            formatted.push_str("Currently Announced Prefixes\n");
            formatted.push_str("============================\n\n");

            // Dynamic header
            writeln!(
                formatted,
                "{:<width1$} | {:<width2$} | {:<width3$}",
                "Prefix",
                "Country",
                "AS Name",
                width1 = prefix_width,
                width2 = country_width,
                width3 = as_name_width
            )
            .unwrap();

            // Dynamic separator
            writeln!(
                formatted,
                "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}",
                "",
                "",
                "",
                width1 = prefix_width,
                width2 = country_width,
                width3 = as_name_width
            )
            .unwrap();

            // Data rows
            for (prefix, country, as_name) in prefix_data {
                writeln!(
                    formatted,
                    "{:<width1$} | {:<width2$} | {:<width3$}",
                    truncate_string(&prefix, prefix_width),
                    truncate_string(&country, country_width),
                    truncate_string(&as_name, as_name_width),
                    width1 = prefix_width,
                    width2 = country_width,
                    width3 = as_name_width
                )
                .unwrap();
            }

            write!(
                formatted,
                "\n% Total announced prefixes: {}\n",
                prefixes.len()
            )
            .unwrap();
        } else {
            formatted.push_str("% No announced prefixes found\n");
        }
    } else {
        formatted.push_str("% No prefixes data available\n");
    }

    // Show messages if any
    if let Some(messages) = &response.messages
        && !messages.is_empty()
    {
        formatted.push_str("\n% API Messages:\n");
        for message in messages {
            for msg_part in message {
                writeln!(formatted, "% {}", msg_part).unwrap();
            }
        }
    }

    Ok(formatted)
}
