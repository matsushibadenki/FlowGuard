use crate::{
    model::now,
    network::{InterfaceAddress, NetworkSnapshot},
    observer::run_bounded,
};
use serde::Serialize;
use std::{collections::BTreeMap, net::Ipv4Addr};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub name: crate::names::NameObservation,
    pub id: String,
    pub interface: String,
    pub address: String,
    pub family: String,
    pub mac: Option<String>,
    pub source: String,
    pub response: String,
    pub observed_at: u64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySnapshot {
    pub name_mode: String,
    pub names_completed: usize,
    pub names_total: usize,
    pub stage: String,
    pub interface: String,
    pub range: String,
    pub active: bool,
    pub completed: usize,
    pub total: usize,
    pub devices: Vec<Device>,
    pub errors: Vec<String>,
    pub observed_at: Option<u64>,
}
impl Default for DiscoverySnapshot {
    fn default() -> Self {
        Self {
            name_mode: "none".into(),
            names_completed: 0,
            names_total: 0,
            stage: "idle".into(),
            interface: String::new(),
            range: String::new(),
            active: false,
            completed: 0,
            total: 0,
            devices: vec![],
            errors: vec![],
            observed_at: None,
        }
    }
}
#[derive(Clone)]
pub struct Plan {
    pub name_mode: String,
    pub local: InterfaceAddress,
    pub targets: Vec<Ipv4Addr>,
    pub range: String,
    pub active: bool,
}
impl Plan {
    pub fn new(
        network: &NetworkSnapshot,
        interface: &str,
        address: &str,
        active: bool,
    ) -> Result<Self, String> {
        if network.error.is_some() {
            return Err("interfaces_unavailable".into());
        }
        let local = network
            .addresses
            .iter()
            .find(|a| {
                a.interface == interface
                    && a.address == address
                    && a.up
                    && a.interface.starts_with("en")
                    && a.interface[2..].chars().all(|c| c.is_ascii_digit())
                    && a.interface.len() > 2
            })
            .ok_or("invalid_lan_interface")?
            .clone();
        let (targets, range) = if active {
            ipv4_range(&local)?
        } else {
            (vec![], interface.to_string())
        };
        Ok(Self {
            name_mode: "none".into(),
            local,
            targets,
            range,
            active,
        })
    }
    pub fn with_names(mut self, mode: &str) -> Result<Self, String> {
        if !["none", "mdns", "system"].contains(&mode) {
            return Err("invalid_name_mode".into());
        }
        self.name_mode = mode.into();
        Ok(self)
    }
    pub fn matches(&self, network: &NetworkSnapshot) -> bool {
        network.error.is_none()
            && network.addresses.iter().any(|a| {
                a.interface == self.local.interface
                    && a.address == self.local.address
                    && a.netmask == self.local.netmask
                    && a.up
            })
    }
}
fn ipv4_range(local: &InterfaceAddress) -> Result<(Vec<Ipv4Addr>, String), String> {
    let ip: Ipv4Addr = local.address.parse().map_err(|_| "ipv4_range_required")?;
    if !ip.is_private() {
        return Err("private_ipv4_required".into());
    }
    let mask: Ipv4Addr = local.netmask.parse().map_err(|_| "invalid_netmask")?;
    let mask = u32::from(mask);
    let prefix = mask.leading_ones();
    if !(24..=30).contains(&prefix) || mask != u32::MAX << (32 - prefix) {
        return Err("range_too_large".into());
    }
    let start = u32::from(ip) & mask;
    let end = start | !mask;
    if u32::from(ip) == start || u32::from(ip) == end {
        return Err("invalid_host_address".into());
    }
    let targets = (start + 1..end)
        .filter(|a| *a != u32::from(ip))
        .map(Ipv4Addr::from)
        .collect();
    Ok((
        targets,
        format!(
            "{}–{} /{}",
            Ipv4Addr::from(start + 1),
            Ipv4Addr::from(end - 1),
            prefix
        ),
    ))
}
fn mac(raw: &str) -> Option<String> {
    let parts: Vec<_> = raw.split(':').collect();
    if parts.len() != 6
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 2 || !p.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return None;
    }
    let bytes: Vec<u8> = parts
        .iter()
        .map(|p| u8::from_str_radix(p, 16).unwrap())
        .collect();
    if bytes.iter().all(|b| *b == 0) || bytes[0] & 1 != 0 {
        return None;
    }
    Some(
        bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}
fn row(interface: &str, address: &str, family: &str, raw_mac: &str, source: &str) -> Device {
    Device {
        name: Default::default(),
        id: format!("{interface}|{address}"),
        interface: interface.into(),
        address: address.into(),
        family: family.into(),
        mac: mac(raw_mac),
        source: source.into(),
        response: "unverified".into(),
        observed_at: now(),
    }
}
pub fn parse_neighbors(arp: &str, ndp: &str, interface: &str) -> Vec<Device> {
    let mut rows = BTreeMap::new();
    for line in arp.lines() {
        if rows.len() >= 1025 {
            break;
        }
        let p: Vec<_> = line.split_whitespace().collect();
        if p.len() < 7 || p[2] != "at" || p[4] != "on" || p[5] != interface {
            continue;
        }
        let address = p[1].trim_matches(['(', ')']);
        let Ok(ip) = address.parse::<Ipv4Addr>() else {
            continue;
        };
        // Incomplete entries and multicast/broadcast entries are not discovered devices.
        if ip.is_multicast() || ip.is_unspecified() || ip.is_broadcast() || mac(p[3]).is_none() {
            continue;
        }
        let device = row(interface, address, "IPv4", p[3], "arp");
        rows.insert(device.id.clone(), device);
    }
    for line in ndp.lines() {
        if rows.len() >= 1025 {
            break;
        }
        let p: Vec<_> = line.split_whitespace().collect();
        if p.len() < 5 || p[2] != interface || mac(p[1]).is_none() {
            continue;
        }
        let Ok(ip) = p[0]
            .split('%')
            .next()
            .unwrap_or("")
            .parse::<std::net::Ipv6Addr>()
        else {
            continue;
        };
        if ip.is_multicast() || ip.is_unspecified() {
            continue;
        }
        let device = row(interface, p[0], "IPv6", p[1], "ndp");
        rows.insert(device.id.clone(), device);
    }
    rows.into_values().collect()
}
fn collect(plan: &Plan, network: &NetworkSnapshot, snapshot: &mut DiscoverySnapshot) {
    let arp = run_bounded("/usr/sbin/arp", &["-an"]);
    let ndp = run_bounded("/usr/sbin/ndp", &["-an"]);
    let read = |value: Result<(bool, String), String>| {
        value
            .ok()
            .filter(|(success, _)| *success)
            .map(|(_, text)| text)
    };
    let arp = read(arp);
    let ndp = read(ndp);
    if arp.is_none() {
        snapshot.errors.push("arp_unavailable".into());
    }
    if ndp.is_none() {
        snapshot.errors.push("ndp_unavailable".into());
    }
    let neighbors = parse_neighbors(
        arp.as_deref().unwrap_or(""),
        ndp.as_deref().unwrap_or(""),
        &plan.local.interface,
    );
    for device in neighbors {
        if let Some(existing) = snapshot.devices.iter_mut().find(|d| d.id == device.id) {
            existing.mac = device.mac;
            if existing.response == "unverified" {
                existing.observed_at = device.observed_at;
            }
            if existing.source == "ping" {
                existing.source = device.source;
            }
        } else {
            snapshot.devices.push(device);
        }
    }
    for address in network
        .addresses
        .iter()
        .filter(|a| a.interface == plan.local.interface)
    {
        let mut device = row(
            &address.interface,
            &address.address,
            &address.family,
            "",
            "local",
        );
        device.response = "local".into();
        if let Some(existing) = snapshot.devices.iter_mut().find(|d| d.id == device.id) {
            device.mac = existing.mac.clone();
            *existing = device;
        } else {
            snapshot.devices.push(device);
        }
    }
    snapshot.devices.sort_by(|a, b| {
        let key = |d: &Device| {
            (
                d.family.clone(),
                d.address
                    .split('%')
                    .next()
                    .unwrap_or("")
                    .parse::<std::net::IpAddr>()
                    .ok(),
            )
        };
        key(a).cmp(&key(b))
    });
    if snapshot.devices.len() > 1024 {
        snapshot.devices.truncate(1024);
        snapshot.errors.push("results_truncated".into());
    }
    snapshot.errors.sort();
    snapshot.errors.dedup();
}
fn enrich_names(
    plan: &Plan,
    s: &mut DiscoverySnapshot,
    update: &mut impl FnMut(DiscoverySnapshot),
    cancelled: &(impl Fn() -> bool + Sync),
) {
    let mut targets = vec![];
    for (index, device) in s.devices.iter_mut().enumerate() {
        device.name.status = if !crate::names::eligible(&device.address, &plan.name_mode) {
            "skipped"
        } else if targets.len() >= 64 {
            "limited"
        } else {
            targets.push(index);
            "pending"
        }
        .into();
    }
    if s.devices.iter().any(|d| d.name.status == "limited") {
        s.errors.push("names_limited".into());
    }
    s.names_total = targets.len();
    update(s.clone());
    for batch in targets.chunks(4) {
        if cancelled() {
            return;
        }
        if !plan.matches(&crate::network::observe()) {
            s.errors.push("network_changed".into());
            return;
        }
        let replies = std::thread::scope(|scope| {
            let jobs: Vec<_> = batch
                .iter()
                .map(|&index| {
                    let device = &s.devices[index];
                    let mode = &plan.name_mode;
                    scope.spawn(move || {
                        (
                            index,
                            crate::names::lookup(
                                &device.address,
                                &device.interface,
                                mode,
                                cancelled,
                            ),
                        )
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join()).collect::<Vec<_>>()
        });
        if cancelled() {
            return;
        }
        for reply in replies {
            match reply {
                Ok((index, name)) => {
                    if name.status == "failed" {
                        s.errors.push("name_lookup_failed".into());
                    }
                    s.devices[index].name = name;
                }
                Err(_) => s.errors.push("name_lookup_failed".into()),
            }
            s.names_completed += 1;
        }
        s.errors.sort();
        s.errors.dedup();
        update(s.clone());
    }
}
fn no_response(output: &str) -> bool {
    !output.contains("ping:")
        && (output.contains("100.0% packet loss") || output.contains("100% packet loss"))
}
pub fn run(
    plan: Plan,
    mut update: impl FnMut(DiscoverySnapshot),
    cancelled: impl Fn() -> bool + Sync,
) {
    let mut s = DiscoverySnapshot {
        name_mode: plan.name_mode.clone(),
        stage: "running".into(),
        interface: plan.local.interface.clone(),
        range: plan.range.clone(),
        active: plan.active,
        total: plan.targets.len(),
        ..Default::default()
    };
    let initial = crate::network::observe();
    if !plan.matches(&initial) {
        s.stage = "failed".into();
        s.observed_at = Some(now());
        s.errors.push("network_changed".into());
        update(s);
        return;
    }
    collect(&plan, &initial, &mut s);
    update(s.clone());
    for batch in plan.targets.chunks(4) {
        if cancelled() {
            s.stage = "cancelled".into();
            s.observed_at = Some(now());
            update(s);
            return;
        }
        if !plan.matches(&crate::network::observe()) {
            s.stage = "failed".into();
            s.observed_at = Some(now());
            s.errors.push("network_changed".into());
            update(s);
            return;
        }
        let results = std::thread::scope(|scope| {
            let jobs: Vec<_> = batch
                .iter()
                .map(|ip| {
                    let interface = &plan.local.interface;
                    scope.spawn(move || {
                        let ip = ip.to_string();
                        let reply = crate::observer::run_bounded_diagnostic(
                            "/sbin/ping",
                            &[
                                "-n", "-c", "1", "-W", "500", "-t", "1", "-b", interface, &ip,
                            ],
                        );
                        (ip, reply)
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join()).collect::<Vec<_>>()
        });
        if cancelled() {
            s.stage = "cancelled".into();
            s.observed_at = Some(now());
            update(s);
            return;
        }
        for result in results {
            s.completed += 1;
            match result {
                Ok((ip, Ok((true, _)))) => {
                    let id = format!("{}|{}", plan.local.interface, ip);
                    if let Some(device) = s.devices.iter_mut().find(|d| d.id == id) {
                        device.response = "responded".into();
                        device.observed_at = now();
                    } else {
                        let mut device = row(&plan.local.interface, &ip, "IPv4", "", "ping");
                        device.response = "responded".into();
                        s.devices.push(device);
                    }
                }
                Ok((ip, Ok((false, output)))) if no_response(&output) => {
                    if let Some(device) = s.devices.iter_mut().find(|d| d.address == ip) {
                        device.response = "no_response".into();
                    }
                }
                _ => {
                    s.errors.push("probe_failed".into());
                }
            }
        }
        s.errors.sort();
        s.errors.dedup();
        update(s.clone());
    }
    if cancelled() {
        s.stage = "cancelled".into();
    } else {
        let current = crate::network::observe();
        if !plan.matches(&current) {
            s.stage = "failed".into();
            s.observed_at = Some(now());
            s.errors.push("network_changed".into());
        } else {
            collect(&plan, &current, &mut s);
            if plan.name_mode != "none" {
                enrich_names(&plan, &mut s, &mut update, &cancelled);
                if cancelled() {
                    s.stage = "cancelled".into();
                    s.observed_at = Some(now());
                    update(s);
                    return;
                }
                if s.errors.iter().any(|e| e == "network_changed")
                    || !plan.matches(&crate::network::observe())
                {
                    s.errors.push("network_changed".into());
                    s.stage = "failed".into();
                    s.observed_at = Some(now());
                    update(s);
                    return;
                }
            }
            s.stage = if s.errors.is_empty() {
                "completed"
            } else {
                "partial"
            }
            .into();
        }
    }
    s.observed_at = Some(now());
    update(s);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn network(ip: &str, mask: &str) -> NetworkSnapshot {
        NetworkSnapshot {
            addresses: vec![InterfaceAddress {
                interface: "en0".into(),
                address: ip.into(),
                family: "IPv4".into(),
                netmask: mask.into(),
                up: true,
            }],
            ..Default::default()
        }
    }
    #[test]
    fn range_rejects_large_public_invalid_and_changed_networks() {
        let n = network("192.168.1.20", "255.255.255.0");
        let p = Plan::new(&n, "en0", "192.168.1.20", true).unwrap();
        assert_eq!(p.targets.len(), 253);
        assert!(!p.targets.contains(&"192.168.1.20".parse().unwrap()));
        assert!(!p.matches(&network("192.168.2.20", "255.255.255.0")));
        for (ip, mask) in [
            ("192.168.1.2", "255.255.0.0"),
            ("8.8.8.8", "255.255.255.0"),
            ("192.168.1.2", "255.255.255.1"),
        ] {
            assert!(Plan::new(&network(ip, mask), "en0", ip, true).is_err());
        }
        assert!(Plan::new(&n, "en0; echo test", "192.168.1.20", false).is_err());
    }
    #[test]
    fn parses_neighbors_without_equating_cache_and_reachability() {
        let rows=parse_neighbors("? (192.168.1.1) at a:b:c:d:e:f on en0 ifscope [ethernet]\n? (192.168.1.2) at (incomplete) on en0 ifscope [ethernet]\n? (192.168.1.3) at aa:bb:cc:dd:ee:ff on en1 ifscope [ethernet]", "Neighbor Linklayer Netif Expire St\nfe80::2%en0 aa:bb:cc:dd:ee:ff en0 23h S\nfe80::3%en0 (incomplete) en0 0s I", "en0");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].mac.as_deref(), Some("0a:0b:0c:0d:0e:0f"));
        assert_eq!(rows[0].response, "unverified");
        assert_eq!(rows[1].address, "fe80::2%en0");
    }
    #[test]
    fn denied_probes_are_not_reported_as_no_response() {
        assert!(no_response(
            "1 packets transmitted, 0 packets received, 100.0% packet loss"
        ));
        assert!(!no_response("1 packets transmitted, 0 packets received, 100.0% packet loss\nping: sendto: Operation not permitted"));
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Sends one ICMP echo to loopback only; never scans LAN"]
    fn unsigned_ping_can_check_loopback() {
        let (success, output) = run_bounded(
            "/sbin/ping",
            &[
                "-n",
                "-c",
                "1",
                "-W",
                "500",
                "-t",
                "1",
                "-b",
                "lo0",
                "127.0.0.1",
            ],
        )
        .unwrap();
        assert!(success, "Loopback ping unavailable");
        assert!(output.contains("1 packets received"));
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Reads existing neighbor caches only; no probes"]
    fn unsigned_cache_collection_reads_real_tools() {
        let n = crate::network::observe();
        if let Some(a) = n
            .addresses
            .iter()
            .find(|a| a.interface.starts_with("en") && a.up)
        {
            let p = Plan::new(&n, &a.interface, &a.address, false).unwrap();
            let mut result = None;
            run(p, |s| result = Some(s), || false);
            let result = result.unwrap();
            assert!(matches!(result.stage.as_str(), "completed" | "partial"));
            assert!(result.devices.iter().any(|d| d.source == "local"));
        }
    }
}
