//! Opt-in, manual system reverse lookup. Display-only and never a policy identity.
use crate::{model::now, network::NetworkSnapshot};
use serde::Serialize;
use std::{
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const CAP: usize = 64;
const FRESH: Duration = Duration::from_secs(300);
const INTERVAL: Duration = Duration::from_secs(10);
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub address: String,
    pub value: Option<String>,
    pub source: &'static str,
    pub observed_at: u64,
    pub status: String,
    pub stale: bool,
    pub trust: &'static str,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub enabled: bool,
    pub operation: u64,
    pub busy: bool,
    pub cancelling: bool,
    pub active_address: Option<String>,
    pub cooldown_seconds: u64,
    pub rows: Vec<Row>,
}
#[derive(Clone)]
pub struct Ticket {
    pub operation: u64,
    pub address: String,
    pub ip: IpAddr,
    network: String,
    pub cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Machine {
    enabled: bool,
    operation: u64,
    running: Option<Ticket>,
    network: Option<String>,
    last_start: Option<Instant>,
    rows: Vec<(Row, Instant)>,
}
pub fn eligible(address: &str) -> Option<IpAddr> {
    let ip = address.parse::<IpAddr>().ok()?;
    let v4 = match ip {
        IpAddr::V4(v) => Some(v),
        IpAddr::V6(v) => v.to_ipv4_mapped(),
    };
    if let Some(v) = v4 {
        let first = v.octets()[0];
        if v.is_loopback() || first == 0 || first >= 224 {
            return None;
        }
    } else if let IpAddr::V6(v) = ip {
        if v.is_loopback() || v.is_unspecified() || v.is_multicast() || v.is_unicast_link_local() {
            return None;
        }
    }
    Some(ip)
}
pub fn network_key(network: &NetworkSnapshot) -> Option<String> {
    if network.error.is_some() {
        return None;
    }
    let mut addresses: Vec<_> = network
        .addresses
        .iter()
        .filter(|a| a.up)
        .map(|a| (&a.interface, &a.address, &a.netmask, &a.family))
        .collect();
    if addresses.is_empty() {
        return None;
    }
    addresses.sort();
    serde_json::to_string(&addresses).ok()
}
fn valid_name(raw: &str) -> Option<String> {
    let value = raw.strip_suffix('.').unwrap_or(raw);
    if value.is_empty() || value.len() > 253 || !value.is_ascii() || value.parse::<IpAddr>().is_ok()
    {
        return None;
    }
    value
        .split('.')
        .all(|s| {
            !s.is_empty()
                && s.len() <= 63
                && !s.starts_with('-')
                && !s.ends_with('-')
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .then(|| value.to_ascii_lowercase())
}
fn parse_response(text: &str, ip: IpAddr) -> (Option<String>, String) {
    for block in text.split("\n\n") {
        let matches = block.lines().any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                ["ip_address", "ipv6_address"].contains(&key.trim())
                    && value.trim().parse::<IpAddr>().ok() == Some(ip)
            })
        });
        if !matches {
            continue;
        }
        if let Some(raw) = block.lines().find_map(|line| line.strip_prefix("name:")) {
            return match valid_name(raw.trim()) {
                Some(name) => (Some(name), "resolved".into()),
                None => (None, "invalid_name".into()),
            };
        }
    }
    (None, "no_name".into())
}
pub fn resolve(ticket: &Ticket) -> (Option<String>, String) {
    match crate::observer::run_destination_lookup(ticket.ip, ticket.cancel.clone()) {
        Ok((true, text)) => parse_response(&text, ticket.ip),
        Err(e) if e == "observer_timeout" => (None, "timed_out".into()),
        _ => (None, "failed".into()),
    }
}
impl Ticket {
    pub fn matches_network(&self, network: &NetworkSnapshot) -> bool {
        network_key(network).as_deref() == Some(self.network.as_str())
    }
}
impl Machine {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            enabled: self.enabled,
            operation: self.operation,
            busy: self.running.is_some(),
            cancelling: self
                .running
                .as_ref()
                .is_some_and(|t| t.operation != self.operation),
            active_address: self
                .running
                .as_ref()
                .filter(|t| self.enabled && t.operation == self.operation)
                .map(|t| t.address.clone()),
            cooldown_seconds: self
                .last_start
                .map(|s| {
                    INTERVAL
                        .saturating_sub(s.elapsed())
                        .as_secs()
                        .saturating_add(u64::from(s.elapsed() < INTERVAL))
                })
                .unwrap_or(0),
            rows: if self.enabled {
                self.rows
                    .iter()
                    .map(|(r, time)| {
                        let mut row = r.clone();
                        row.stale = time.elapsed() >= FRESH;
                        row
                    })
                    .collect()
            } else {
                vec![]
            },
        }
    }
    pub fn configure(&mut self, enabled: bool, network: &NetworkSnapshot) -> Result<(), String> {
        if let Some(t) = &self.running {
            t.cancel.store(true, Ordering::SeqCst);
        }
        if enabled {
            self.network = Some(network_key(network).ok_or("network_unavailable")?);
        } else {
            self.network = None;
        }
        self.enabled = enabled;
        self.operation += 1;
        self.rows.clear();
        Ok(())
    }
    pub fn clear(&mut self, expected: u64) -> Result<(), String> {
        if expected != self.operation {
            return Err("name_lookup_stale".into());
        }
        if let Some(t) = &self.running {
            t.cancel.store(true, Ordering::SeqCst);
        }
        self.operation += 1;
        self.rows.clear();
        Ok(())
    }
    pub fn invalidate(&mut self) {
        if let Some(t) = &self.running {
            t.cancel.store(true, Ordering::SeqCst);
        }
        self.enabled = false;
        self.network = None;
        self.rows.clear();
        self.operation += 1;
    }
    pub fn observe(&mut self, network: &NetworkSnapshot) {
        if self.enabled && self.network != network_key(network) {
            self.invalidate();
        }
    }
    pub fn begin(&mut self, address: &str, network: &NetworkSnapshot) -> Result<Ticket, String> {
        if self.network != network_key(network) {
            return Err("network_changed".into());
        }
        if !self.enabled {
            return Err("name_lookup_consent_required".into());
        }
        if self.running.is_some() {
            return Err("name_lookup_busy".into());
        }
        if self.last_start.is_some_and(|s| s.elapsed() < INTERVAL) {
            return Err("name_lookup_cooldown".into());
        }
        let ip = eligible(address).ok_or("name_lookup_unsupported")?;
        let network = network_key(network).ok_or("network_unavailable")?;
        self.operation += 1;
        self.last_start = Some(Instant::now());
        let ticket = Ticket {
            operation: self.operation,
            address: address.into(),
            ip,
            network,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        self.running = Some(ticket.clone());
        // A refresh removes old evidence immediately; no stale result is presented as current.
        self.rows.retain(|(r, _)| r.address != address);
        Ok(ticket)
    }
    pub fn finish(
        &mut self,
        ticket: &Ticket,
        value: Option<String>,
        status: &str,
        network: &NetworkSnapshot,
    ) -> Result<(), String> {
        if !self
            .running
            .as_ref()
            .is_some_and(|t| t.operation == ticket.operation)
        {
            return Err("name_lookup_stale".into());
        }
        self.running = None;
        self.observe(network);
        if !self.enabled
            || self.operation != ticket.operation
            || self.network.as_deref() != Some(ticket.network.as_str())
        {
            return Ok(());
        }
        let value = value.and_then(|v| valid_name(&v));
        let status = if status == "resolved" && value.is_none() {
            "invalid_name"
        } else if ["resolved", "no_name", "invalid_name", "timed_out", "failed"].contains(&status) {
            status
        } else {
            "failed"
        };
        self.rows.push((
            Row {
                address: ticket.address.clone(),
                value: if status == "resolved" { value } else { None },
                source: "system_resolver",
                observed_at: now(),
                status: status.into(),
                stale: false,
                trust: "unverified_name",
            },
            Instant::now(),
        ));
        if self.rows.len() > CAP {
            self.rows.remove(0);
        }
        Ok(())
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn network() -> NetworkSnapshot {
        NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.2".into(),
                netmask: "255.255.255.0".into(),
                family: "IPv4".into(),
                up: true,
            }],
            observed_at: Some(now()),
            error: None,
        }
    }
    #[test]
    fn response_is_for_the_exact_ip_and_no_bad_name_is_trusted() {
        let ip = "192.0.2.1".parse().unwrap();
        assert_eq!(
            parse_response("name: Host.Example.\nip_address: 192.0.2.1", ip),
            (Some("host.example".into()), "resolved".into())
        );
        assert_eq!(
            parse_response("name: other.example\nip_address: 192.0.2.2", ip),
            (None, "no_name".into())
        );
        assert_eq!(
            parse_response("name: <script>x</script>\nip_address: 192.0.2.1", ip).1,
            "invalid_name"
        );
        assert_eq!(
            parse_response(
                "name: v6.example\nipv6_address: 2001:db8::1",
                "2001:db8::1".parse().unwrap()
            )
            .0,
            Some("v6.example".into())
        );
        assert!(valid_name("host.example..").is_none());
    }
    #[test]
    fn only_literal_unicast_destinations() {
        for good in ["192.168.1.1", "150.171.110.134", "2001:db8::1", "fd12::1"] {
            assert!(eligible(good).is_some());
        }
        for bad in [
            "host.example",
            "https://a",
            "127.0.0.1",
            "::1",
            "::",
            "224.0.0.1",
            "255.255.255.255",
            "0.0.0.1",
            "::ffff:127.0.0.1",
            "fe80::1",
            "fe80::1%en0",
            "--help",
        ] {
            assert!(eligible(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn names_are_bounded_literal_hostnames() {
        assert_eq!(valid_name("Host.Example."), Some("host.example".into()));
        for value in [
            "<script>x</script>",
            "https://example.com",
            "a\nb",
            "a\u{202e}b",
            "a..b",
            "-a.example",
            "192.0.2.1",
        ] {
            assert!(valid_name(value).is_none());
        }
        assert!(valid_name(&"x".repeat(254)).is_none());
    }
    #[test]
    fn revocation_clear_and_late_results_do_not_restore_names() {
        let n = network();
        let mut m = Machine::default();
        assert!(m.begin("192.0.2.1", &n).is_err());
        m.configure(true, &n).unwrap();
        assert!(!m.snapshot().busy);
        let t = m.begin("192.0.2.1", &n).unwrap();
        assert!(m.begin("192.0.2.2", &n).is_err());
        m.configure(false, &n).unwrap();
        m.configure(true, &n).unwrap();
        assert!(m.begin("192.0.2.2", &n).is_err());
        m.finish(&t, Some("late.example".into()), "resolved", &n)
            .unwrap();
        assert!(m.snapshot().rows.is_empty());
        m.last_start = None;
        let t = m.begin("192.0.2.1", &n).unwrap();
        m.clear(t.operation).unwrap();
        m.finish(&t, Some("late.example".into()), "resolved", &n)
            .unwrap();
        assert!(m.snapshot().rows.is_empty());
        assert!(m.clear(t.operation).is_err());
    }
    #[test]
    fn fresh_stale_failed_capacity_and_network_changes() {
        let n = network();
        let mut m = Machine::default();
        m.configure(true, &n).unwrap();
        for i in 1..=65 {
            m.last_start = None;
            let t = m.begin(&format!("192.0.2.{i}"), &n).unwrap();
            m.finish(&t, Some("test.example".into()), "resolved", &n)
                .unwrap();
        }
        assert_eq!(m.snapshot().rows.len(), 64);
        m.rows[0].1 = Instant::now() - FRESH;
        assert!(m.snapshot().rows[0].stale);
        assert!(m.begin("192.0.2.2", &n).is_err());
        let mut changed = n.clone();
        changed.addresses[0].address = "192.168.2.2".into();
        m.observe(&changed);
        assert!(!m.snapshot().enabled);
        assert!(m.snapshot().rows.is_empty());
        m.configure(true, &n).unwrap();
        m.last_start = None;
        let t = m.begin("192.0.2.1", &n).unwrap();
        m.finish(&t, Some("<html>".into()), "resolved", &n).unwrap();
        assert_eq!(m.snapshot().rows[0].status, "invalid_name");
        assert!(m.snapshot().rows[0].value.is_none());
    }
}
