use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceAddress {
    pub interface: String,
    pub address: String,
    pub family: String,
    pub netmask: String,
    pub up: bool,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSnapshot {
    pub addresses: Vec<InterfaceAddress>,
    pub observed_at: Option<u64>,
    pub error: Option<String>,
}
pub fn observe() -> NetworkSnapshot {
    #[cfg(target_os = "macos")]
    {
        match crate::observer::run_bounded("/sbin/ifconfig", &["-a"]) {
            Ok((true, output)) => NetworkSnapshot {
                addresses: parse(&output),
                observed_at: Some(crate::model::now()),
                error: None,
            },
            _ => NetworkSnapshot {
                error: Some("network_observation_failed".into()),
                ..Default::default()
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    NetworkSnapshot {
        error: Some("macos_required".into()),
        ..Default::default()
    }
}
fn parse(output: &str) -> Vec<InterfaceAddress> {
    let mut name = String::new();
    let mut up = false;
    let mut rows = vec![];
    for line in output.lines() {
        if !line.starts_with(char::is_whitespace) {
            if let Some((interface, flags)) = line.split_once(": flags=") {
                name = interface.into();
                up = flags
                    .split('<')
                    .nth(1)
                    .and_then(|s| s.split('>').next())
                    .is_some_and(|s| s.split(',').any(|f| f == "UP"));
            } else {
                name.clear();
            }
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if name.is_empty() || parts.len() < 2 || !["inet", "inet6"].contains(&parts[0]) {
            continue;
        }
        if parts[0] == "inet6"
            && parts
                .iter()
                .any(|p| ["tentative", "duplicated", "detached", "deprecated"].contains(p))
        {
            continue;
        }
        let raw = parts[1];
        let valid = if parts[0] == "inet" {
            raw.parse::<std::net::Ipv4Addr>().is_ok()
        } else {
            raw.split('%')
                .next()
                .is_some_and(|s| s.parse::<std::net::Ipv6Addr>().is_ok())
        };
        if !valid {
            continue;
        }
        let marker = if parts[0] == "inet" {
            "netmask"
        } else {
            "prefixlen"
        };
        let mask = parts
            .windows(2)
            .find(|p| p[0] == marker)
            .map(|p| p[1])
            .unwrap_or("");
        let netmask = if marker == "netmask" && mask.starts_with("0x") {
            u32::from_str_radix(&mask[2..], 16)
                .map(std::net::Ipv4Addr::from)
                .map(|ip| ip.to_string())
                .unwrap_or_default()
        } else {
            mask.to_string()
        };
        rows.push(InterfaceAddress {
            interface: name.clone(),
            address: raw.into(),
            family: if parts[0] == "inet" { "IPv4" } else { "IPv6" }.into(),
            netmask,
            up,
        });
    }
    rows
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn unusable_ipv6_sources_are_not_offered() {
        let output="en0: flags=1<UP>\n inet6 fe80::1%en0 prefixlen 64 secured\n inet6 fd00::1 prefixlen 64 tentative\n inet6 fd00::2 prefixlen 64 duplicated\n inet6 fd00::3 prefixlen 64 deprecated\n inet6 fd00::4 prefixlen 64 detached\n";
        let rows = parse(output);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].address, "fe80::1%en0");
    }
    #[test]
    #[ignore = "Reads this Mac's local interfaces only; no network scan or configuration change"]
    fn reads_real_local_interfaces_without_network_probes() {
        let snapshot = observe();
        assert!(snapshot.error.is_none());
        assert!(snapshot.observed_at.is_some());
        assert!(snapshot
            .addresses
            .iter()
            .any(|row| row.interface == "lo0" && row.address == "127.0.0.1"));
    }
    #[test]
    fn parses_multiple_addresses_and_scope_without_inventing_devices() {
        let rows = parse("en0: flags=8863<UP,BROADCAST,RUNNING> mtu 1500\n\tinet 192.168.1.2 netmask 0xffffff00 broadcast 192.168.1.255\n\tinet6 fe80::1%en0 prefixlen 64 scopeid 0xe\n\tether aa:bb:cc:dd:ee:ff\nen1: flags=8802<BROADCAST> mtu 1500\n\tinet nonsense netmask 0x00\n\tinet 10.0.0.2 netmask 0xff000000\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].netmask, "255.255.255.0");
        assert_eq!(rows[1].address, "fe80::1%en0");
        assert!(!rows[2].up);
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LanContext {
    pub gateway: Option<String>,
    pub ssid: Option<String>,
}
pub fn lan_context(interface: &str) -> LanContext {
    if !interface.starts_with("en")
        || interface.len() < 3
        || !interface[2..].chars().all(|c| c.is_ascii_digit())
    {
        return LanContext::default();
    }
    let gateway = crate::observer::run_bounded(
        "/sbin/route",
        &["-n", "get", "-ifscope", interface, "default"],
    )
    .ok()
    .filter(|(ok, _)| *ok)
    .and_then(|(_, text)| parse_gateway(&text, interface));
    let ssid =
        crate::observer::run_bounded("/usr/sbin/networksetup", &["-getairportnetwork", interface])
            .ok()
            .filter(|(ok, _)| *ok)
            .and_then(|(_, text)| {
                text.lines().find_map(|line| {
                    line.strip_prefix("Current Wi-Fi Network: ")
                        .filter(|s| {
                            !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
                        })
                        .map(str::to_owned)
                })
            });
    LanContext { gateway, ssid }
}
fn parse_gateway(text: &str, interface: &str) -> Option<String> {
    if !text.lines().any(|line| {
        line.trim()
            .strip_prefix("interface:")
            .is_some_and(|v| v.trim() == interface)
    }) {
        return None;
    }
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("gateway:")
            .and_then(|v| v.trim().parse::<std::net::Ipv4Addr>().ok())
            .filter(|ip| !ip.is_unspecified() && !ip.is_multicast())
            .map(|ip| ip.to_string())
    })
}
