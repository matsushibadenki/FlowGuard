use serde::Serialize;
use serde_json::Value;
use std::{net::IpAddr, time::Duration};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub operation: u64,
    pub busy: bool,
    pub query: String,
    pub source: String,
    pub fields: Vec<Field>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub key: String,
    pub value: String,
}

pub fn validate(input: &str) -> Result<(String, Option<IpAddr>), String> {
    let q = input.trim().to_ascii_lowercase();
    if let Ok(ip) = q.parse::<IpAddr>() {
        return Ok((ip.to_string(), Some(ip)));
    }
    if q.len() > 253
        || q.len() < 3
        || !q.contains('.')
        || q.split('.').any(|p| {
            p.is_empty()
                || p.len() > 63
                || p.starts_with('-')
                || p.ends_with('-')
                || !p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("invalid_query".into());
    }
    Ok((q, None))
}
fn prefix_match(ip: IpAddr, prefix: &str) -> Option<u8> {
    let (net, bits) = prefix.split_once('/')?;
    let net = net.parse::<IpAddr>().ok()?;
    let n = bits.parse::<u8>().ok()?;
    match (ip, net) {
        (IpAddr::V4(a), IpAddr::V4(b)) if n <= 32 => {
            let mask = if n == 0 { 0 } else { u32::MAX << (32 - n) };
            if u32::from(a) & mask == u32::from(b) & mask {
                Some(n)
            } else {
                None
            }
        }
        (IpAddr::V6(a), IpAddr::V6(b)) if n <= 128 => {
            let mask = if n == 0 { 0 } else { u128::MAX << (128 - n) };
            if u128::from(a) & mask == u128::from(b) & mask {
                Some(n)
            } else {
                None
            }
        }
        _ => None,
    }
}
fn server(bootstrap: &Value, query: &str, ip: Option<IpAddr>) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for entry in bootstrap.get("services")?.as_array()? {
        let pair = entry.as_array()?;
        if pair.len() != 2 {
            continue;
        }
        let Some(keys) = pair[0].as_array() else {
            continue;
        };
        let Some(urls) = pair[1].as_array() else {
            continue;
        };
        let score = keys
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|key| {
                if let Some(ip) = ip {
                    prefix_match(ip, key)
                } else if query == key || query.ends_with(&format!(".{key}")) {
                    Some(key.len().min(255) as u8)
                } else {
                    None
                }
            })
            .max();
        if let Some(score) = score {
            if best.as_ref().is_some_and(|(n, _)| *n >= score) {
                continue;
            }
            if let Some(url) = urls
                .iter()
                .filter_map(Value::as_str)
                .find(|s| s.starts_with("https://"))
            {
                best = Some((score, url.to_string()));
            }
        }
    }
    best.map(|(_, url)| url)
}
async fn json(client: &reqwest::Client, url: &str, limit: usize) -> Result<Value, String> {
    let mut r = client
        .get(url)
        .header("Accept", "application/rdap+json, application/json")
        .send()
        .await
        .map_err(|_| "network_error")?;
    if !r.status().is_success() {
        return Err(if r.status().as_u16() == 404 {
            "not_found"
        } else {
            "server_error"
        }
        .into());
    }
    if r.content_length().is_some_and(|n| n > limit as u64) {
        return Err("response_limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(c) = r.chunk().await.map_err(|_| "network_error")? {
        if bytes.len() + c.len() > limit {
            return Err("response_limit".into());
        }
        bytes.extend_from_slice(&c);
    }
    serde_json::from_slice(&bytes).map_err(|_| "invalid_response".into())
}
pub async fn fetch(q: &str) -> Result<(String, Vec<Field>), String> {
    let (q, ip) = validate(q)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let kind = match ip {
        Some(IpAddr::V4(_)) => "ipv4",
        Some(IpAddr::V6(_)) => "ipv6",
        None => "dns",
    };
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .user_agent("FlowGuard/0.1")
        .build()
        .map_err(|_| "network_error")?;
    let registry = json(
        &client,
        &format!("https://data.iana.org/rdap/{kind}.json"),
        512 * 1024,
    )
    .await?;
    let base = server(&registry, &q, ip).ok_or("no_registry")?;
    let url = reqwest::Url::parse(&base).map_err(|_| "invalid_registry")?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url
            .host_str()
            .is_none_or(|h| h.parse::<IpAddr>().is_ok() || h.eq_ignore_ascii_case("localhost"))
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_registry".into());
    }
    let url = format!(
        "{}{}/{}",
        base.trim_end_matches('/'),
        if ip.is_some() { "/ip" } else { "/domain" },
        q
    );
    let data = json(&client, &url, 1024 * 1024).await?;
    let mut fields = Vec::new();
    for key in [
        "handle",
        "name",
        "ldhName",
        "unicodeName",
        "startAddress",
        "endAddress",
        "ipVersion",
        "type",
        "country",
        "parentHandle",
    ] {
        if let Some(value) = data
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            fields.push(Field {
                key: key.into(),
                value: value.chars().take(512).collect(),
            });
        }
    }
    if let Some(status) = data.get("status").and_then(Value::as_array) {
        let v = status
            .iter()
            .filter_map(Value::as_str)
            .take(16)
            .collect::<Vec<_>>()
            .join(", ");
        if !v.is_empty() {
            fields.push(Field {
                key: "status".into(),
                value: v.chars().take(512).collect(),
            });
        }
    }
    if let Some(events) = data.get("events").and_then(Value::as_array) {
        for e in events.iter().take(16) {
            if let (Some(k), Some(v)) = (
                e.get("eventAction").and_then(Value::as_str),
                e.get("eventDate").and_then(Value::as_str),
            ) {
                fields.push(Field {
                    key: format!("event:{}", k.chars().take(40).collect::<String>()),
                    value: v.chars().take(128).collect(),
                });
            }
        }
    }
    if let Some(entities) = data.get("entities").and_then(Value::as_array) {
        for entity in entities.iter().take(16) {
            let role = entity
                .get("roles")
                .and_then(Value::as_array)
                .and_then(|r| r.first())
                .and_then(Value::as_str)
                .unwrap_or("entity");
            if let Some(properties) = entity
                .get("vcardArray")
                .and_then(Value::as_array)
                .and_then(|v| v.get(1))
                .and_then(Value::as_array)
            {
                for property in properties.iter().take(32) {
                    if let Some(parts) = property.as_array() {
                        if parts.first().and_then(Value::as_str) == Some("fn") {
                            if let Some(value) = parts.get(3).and_then(Value::as_str) {
                                fields.push(Field {
                                    key: format!(
                                        "entity:{}",
                                        role.chars().take(40).collect::<String>()
                                    ),
                                    value: value.chars().take(512).collect(),
                                });
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    if let Some(nameservers) = data.get("nameservers").and_then(Value::as_array) {
        for ns in nameservers.iter().take(16) {
            if let Some(value) = ns.get("ldhName").and_then(Value::as_str) {
                fields.push(Field {
                    key: "nameserver".into(),
                    value: value.chars().take(253).collect(),
                });
            }
        }
    }
    if fields.is_empty() {
        return Err("invalid_response".into());
    }
    Ok((base, fields))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_validation() {
        assert!(validate("Example.COM").is_ok());
        for s in [
            "https://example.com",
            "a/b.com",
            "a..com",
            "foo:80",
            "-bad.com",
            "",
        ] {
            assert!(validate(s).is_err());
        }
    }
    #[test]
    fn longest_prefix() {
        let data = serde_json::json!({"services":[[["0.0.0.0/0"],["https://a.example/"]],[["192.0.2.0/24"],["https://b.example/"]]]});
        assert_eq!(
            server(&data, "192.0.2.2", Some("192.0.2.2".parse().unwrap())).unwrap(),
            "https://b.example/"
        );
    }
}
