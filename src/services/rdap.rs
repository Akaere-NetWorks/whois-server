use anyhow::Result;
use rdap::{RdapClient, RdapRequest};
use tracing::{debug, warn};

/// Process RDAP query
pub async fn process_rdap_query(query: &str) -> Result<String> {
    debug!("Processing RDAP query: {}", query);

    // Create RDAP client
    let client = match RdapClient::new() {
        Ok(c) => c,
        Err(e) => {
            return Ok(format!(
                "% RDAP Client Error\n\
                 % Failed to create RDAP client: {}\n",
                e
            ));
        }
    };

    // Auto-detect query type
    let query_type = match RdapRequest::detect_type(query) {
        Ok(qt) => qt,
        Err(e) => {
            return Ok(format!(
                "% RDAP Query Error\n\
                 % Unable to detect query type for: {}\n\
                 % Error: {}\n\
                 % \n\
                 % Supported query types:\n\
                 %   - Domain names (e.g., example.com)\n\
                 %   - IP addresses (e.g., 8.8.8.8, 2001:4860:4860::8888)\n\
                 %   - AS numbers (e.g., AS15169 or 15169)\n",
                query, e
            ));
        }
    };

    debug!("Detected RDAP query type: {:?}", query_type);

    // Create request
    let request = RdapRequest::new(query_type, query);

    // Execute query
    match client.query(&request).await {
        Ok(result) => {
            // Format the RDAP response in WHOIS-like style
            let output = format!(
                "% RDAP (Registration Data Access Protocol) Response\n\
                 % Query: {}\n\
                 % Query Type: {:?}\n\
                 % \n",
                query, query_type
            );
            
            // Format the result manually
            Ok(output + &format_rdap_output(&result))
        }
        Err(e) => {
            warn!("RDAP query failed for {}: {}", query, e);
            Ok(format!(
                "% RDAP Query Failed\n\
                 % Query: {}\n\
                 % Error: {}\n\
                 % \n\
                 % This may be due to:\n\
                 %   - Network connectivity issues\n\
                 %   - RDAP service unavailable\n\
                 %   - Invalid or non-existent resource\n",
                query, e
            ))
        }
    }
}

/// Format RDAP output in WHOIS-like style
fn format_rdap_output(result: &rdap::RdapObject) -> String {
    use rdap::RdapObject;
    
    match result {
        RdapObject::Domain(domain) => {
            let mut output = String::new();
            output.push_str("% Object Type: Domain\n%\n");
            
            if let Some(name) = &domain.ldh_name {
                output.push_str(&format!("domain:          {}\n", name));
            }
            
            if let Some(handle) = &domain.handle {
                output.push_str(&format!("handle:          {}\n", handle));
            }
            
            // Status
            if !domain.status.is_empty() {
                for status in &domain.status {
                    output.push_str(&format!("status:          {}\n", status));
                }
            }
            
            // Nameservers
            if !domain.nameservers.is_empty() {
                output.push_str("\n");
                for ns in &domain.nameservers {
                    if let Some(name) = &ns.ldh_name {
                        output.push_str(&format!("nserver:         {}\n", name));
                    }
                }
            }
            
            // DNSSEC
            if let Some(dnssec) = &domain.secure_dns {
                output.push_str("\n");
                if let Some(signed) = dnssec.delegation_signed {
                    output.push_str(&format!(
                        "dnssec:          {}\n",
                        if signed { "signedDelegation" } else { "unsigned" }
                    ));
                }
            }
            
            // Events
            if !domain.events.is_empty() {
                output.push_str("\n");
                for event in &domain.events {
                    output.push_str(&format!("{:16} {}\n", format!("{}:", event.action), event.date));
                }
            }
            
            // Entities
            if !domain.entities.is_empty() {
                output.push_str("\n");
                for entity in &domain.entities {
                    format_entity(&mut output, entity);
                }
            }
            
            output.push_str("\n% End of RDAP response\n");
            output
        }
        
        RdapObject::IpNetwork(network) => {
            let mut output = String::new();
            output.push_str("% Object Type: IP Network\n%\n");
            
            if let Some(name) = &network.name {
                output.push_str(&format!("netname:         {}\n", name));
            }
            
            if let Some(handle) = &network.handle {
                output.push_str(&format!("handle:          {}\n", handle));
            }
            
            if let Some(start) = &network.start_address {
                output.push_str(&format!("start-address:   {}\n", start));
            }
            
            if let Some(end) = &network.end_address {
                output.push_str(&format!("end-address:     {}\n", end));
            }
            
            if let Some(ip_version) = &network.ip_version {
                output.push_str(&format!("ip-version:      v{}\n", ip_version));
            }
            
            if let Some(net_type) = &network.network_type {
                output.push_str(&format!("type:            {}\n", net_type));
            }
            
            if let Some(country) = &network.country {
                output.push_str(&format!("country:         {}\n", country));
            }
            
            // Status
            if !network.status.is_empty() {
                for status in &network.status {
                    output.push_str(&format!("status:          {}\n", status));
                }
            }
            
            // Events
            if !network.events.is_empty() {
                output.push_str("\n");
                for event in &network.events {
                    output.push_str(&format!("{:16} {}\n", format!("{}:", event.action), event.date));
                }
            }
            
            // Entities
            if !network.entities.is_empty() {
                output.push_str("\n");
                for entity in &network.entities {
                    format_entity(&mut output, entity);
                }
            }
            
            output.push_str("\n% End of RDAP response\n");
            output
        }
        
        RdapObject::Autnum(asn) => {
            let mut output = String::new();
            output.push_str("% Object Type: Autonomous System Number\n%\n");
            
            if let Some(start) = asn.start_autnum {
                output.push_str(&format!("aut-num:         AS{}\n", start));
            }
            
            if let Some(end) = asn.end_autnum {
                if end != asn.start_autnum.unwrap_or(0) {
                    output.push_str(&format!("end-autnum:      AS{}\n", end));
                }
            }
            
            if let Some(handle) = &asn.handle {
                output.push_str(&format!("handle:          {}\n", handle));
            }
            
            if let Some(name) = &asn.name {
                output.push_str(&format!("as-name:         {}\n", name));
            }
            
            if let Some(as_type) = &asn.as_type {
                output.push_str(&format!("type:            {}\n", as_type));
            }
            
            if let Some(country) = &asn.country {
                output.push_str(&format!("country:         {}\n", country));
            }
            
            // Status
            if !asn.status.is_empty() {
                for status in &asn.status {
                    output.push_str(&format!("status:          {}\n", status));
                }
            }
            
            // Events
            if !asn.events.is_empty() {
                output.push_str("\n");
                for event in &asn.events {
                    output.push_str(&format!("{:16} {}\n", format!("{}:", event.action), event.date));
                }
            }
            
            // Entities
            if !asn.entities.is_empty() {
                output.push_str("\n");
                for entity in &asn.entities {
                    format_entity(&mut output, entity);
                }
            }
            
            output.push_str("\n% End of RDAP response\n");
            output
        }
        
        RdapObject::Error(err) => {
            let mut output = String::new();
            output.push_str("% RDAP Error Response\n%\n");
            
            if let Some(code) = err.error_code {
                output.push_str(&format!("error-code:      {}\n", code));
            }
            
            if let Some(title) = &err.title {
                output.push_str(&format!("title:           {}\n", title));
            }
            
            if !err.description.is_empty() {
                output.push_str("\n");
                for desc in &err.description {
                    output.push_str(&format!("description:     {}\n", desc));
                }
            }
            
            output
        }
        
        _ => {
            "% Unsupported RDAP object type\n".to_string()
        }
    }
}

/// Format entity information
fn format_entity(output: &mut String, entity: &rdap::Entity) {
    if let Some(handle) = &entity.handle {
        output.push_str(&format!("entity-handle:   {}\n", handle));
    }
    
    if !entity.roles.is_empty() {
        let roles: Vec<String> = entity.roles.iter().map(|r| format!("{:?}", r)).collect();
        output.push_str(&format!("roles:           {}\n", roles.join(", ")));
    }
    
    // vCard data
    if let Some(vcard) = &entity.vcard {
        if let Some(name) = vcard.name() {
            output.push_str(&format!("name:            {}\n", name));
        }
        
        if let Some(email) = vcard.email() {
            output.push_str(&format!("email:           {}\n", email));
        }
    }
    
    output.push_str("\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rdap_query_format() {
        // Basic test to ensure the function signature works
        let result = process_rdap_query("example.com").await;
        assert!(result.is_ok());
    }
}
