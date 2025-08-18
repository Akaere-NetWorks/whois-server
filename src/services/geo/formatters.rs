use anyhow::Result;
use tracing::debug;

use super::types::{
    RipeStatResponse,
    RirGeoResponse,
    PrefixesResponse,
    IpinfoResponse,
    IpApiResponse,
    BilibiliIpResponse,
};
use super::meituan::MeituanCombinedResponse;
use super::utils::{ truncate_string, extract_ip_from_prefix };
use super::ipinfo_api::{ query_ipinfo_api, query_ipinfo_api_blocking };

/// Format RIR geo location response
pub fn format_rir_geo_response(resource: &str, response: &RirGeoResponse) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% RIPE NCC STAT RIR Geographic Query\n");
    formatted.push_str("% Data from RIR Statistics\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");

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
                formatted.push_str(
                    &format!("{:<27} | {}\n", truncate_string(&item.resource, 27), item.location)
                );
            }
            formatted.push_str("\n");

            // Summary
            formatted.push_str(&format!("% Total located resources: {}\n", located.len()));
        }
    } else {
        formatted.push_str("% No located resources found\n");
    }

    // Show messages if any
    if let Some(messages) = &response.messages {
        if !messages.is_empty() {
            formatted.push_str("\n% API Messages:\n");
            for message in messages {
                for msg_part in message {
                    formatted.push_str(&format!("% {}\n", msg_part));
                }
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
    meituan_result: Result<MeituanCombinedResponse>
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Ultimate Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT, IPinfo, IP-API, BiliBili, and Meituan\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");

    // RIPE NCC STAT section
    formatted.push_str("=== RIPE NCC STAT (MaxMind GeoLite2) ===\n");
    match ripe_result {
        Ok(ripe_response) => {
            if let Some(data) = &ripe_response.data {
                if let Some(located) = &data.located_resources {
                    if !located.is_empty() {
                        for item in located {
                            formatted.push_str(&format!("Resource:  {}\n", item.resource));

                            if let Some(locations) = &item.locations {
                                for location in locations {
                                    let country = location.country.as_deref().unwrap_or("N/A");
                                    let city = location.city.as_deref().unwrap_or("N/A");

                                    formatted.push_str(&format!("Country:   {}\n", country));
                                    formatted.push_str(&format!("City:      {}\n", city));

                                    if
                                        let (Some(lat), Some(lon)) = (
                                            location.latitude,
                                            location.longitude,
                                        )
                                    {
                                        formatted.push_str(
                                            &format!("Location:  {:.4}, {:.4}\n", lat, lon)
                                        );
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
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match &ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");

            formatted.push_str(&format!("Resource: {}\n", ipinfo_response.ip));
            formatted.push_str(&format!("Country:  {}\n", country));
            formatted.push_str(&format!("City:     {}\n", city));
            formatted.push_str(&format!("ASN:      {}\n", asn));
            formatted.push_str(&format!("AS Name:  {}\n", as_name));

            if let Some(continent) = &ipinfo_response.continent {
                formatted.push_str(&format!("Continent: {}\n", continent));
            }
            if let Some(region) = &ipinfo_response.region {
                formatted.push_str(&format!("Region:    {}\n", region));
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                formatted.push_str(&format!("AS Domain: {}\n", as_domain));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // IP-API section
    formatted.push_str("=== IP-API ===\n");
    match &ipapi_result {
        Ok(ipapi_response) => {
            formatted.push_str(&format!("Resource:  {}\n", ipapi_response.ip));
            formatted.push_str(
                &format!(
                    "Country:   {} ({})\n",
                    ipapi_response.country.as_deref().unwrap_or("N/A"),
                    ipapi_response.country_code.as_deref().unwrap_or("N/A")
                )
            );
            formatted.push_str(
                &format!(
                    "Region:    {} ({})\n",
                    ipapi_response.region_name.as_deref().unwrap_or("N/A"),
                    ipapi_response.region.as_deref().unwrap_or("N/A")
                )
            );
            formatted.push_str(
                &format!("City:      {}\n", ipapi_response.city.as_deref().unwrap_or("N/A"))
            );

            if let (Some(lat), Some(lon)) = (ipapi_response.lat, ipapi_response.lon) {
                formatted.push_str(&format!("Location:  {:.4}, {:.4}\n", lat, lon));
            }

            if let Some(zip) = &ipapi_response.zip {
                formatted.push_str(&format!("ZIP:       {}\n", zip));
            }
            if let Some(timezone) = &ipapi_response.timezone {
                formatted.push_str(&format!("Timezone:  {}\n", timezone));
            }
            if let Some(isp) = &ipapi_response.isp {
                formatted.push_str(&format!("ISP:       {}\n", isp));
            }
            if let Some(org) = &ipapi_response.org {
                formatted.push_str(&format!("Org:       {}\n", org));
            }
            if let Some(asn_info) = &ipapi_response.asn_info {
                formatted.push_str(&format!("ASN Info:  {}\n", asn_info));
            }

            // Additional flags
            if let Some(mobile) = ipapi_response.mobile {
                formatted.push_str(&format!("Mobile:    {}\n", mobile));
            }
            if let Some(proxy) = ipapi_response.proxy {
                formatted.push_str(&format!("Proxy:     {}\n", proxy));
            }
            if let Some(hosting) = ipapi_response.hosting {
                formatted.push_str(&format!("Hosting:   {}\n", hosting));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // BiliBili section
    formatted.push_str("=== BiliBili ===\n");
    match &bilibili_result {
        Ok(bilibili_response) => {
            if let Some(data) = &bilibili_response.data {
                formatted.push_str(&format!("Address:   {}\n", data.addr));
                formatted.push_str(&format!("Country:   {}\n", data.country));
                formatted.push_str(&format!("Province:  {}\n", data.province));
                formatted.push_str(&format!("City:      {}\n", data.city));
                formatted.push_str(&format!("ISP:       {}\n", data.isp));
                formatted.push_str(&format!("Location:  {}, {}\n", data.latitude, data.longitude));
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // Meituan section
    formatted.push_str("=== Meituan ===\n");
    match &meituan_result {
        Ok(meituan_response) => {
            formatted.push_str(&format!("Resource:  {}\n", meituan_response.ip));
            formatted.push_str(&format!("Country:   {}\n", meituan_response.country));
            formatted.push_str(&format!("Province:  {}\n", meituan_response.province));
            formatted.push_str(&format!("City:      {}\n", meituan_response.city));
            formatted.push_str(&format!("District:  {}\n", meituan_response.district));
            formatted.push_str(&format!("AD Code:   {}\n", meituan_response.adcode));
            formatted.push_str(
                &format!("Location:  {:.2}, {:.2}\n", meituan_response.lat, meituan_response.lng)
            );
            formatted.push_str(&format!("Source:    {}\n", meituan_response.fromwhere));

            // Add city details if available
            if let Some(city_details) = &meituan_response.city_details {
                formatted.push_str("\n--- City Details ---\n");
                formatted.push_str(&format!("City ID:      {}\n", city_details.id));
                formatted.push_str(&format!("City (EN):    {}\n", city_details.city_pinyin));
                formatted.push_str(&format!("Open City:    {}\n", city_details.open_city_name));
                formatted.push_str(&format!("Area Name:    {}\n", city_details.area_name));
                formatted.push_str(&format!("Detail:       {}\n", city_details.detail));
                formatted.push_str(&format!("Is Open:      {}\n", city_details.is_open));
                formatted.push_str(&format!("Is Foreign:   {}\n", city_details.is_foreign));
                formatted.push_str(&format!("DP City ID:   {}\n", city_details.dp_city_id));
                formatted.push_str(&format!("Area Code:    {}\n", city_details.area));
                formatted.push_str(&format!("Parent Area:  {}\n", city_details.parent_area));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    Ok(formatted)
}

/// Format enhanced geo location response from multiple APIs (4 APIs - legacy)
#[allow(dead_code)]
pub fn format_enhanced_geo_response(
    resource: &str,
    ripe_result: Result<RipeStatResponse>,
    ipinfo_result: Result<IpinfoResponse>,
    ipapi_result: Result<IpApiResponse>,
    bilibili_result: Result<BilibiliIpResponse>
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Enhanced Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT, IPinfo, IP-API, and BiliBili\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");

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
                                    let lat = location.latitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());
                                    let lon = location.longitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());

                                    max_resource_len = std::cmp::max(
                                        max_resource_len,
                                        item.resource.len()
                                    );
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
                                max_resource_len = std::cmp::max(
                                    max_resource_len,
                                    item.resource.len()
                                );
                                rows.push((
                                    item.resource.clone(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                    "N/A".to_string(),
                                ));
                            }
                        }

                        formatted.push_str(
                            &format!(
                                "{:<width1$} | {:<width2$} | {:<width3$} | Latitude  | Longitude\n",
                                "Resource",
                                "Country",
                                "City",
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                        );

                        formatted.push_str(
                            &format!(
                                "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-----------|----------\n",
                                "",
                                "",
                                "",
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                        );

                        for (resource, country, city, lat, lon) in rows {
                            formatted.push_str(
                                &format!(
                                    "{:<width1$} | {:<width2$} | {:<width3$} | {:<9} | {}\n",
                                    resource,
                                    country,
                                    city,
                                    lat,
                                    lon,
                                    width1 = max_resource_len,
                                    width2 = max_country_len,
                                    width3 = max_city_len
                                )
                            );
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
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // IPinfo section
    formatted.push_str("=== IPinfo ===\n");
    match &ipinfo_result {
        Ok(ipinfo_response) => {
            let country = ipinfo_response.country.as_deref().unwrap_or("N/A");
            let city = ipinfo_response.city.as_deref().unwrap_or("N/A");
            let asn = ipinfo_response.asn.as_deref().unwrap_or("N/A");
            let as_name = ipinfo_response.as_name.as_deref().unwrap_or("N/A");

            formatted.push_str(&format!("Resource: {}\n", ipinfo_response.ip));
            formatted.push_str(&format!("Country:  {}\n", country));
            formatted.push_str(&format!("City:     {}\n", city));
            formatted.push_str(&format!("ASN:      {}\n", asn));
            formatted.push_str(&format!("AS Name:  {}\n", as_name));

            if let Some(continent) = &ipinfo_response.continent {
                formatted.push_str(&format!("Continent: {}\n", continent));
            }
            if let Some(region) = &ipinfo_response.region {
                formatted.push_str(&format!("Region:    {}\n", region));
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                formatted.push_str(&format!("AS Domain: {}\n", as_domain));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // IP-API section
    formatted.push_str("=== IP-API ===\n");
    match &ipapi_result {
        Ok(ipapi_response) => {
            formatted.push_str(&format!("Resource:  {}\n", ipapi_response.ip));
            formatted.push_str(
                &format!(
                    "Country:   {} ({})\n",
                    ipapi_response.country.as_deref().unwrap_or("N/A"),
                    ipapi_response.country_code.as_deref().unwrap_or("N/A")
                )
            );
            formatted.push_str(
                &format!(
                    "Region:    {} ({})\n",
                    ipapi_response.region_name.as_deref().unwrap_or("N/A"),
                    ipapi_response.region.as_deref().unwrap_or("N/A")
                )
            );
            formatted.push_str(
                &format!("City:      {}\n", ipapi_response.city.as_deref().unwrap_or("N/A"))
            );

            if let (Some(lat), Some(lon)) = (ipapi_response.lat, ipapi_response.lon) {
                formatted.push_str(&format!("Location:  {:.4}, {:.4}\n", lat, lon));
            }

            if let Some(zip) = &ipapi_response.zip {
                formatted.push_str(&format!("ZIP:       {}\n", zip));
            }
            if let Some(timezone) = &ipapi_response.timezone {
                formatted.push_str(&format!("Timezone:  {}\n", timezone));
            }
            if let Some(isp) = &ipapi_response.isp {
                formatted.push_str(&format!("ISP:       {}\n", isp));
            }
            if let Some(org) = &ipapi_response.org {
                formatted.push_str(&format!("Org:       {}\n", org));
            }
            if let Some(asn_info) = &ipapi_response.asn_info {
                formatted.push_str(&format!("ASN Info:  {}\n", asn_info));
            }

            // Additional flags
            if let Some(mobile) = ipapi_response.mobile {
                formatted.push_str(&format!("Mobile:    {}\n", mobile));
            }
            if let Some(proxy) = ipapi_response.proxy {
                formatted.push_str(&format!("Proxy:     {}\n", proxy));
            }
            if let Some(hosting) = ipapi_response.hosting {
                formatted.push_str(&format!("Hosting:   {}\n", hosting));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    // BiliBili section
    formatted.push_str("=== BiliBili ===\n");
    match &bilibili_result {
        Ok(bilibili_response) => {
            if let Some(data) = &bilibili_response.data {
                formatted.push_str(&format!("Address:   {}\n", data.addr));
                formatted.push_str(&format!("Country:   {}\n", data.country));
                formatted.push_str(&format!("Province:  {}\n", data.province));
                formatted.push_str(&format!("City:      {}\n", data.city));
                formatted.push_str(&format!("ISP:       {}\n", data.isp));
                formatted.push_str(&format!("Location:  {}, {}\n", data.latitude, data.longitude));
            } else {
                formatted.push_str("% No data available\n");
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    Ok(formatted)
}

/// Format combined geo location response from both APIs (legacy function for compatibility)
#[allow(dead_code)]
pub fn format_combined_geo_response(
    resource: &str,
    ripe_result: Result<RipeStatResponse>,
    ipinfo_result: Result<IpinfoResponse>
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% Multi-Source Geo Location Query\n");
    formatted.push_str("% Data from RIPE NCC STAT (MaxMind GeoLite2) and IPinfo\n");
    formatted.push_str(&format!("% Query: {}\n", resource));
    formatted.push_str("\n");

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
                                    let lat = location.latitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());
                                    let lon = location.longitude
                                        .map(|f| format!("{:.4}", f))
                                        .unwrap_or_else(|| "N/A".to_string());

                                    max_resource_len = std::cmp::max(
                                        max_resource_len,
                                        item.resource.len()
                                    );
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
                                max_resource_len = std::cmp::max(
                                    max_resource_len,
                                    item.resource.len()
                                );
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
                        formatted.push_str(
                            &format!(
                                "{:<width1$} | {:<width2$} | {:<width3$} | Latitude  | Longitude\n",
                                "Resource",
                                "Country",
                                "City",
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                        );

                        // Dynamic separator
                        formatted.push_str(
                            &format!(
                                "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-----------|----------\n",
                                "",
                                "",
                                "",
                                width1 = max_resource_len,
                                width2 = max_country_len,
                                width3 = max_city_len
                            )
                        );

                        // Data rows
                        for (resource, country, city, lat, lon) in rows {
                            formatted.push_str(
                                &format!(
                                    "{:<width1$} | {:<width2$} | {:<width3$} | {:<9} | {}\n",
                                    resource,
                                    country,
                                    city,
                                    lat,
                                    lon,
                                    width1 = max_resource_len,
                                    width2 = max_country_len,
                                    width3 = max_city_len
                                )
                            );
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
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

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
            formatted.push_str(
                &format!(
                    "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | AS Name\n",
                    "Resource",
                    "Country",
                    "City",
                    "ASN",
                    width1 = resource_width,
                    width2 = country_width,
                    width3 = city_width,
                    width4 = asn_width
                )
            );

            // Dynamic separator
            formatted.push_str(
                &format!(
                    "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}-|-{:-<width4$}-|----------\n",
                    "",
                    "",
                    "",
                    "",
                    width1 = resource_width,
                    width2 = country_width,
                    width3 = city_width,
                    width4 = asn_width
                )
            );

            // Data row with dynamic widths
            formatted.push_str(
                &format!(
                    "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | {}\n",
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
            );

            // Additional info if available
            if let Some(continent) = &ipinfo_response.continent {
                formatted.push_str(&format!("% Continent: {}\n", continent));
            }
            if let Some(region) = &ipinfo_response.region {
                formatted.push_str(&format!("% Region: {}\n", region));
            }
            if let Some(as_domain) = &ipinfo_response.as_domain {
                formatted.push_str(&format!("% AS Domain: {}\n", as_domain));
            }
        }
        Err(e) => {
            formatted.push_str(&format!("% Error: {}\n", e));
        }
    }

    formatted.push_str("\n");

    Ok(formatted)
}

/// Format prefixes response with country and AS name information from IPinfo
pub async fn format_prefixes_response(
    asn: &str,
    response: &PrefixesResponse,
    client: &reqwest::Client
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% ASN Announced Prefixes Query\n");
    formatted.push_str("% Data from RIPE NCC STAT\n");
    formatted.push_str(&format!("% Query: {}\n", asn));
    formatted.push_str("\n");

    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No prefixes data available\n");
            return Ok(formatted);
        }
    };

    if let Some(prefixes) = &data.prefixes {
        if !prefixes.is_empty() {
            debug!("Processing {} prefixes with concurrent IPinfo queries", prefixes.len());

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
                    let _permit = permit.acquire().await.unwrap();

                    debug!("Querying IPinfo for IP: {} (from prefix: {})", ip_addr, prefix);

                    let (country, as_name) = match query_ipinfo_api(&client, &ip_addr).await {
                        Ok(ipinfo_response) => {
                            debug!(
                                "IPinfo response for {}: as_name={:?}, country={:?}",
                                ip_addr,
                                ipinfo_response.as_name,
                                ipinfo_response.country
                            );
                            let country = ipinfo_response.country
                                .as_deref()
                                .unwrap_or("N/A")
                                .to_string();
                            let as_name = ipinfo_response.as_name
                                .as_deref()
                                .unwrap_or("N/A")
                                .to_string();
                            (country, as_name)
                        }
                        Err(e) => {
                            debug!("IPinfo query failed for {}: {}", ip_addr, e);
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
                        debug!("Task join error: {}", e);
                        // Continue with other results
                    }
                }
            }

            debug!("Completed IPinfo queries for {} prefixes", prefix_data.len());

            // Calculate adaptive column widths
            let prefix_width = std::cmp::max(
                6, // Minimum width for "Prefix"
                prefix_data
                    .iter()
                    .map(|(p, _, _)| p.len())
                    .max()
                    .unwrap_or(6)
            );

            let country_width = std::cmp::max(
                7, // Minimum width for "Country"
                prefix_data
                    .iter()
                    .map(|(_, c, _)| c.len())
                    .max()
                    .unwrap_or(7)
            );

            let as_name_width = std::cmp::max(
                7, // Minimum width for "AS Name"
                prefix_data
                    .iter()
                    .map(|(_, _, a)| a.len())
                    .max()
                    .unwrap_or(7)
            );

            formatted.push_str("Currently Announced Prefixes\n");
            formatted.push_str("============================\n\n");

            // Dynamic header
            formatted.push_str(
                &format!(
                    "{:<width1$} | {:<width2$} | {:<width3$}\n",
                    "Prefix",
                    "Country",
                    "AS Name",
                    width1 = prefix_width,
                    width2 = country_width,
                    width3 = as_name_width
                )
            );

            // Dynamic separator
            formatted.push_str(
                &format!(
                    "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}\n",
                    "",
                    "",
                    "",
                    width1 = prefix_width,
                    width2 = country_width,
                    width3 = as_name_width
                )
            );

            // Data rows
            for (prefix, country, as_name) in prefix_data {
                formatted.push_str(
                    &format!(
                        "{:<width1$} | {:<width2$} | {:<width3$}\n",
                        truncate_string(&prefix, prefix_width),
                        truncate_string(&country, country_width),
                        truncate_string(&as_name, as_name_width),
                        width1 = prefix_width,
                        width2 = country_width,
                        width3 = as_name_width
                    )
                );
            }

            formatted.push_str(&format!("\n% Total announced prefixes: {}\n", prefixes.len()));
        } else {
            formatted.push_str("% No announced prefixes found\n");
        }
    } else {
        formatted.push_str("% No prefixes data available\n");
    }

    // Show messages if any
    if let Some(messages) = &response.messages {
        if !messages.is_empty() {
            formatted.push_str("\n% API Messages:\n");
            for message in messages {
                for msg_part in message {
                    formatted.push_str(&format!("% {}\n", msg_part));
                }
            }
        }
    }

    Ok(formatted)
}

/// Format prefixes response with country information from IPinfo (blocking version)
pub fn format_prefixes_response_blocking(
    asn: &str,
    response: &PrefixesResponse,
    _client: &reqwest::blocking::Client
) -> Result<String> {
    let mut formatted = String::new();

    // Header
    formatted.push_str("% ASN Announced Prefixes Query\n");
    formatted.push_str("% Data from RIPE NCC STAT\n");
    formatted.push_str(&format!("% Query: {}\n", asn));
    formatted.push_str("\n");

    let data = match &response.data {
        Some(data) => data,
        None => {
            formatted.push_str("% No prefixes data available\n");
            return Ok(formatted);
        }
    };

    if let Some(prefixes) = &data.prefixes {
        if !prefixes.is_empty() {
            debug!(
                "Processing {} prefixes with concurrent IPinfo queries (blocking)",
                prefixes.len()
            );

            // Use rayon for parallel processing with limited parallelism
            use rayon::prelude::*;

            // Create a thread pool with 32 threads max
            let pool = rayon::ThreadPoolBuilder
                ::new()
                .num_threads(std::cmp::min(32, prefixes.len()))
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

            let prefix_data: Vec<(String, String, String)> = pool.install(|| {
                prefixes
                    .par_iter()
                    .map(|prefix_info| {
                        let prefix = prefix_info.prefix.clone();
                        let ip_addr = extract_ip_from_prefix(&prefix);
                        debug!("Querying IPinfo for IP: {} (from prefix: {})", ip_addr, prefix);

                        // Create a new client for this thread since reqwest::blocking::Client is not Clone + Send
                        let thread_client = reqwest::blocking::Client
                            ::builder()
                            .timeout(std::time::Duration::from_secs(10))
                            .build()
                            .unwrap_or_else(|_| reqwest::blocking::Client::new());

                        let (country, as_name) = match
                            query_ipinfo_api_blocking(&thread_client, &ip_addr)
                        {
                            Ok(ipinfo_response) => {
                                debug!(
                                    "IPinfo response for {}: as_name={:?}, country={:?}",
                                    ip_addr,
                                    ipinfo_response.as_name,
                                    ipinfo_response.country
                                );
                                let country = ipinfo_response.country
                                    .as_deref()
                                    .unwrap_or("N/A")
                                    .to_string();
                                let as_name = ipinfo_response.as_name
                                    .as_deref()
                                    .unwrap_or("N/A")
                                    .to_string();
                                (country, as_name)
                            }
                            Err(e) => {
                                debug!("IPinfo query failed for {}: {}", ip_addr, e);
                                ("N/A".to_string(), "N/A".to_string())
                            }
                        };

                        (prefix, country, as_name)
                    })
                    .collect()
            });

            debug!("Completed IPinfo queries for {} prefixes", prefix_data.len());

            // Calculate adaptive column widths
            let prefix_width = std::cmp::max(
                6, // Minimum width for "Prefix"
                prefix_data
                    .iter()
                    .map(|(p, _, _)| p.len())
                    .max()
                    .unwrap_or(6)
            );

            let country_width = std::cmp::max(
                7, // Minimum width for "Country"
                prefix_data
                    .iter()
                    .map(|(_, c, _)| c.len())
                    .max()
                    .unwrap_or(7)
            );

            let as_name_width = std::cmp::max(
                7, // Minimum width for "AS Name"
                prefix_data
                    .iter()
                    .map(|(_, _, a)| a.len())
                    .max()
                    .unwrap_or(7)
            );

            formatted.push_str("Currently Announced Prefixes\n");
            formatted.push_str("============================\n\n");

            // Dynamic header
            formatted.push_str(
                &format!(
                    "{:<width1$} | {:<width2$} | {:<width3$}\n",
                    "Prefix",
                    "Country",
                    "AS Name",
                    width1 = prefix_width,
                    width2 = country_width,
                    width3 = as_name_width
                )
            );

            // Dynamic separator
            formatted.push_str(
                &format!(
                    "{:-<width1$}-|-{:-<width2$}-|-{:-<width3$}\n",
                    "",
                    "",
                    "",
                    width1 = prefix_width,
                    width2 = country_width,
                    width3 = as_name_width
                )
            );

            // Data rows
            for (prefix, country, as_name) in prefix_data {
                formatted.push_str(
                    &format!(
                        "{:<width1$} | {:<width2$} | {:<width3$}\n",
                        truncate_string(&prefix, prefix_width),
                        truncate_string(&country, country_width),
                        truncate_string(&as_name, as_name_width),
                        width1 = prefix_width,
                        width2 = country_width,
                        width3 = as_name_width
                    )
                );
            }

            formatted.push_str(&format!("\n% Total announced prefixes: {}\n", prefixes.len()));
        } else {
            formatted.push_str("% No announced prefixes found\n");
        }
    } else {
        formatted.push_str("% No prefixes data available\n");
    }

    // Show messages if any
    if let Some(messages) = &response.messages {
        if !messages.is_empty() {
            formatted.push_str("\n% API Messages:\n");
            for message in messages {
                for msg_part in message {
                    formatted.push_str(&format!("% {}\n", msg_part));
                }
            }
        }
    }

    Ok(formatted)
}
