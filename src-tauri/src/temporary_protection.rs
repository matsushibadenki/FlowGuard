//! Ephemeral, explicit exact-session allow intentions, independently acknowledged by the Provider.
use crate::{identity::SigningIdentity, model::Connection, network::NetworkSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
#[repr(C)]
struct Timebase {
    numer: u32,
    denom: u32,
}
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn mach_timebase_info(info: *mut Timebase) -> i32;
    fn mach_continuous_time() -> u64;
}
pub fn clock() -> Result<(u64, u64), String> {
    #[cfg(target_os = "macos")]
    {
        let mut info = Timebase { numer: 0, denom: 0 };
        if unsafe { mach_timebase_info(&mut info) } != 0 || info.numer == 0 {
            return Err("clock_unavailable".into());
        }
        Ok((
            unsafe { mach_continuous_time() },
            1_000_000_000u64 * info.denom as u64 / info.numer as u64,
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("clock_unavailable".into())
    }
}
fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn nonce() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        ^ std::process::id() as u128;
    let s = format!("{t:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    )
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    #[serde(rename = "flowID")]
    pub flow_id: String,
    pub id: String,
    pub identity: SigningIdentity,
    pub transport: &'static str,
    pub direction: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub interface_name: String,
    pub network_scope: String,
    pub peer_evidence: String,
    pub deadline: u64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub connection_id: String,
    pub app: String,
    pub local: String,
    pub remote: String,
    pub interface_name: String,
    pub peer_evidence: String,
    pub remaining_seconds: u16,
    pub received: bool,
}
#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub operation: u64,
    pub busy: bool,
    pub received: bool,
    pub error: Option<String>,
    pub rows: Vec<Row>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    version: u32,
    #[serde(rename = "requestID")]
    request_id: String,
    #[serde(rename = "instanceID")]
    instance_id: String,
    expected_revision: Option<u64>,
    grants: Option<Vec<Grant>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ObservedFlow {
    #[serde(rename = "flowID")]
    flow_id: String,
    identity: SigningIdentity,
    direction: String,
    local_address: String,
    local_port: u16,
    remote_address: String,
    remote_port: u16,
    interface_name: String,
    observed_at: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    version: u32,
    #[serde(rename = "requestID")]
    request_id: String,
    #[serde(rename = "instanceID")]
    instance_id: String,
    revision: u64,
    accepted: bool,
    #[serde(rename = "grantIDs")]
    grant_ids: Vec<String>,
    flows: Vec<ObservedFlow>,
    lease_deadline: u64,
    error: Option<String>,
}
#[derive(Clone)]
pub struct Ticket {
    pub operation: u64,
    pub query: Query,
}
struct Entry {
    connection: Connection,
    grant: Grant,
}
#[derive(Default)]
pub struct Machine {
    entries: Vec<Entry>,
    operation: u64,
    instance: Option<String>,
    network: Option<String>,
    provider_revision: Option<u64>,
    lease: u64,
    acked: Option<u64>,
    running: Option<u64>,
    last: Option<Instant>,
    dirty: bool,
    error: Option<String>,
    failures: u8,
}
fn peer(value: &str) -> bool {
    if let Some(v) = value.strip_prefix("tls:") {
        crate::tls_details::hostname(v).ok().flatten().is_some()
    } else if let Some(v) = value.strip_prefix("ssh:SHA256:") {
        v.len() == 43
            && v.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/".contains(&b))
    } else {
        false
    }
}
impl Machine {
    pub fn add(
        &mut self,
        c: Connection,
        network: &NetworkSnapshot,
        instance: &str,
        direction: &str,
        evidence: &str,
        seconds: u16,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed
            || !peer(evidence)
            || seconds == 0
            || seconds > 600
            || c.protocol != "TCP"
            || !c.identity.as_ref().is_some_and(|v| v.valid())
            || !["inbound", "outbound"].contains(&direction)
        {
            return Err("invalid_temporary_scope".into());
        }
        let local = c
            .local
            .parse::<SocketAddr>()
            .map_err(|_| "invalid_temporary_scope")?;
        let remote = c
            .destination
            .parse::<IpAddr>()
            .map_err(|_| "invalid_temporary_scope")?;
        if local.port() == 0 {
            return Err("invalid_temporary_scope".into());
        }
        let interfaces: std::collections::BTreeSet<_> = network
            .addresses
            .iter()
            .filter(|a| {
                a.up && a
                    .address
                    .split('%')
                    .next()
                    .and_then(|v| v.parse::<IpAddr>().ok())
                    .map(|v| v.to_canonical())
                    == Some(local.ip().to_canonical())
            })
            .map(|a| a.interface.clone())
            .collect();
        if interfaces.len() != 1 {
            return Err("ambiguous_source_interface".into());
        }
        let key = crate::destination_names::network_key(network).ok_or("network_changed")?;
        let (now, second) = clock()?;
        if self.instance.as_deref() != Some(instance) || self.network.as_deref() != Some(&key) {
            self.clear();
            self.instance = Some(instance.into());
            self.network = Some(key.clone());
            self.provider_revision = None;
        }
        self.entries.retain(|e| e.connection.id != c.id);
        if self.entries.len() >= 16 {
            return Err("temporary_capacity".into());
        }
        use sha2::{Digest, Sha256};
        let scope = format!("{:x}", Sha256::digest(key.as_bytes()));
        let grant = Grant {
            flow_id: String::new(),
            id: nonce(),
            identity: c.identity.clone().unwrap(),
            transport: "TCP",
            direction: direction.into(),
            local_address: local.ip().to_canonical().to_string(),
            local_port: local.port(),
            remote_address: remote.to_canonical().to_string(),
            remote_port: c.port,
            interface_name: interfaces.into_iter().next().unwrap(),
            network_scope: scope,
            peer_evidence: evidence.into(),
            deadline: now
                .checked_add(seconds as u64 * second)
                .ok_or("clock_unavailable")?,
        };
        self.provider_revision = None;
        self.entries.push(Entry {
            connection: c,
            grant,
        });
        self.changed();
        Ok(())
    }
    fn changed(&mut self) {
        self.operation += 1;
        self.dirty = true;
        self.acked = None;
        self.error = None;
        self.failures = 0;
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lease = 0;
        self.changed();
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if !self.entries.iter().any(|e| e.grant.id == id) {
            return Err("stale_temporary_grant".into());
        }
        self.entries.retain(|e| e.grant.id != id);
        self.changed();
        Ok(())
    }
    pub fn observe(
        &mut self,
        network: &NetworkSnapshot,
        instance: Option<&str>,
        connections: &[Connection],
        live: bool,
    ) {
        let invalid = !live
            || self.instance.as_deref() != instance
            || self.network != crate::destination_names::network_key(network);
        let before = self.entries.len();
        let now = clock().map_or(u64::MAX, |v| v.0);
        self.entries.retain(|e| {
            !invalid
                && e.grant.deadline > now
                && connections.iter().any(|c| {
                    c.id == e.connection.id
                        && c.identity == e.connection.identity
                        && c.local == e.connection.local
                        && c.destination == e.connection.destination
                        && c.port == e.connection.port
                })
        });
        if before != self.entries.len() {
            self.changed();
        }
        if self.instance.as_deref() != instance {
            self.instance = instance.map(str::to_owned);
            self.provider_revision = None;
            self.lease = 0;
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let (now, second) = clock().unwrap_or((u64::MAX, 1));
        let received = self.acked == Some(self.operation) && now < self.lease;
        Snapshot {
            operation: self.operation,
            busy: self.running.is_some(),
            received,
            error: self.error.clone(),
            rows: self
                .entries
                .iter()
                .filter(|e| e.grant.deadline > now)
                .map(|e| Row {
                    id: e.grant.id.clone(),
                    connection_id: e.connection.id.clone(),
                    app: e.connection.app.clone(),
                    local: e.connection.local.clone(),
                    remote: format!("{}:{}", e.grant.remote_address, e.grant.remote_port),
                    interface_name: e.grant.interface_name.clone(),
                    peer_evidence: e.grant.peer_evidence.clone(),
                    remaining_seconds: ((e.grant.deadline - now) / second).min(600) as u16,
                    received,
                })
                .collect(),
        }
    }
    pub fn begin(&mut self) -> Option<Ticket> {
        if self.lease > 0 && clock().map_or(true, |v| v.0 >= self.lease) && !self.entries.is_empty()
        {
            self.clear();
        }
        if self.running.is_some()
            || self.failures >= 3
            || !self.dirty && self.entries.is_empty()
            || self
                .last
                .is_some_and(|v| v.elapsed() < Duration::from_secs(1))
        {
            return None;
        }
        let instance = self.instance.clone()?;
        self.last = Some(Instant::now());
        self.running = Some(self.operation);
        Some(Ticket {
            operation: self.operation,
            query: Query {
                version: 1,
                request_id: nonce(),
                instance_id: instance,
                expected_revision: self.provider_revision,
                grants: self
                    .provider_revision
                    .map(|_| self.entries.iter().map(|e| e.grant.clone()).collect()),
            },
        })
    }
    pub fn finish(&mut self, ticket: &Ticket, result: Result<Reply, String>) {
        if self.running != Some(ticket.operation) {
            return;
        }
        self.running = None;
        if self.operation != ticket.operation
            || self.instance.as_deref() != Some(&ticket.query.instance_id)
        {
            return;
        }
        if self.lease > 0 && clock().map_or(true, |v| v.0 >= self.lease) && !self.entries.is_empty()
        {
            self.clear();
            return;
        }
        let reply = match result {
            Ok(r)
                if r.version == 1
                    && r.request_id == ticket.query.request_id
                    && r.instance_id == ticket.query.instance_id
                    && r.grant_ids.len() <= 16 =>
            {
                r
            }
            _ => {
                self.error = Some("temporary_unavailable".into());
                self.failures += 1;
                return;
            }
        };
        if !reply.accepted || reply.error.is_some() {
            self.provider_revision = None;
            self.error = Some("temporary_unconfirmed".into());
            self.failures += 1;
            return;
        }
        if ticket.query.expected_revision.is_none() {
            if reply.flows.len() > 128 {
                self.error = Some("temporary_unconfirmed".into());
                self.failures += 1;
                return;
            }
            for entry in &mut self.entries {
                if !entry.grant.flow_id.is_empty() {
                    continue;
                }
                let matches: Vec<_> = reply
                    .flows
                    .iter()
                    .filter(|f| {
                        valid_uuid(&f.flow_id)
                            && f.identity == entry.grant.identity
                            && f.direction == entry.grant.direction
                            && f.local_address == entry.grant.local_address
                            && f.local_port == entry.grant.local_port
                            && f.remote_address == entry.grant.remote_address
                            && f.remote_port == entry.grant.remote_port
                            && f.interface_name == entry.grant.interface_name
                            && f.observed_at.abs_diff(entry.connection.observed_at) <= 3
                    })
                    .collect();
                if matches.len() != 1 {
                    self.error = Some("temporary_flow_unverified".into());
                    self.failures += 1;
                    return;
                }
                entry.grant.flow_id = matches[0].flow_id.clone();
            }
        }
        if let Some(expected) = ticket.query.expected_revision {
            let ids: Vec<_> = ticket
                .query
                .grants
                .as_ref()
                .unwrap()
                .iter()
                .map(|g| g.id.clone())
                .collect();
            let (now, second) = clock().unwrap_or((u64::MAX, 1));
            if expected.checked_add(1) != Some(reply.revision)
                || ids != reply.grant_ids
                || (!ids.is_empty() && reply.lease_deadline <= now)
                || reply.lease_deadline > now.saturating_add(5 * second)
            {
                self.error = Some("temporary_unconfirmed".into());
                self.provider_revision = None;
                self.failures += 1;
                return;
            }
            self.acked = Some(self.operation);
            self.lease = reply.lease_deadline;
            self.dirty = false;
        }
        self.provider_revision = Some(reply.revision);
        self.error = None;
        self.failures = 0;
    }
}
pub fn transact(contents: &std::path::Path, t: &Ticket) -> Result<Reply, String> {
    let input = serde_json::to_vec(&t.query).map_err(|_| "invalid_temporary_query")?;
    let path = contents.join("MacOS/FlowGuardBridge");
    let (ok, data) = crate::observer::run_bounded_input(
        path.to_str().ok_or("bundle_path")?,
        &["--temporary-grants"],
        Some(input),
    )?;
    if !ok || data.len() > 128 * 1024 {
        return Err("temporary_unavailable".into());
    }
    serde_json::from_str(&data).map_err(|_| "temporary_unconfirmed".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grants_are_explicit_bounded_and_never_restore_after_removal() {
        let mut m = Machine::default();
        let mut c = crate::model::demo_connections().remove(0);
        c.local = "192.0.2.2:50000".into();
        c.destination = "192.0.2.1".into();
        c.identity = Some(SigningIdentity {
            authority: "developer".into(),
            team_id: "TESTTEAM01".into(),
            identifier: "com.example.test".into(),
        });
        let network = NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.0.2.2".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let peer = "tls:example.com";
        assert!(m
            .add(
                c.clone(),
                &network,
                "instance",
                "outbound",
                peer,
                600,
                false
            )
            .is_err());
        assert!(m
            .add(c.clone(), &network, "instance", "outbound", peer, 601, true)
            .is_err());
        m.add(c.clone(), &network, "instance", "outbound", peer, 60, true)
            .unwrap();
        assert_eq!(m.snapshot().rows.len(), 1);
        assert!(!m.snapshot().received);
        let ticket = m.begin().unwrap();
        m.clear();
        m.finish(&ticket, Err("late".into()));
        assert!(m.snapshot().rows.is_empty());
        assert!(!m.snapshot().busy);
        assert!(m.remove("unknown").is_err());
    }
    #[test]
    fn acknowledgment_expires_and_late_success_cannot_restore_revoked_or_restarted_grants() {
        let mut m = Machine::default();
        let mut c = crate::model::demo_connections().remove(0);
        c.local = "192.0.2.2:50000".into();
        c.destination = "192.0.2.1".into();
        c.identity = Some(SigningIdentity {
            authority: "developer".into(),
            team_id: "TESTTEAM01".into(),
            identifier: "com.example.test".into(),
        });
        let network = NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.0.2.2".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        m.add(
            c.clone(),
            &network,
            "boot",
            "outbound",
            "tls:example.test",
            60,
            true,
        )
        .unwrap();
        let read = m.begin().unwrap();
        let g = &m.entries[0].grant;
        let observation = ObservedFlow {
            flow_id: nonce(),
            identity: g.identity.clone(),
            direction: g.direction.clone(),
            local_address: g.local_address.clone(),
            local_port: g.local_port,
            remote_address: g.remote_address.clone(),
            remote_port: g.remote_port,
            interface_name: g.interface_name.clone(),
            observed_at: c.observed_at,
        };
        m.finish(
            &read,
            Ok(Reply {
                version: 1,
                request_id: read.query.request_id.clone(),
                instance_id: "boot".into(),
                revision: 0,
                accepted: true,
                grant_ids: vec![],
                flows: vec![observation],
                lease_deadline: 0,
                error: None,
            }),
        );
        assert!(!m.entries[0].grant.flow_id.is_empty());
        assert!(!m.snapshot().received);
        m.last = None;
        let write = m.begin().unwrap();
        let ids = write
            .query
            .grants
            .as_ref()
            .unwrap()
            .iter()
            .map(|g| g.id.clone())
            .collect::<Vec<_>>();
        let (now, second) = clock().unwrap();
        let reply = || Reply {
            version: 1,
            request_id: write.query.request_id.clone(),
            instance_id: "boot".into(),
            revision: 1,
            accepted: true,
            grant_ids: ids.clone(),
            flows: vec![],
            lease_deadline: now + second,
            error: None,
        };
        m.finish(&write, Ok(reply()));
        assert!(m.snapshot().received);
        m.lease = now - 1;
        assert!(!m.snapshot().received);
        m.last = None;
        let revoke = m.begin().unwrap();
        assert!(m.snapshot().rows.is_empty());
        m.finish(&revoke, Err("offline".into()));
        m.add(
            c.clone(),
            &network,
            "boot",
            "outbound",
            "tls:example.test",
            60,
            true,
        )
        .unwrap();
        m.last = None;
        let late = m.begin().unwrap();
        m.clear();
        m.finish(
            &late,
            Ok(Reply {
                request_id: late.query.request_id.clone(),
                revision: 2,
                ..reply()
            }),
        );
        assert!(m.snapshot().rows.is_empty());
        assert!(!m.snapshot().received);
        m.add(
            c.clone(),
            &network,
            "boot",
            "outbound",
            "tls:example.test",
            60,
            true,
        )
        .unwrap();
        m.observe(&network, Some("new_boot"), &[c], true);
        assert!(m.snapshot().rows.is_empty());
        assert!(!valid_uuid("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"));
        assert!(valid_uuid(&nonce()));
    }
    #[test]
    fn peer_evidence_is_bounded_typed_and_not_inferred_from_a_port() {
        for v in [
            "",
            "example.com",
            "tls:127.0.0.1",
            "tls:<script>",
            "ssh:SHA256:short",
        ] {
            assert!(!peer(v));
        }
        assert!(peer("tls:example.com"));
        assert!(peer(&format!("ssh:SHA256:{}", "A".repeat(43))));
    }
}
