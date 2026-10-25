//! Separate v1 display-only flow-name contract; never changes identity, endpoint or policy.
use crate::{
    identity::SigningIdentity,
    model::{now, Connection},
    network::NetworkSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    pub value: String,
    pub observed_at: u64,
    pub source: &'static str,
    pub trust: &'static str,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub connection_id: String,
    pub values: Vec<Name>,
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub enabled: bool,
    pub operation: u64,
    pub busy: bool,
    pub search_enabled: bool,
    pub rows: Vec<Row>,
    pub error: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    local_address: String,
    local_port: u16,
    remote_address: String,
    remote_port: u16,
    transport: String,
    direction: String,
    identity: SigningIdentity,
    name: String,
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
    rows: Vec<Record>,
}
#[derive(Clone, Serialize)]
pub struct Query {
    pub version: u32,
    #[serde(rename = "requestID")]
    pub request_id: String,
    #[serde(rename = "instanceID")]
    pub instance_id: String,
    pub enabled: bool,
}
#[derive(Clone)]
pub struct Ticket {
    pub operation: u64,
    pub query: Query,
    network: String,
}
#[derive(Default)]
pub struct Machine {
    enabled: bool,
    operation: u64,
    search: bool,
    rows: Vec<Row>,
    network: Option<String>,
    instance: Option<String>,
    running: Option<u64>,
    last: Option<Instant>,
    error: Option<String>,
}
impl Machine {
    pub fn configure(
        &mut self,
        enabled: bool,
        search: bool,
        expected: u64,
        mode: &str,
        network: &NetworkSnapshot,
        instance: Option<&str>,
    ) -> Result<(), String> {
        if expected != self.operation {
            return Err("stale_name_operation".into());
        }
        let key = crate::destination_names::network_key(network);
        if enabled && (mode != "live" || key.is_none() || instance.is_none()) {
            return Err("flow_names_unavailable".into());
        }
        self.operation += 1;
        self.enabled = enabled;
        self.search = enabled && search;
        if !enabled || !self.context(mode, network, instance) {
            self.rows.clear();
        }
        self.error = None;
        self.network = key;
        self.instance = instance.map(str::to_owned);
        Ok(())
    }
    pub fn observe(
        &mut self,
        mode: &str,
        network: &NetworkSnapshot,
        instance: Option<&str>,
        connections: &[Connection],
    ) {
        if !self.context(mode, network, instance) {
            if self.enabled || !self.rows.is_empty() {
                self.operation += 1;
                self.enabled = false;
                self.search = false;
                self.rows.clear();
            }
        }
        self.rows
            .retain(|r| connections.iter().any(|c| c.id == r.connection_id));
        for row in &mut self.rows {
            row.values
                .retain(|v| v.observed_at <= now() && now() - v.observed_at <= 15);
        }
        self.rows.retain(|r| !r.values.is_empty());
    }
    fn context(&self, mode: &str, network: &NetworkSnapshot, instance: Option<&str>) -> bool {
        self.enabled
            && mode == "live"
            && self.network == crate::destination_names::network_key(network)
            && self.instance.as_deref() == instance
    }
    pub fn snapshot(
        &self,
        mode: &str,
        network: &NetworkSnapshot,
        instance: Option<&str>,
    ) -> Snapshot {
        let enabled = self.context(mode, network, instance);
        Snapshot {
            enabled,
            operation: self.operation,
            busy: self.running.is_some(),
            search_enabled: enabled && self.search,
            rows: if enabled {
                self.rows
                    .iter()
                    .filter_map(|r| {
                        let values = r
                            .values
                            .iter()
                            .filter(|v| v.observed_at <= now() && now() - v.observed_at <= 15)
                            .cloned()
                            .collect::<Vec<_>>();
                        (!values.is_empty()).then(|| Row {
                            connection_id: r.connection_id.clone(),
                            values,
                        })
                    })
                    .collect()
            } else {
                vec![]
            },
            error: self.error.clone(),
        }
    }
    pub fn begin(
        &mut self,
        expected: u64,
        mode: &str,
        network: &NetworkSnapshot,
        instance: Option<&str>,
    ) -> Result<Ticket, String> {
        if expected != self.operation || !self.context(mode, network, instance) {
            return Err("stale_name_operation".into());
        }
        if self.running.is_some()
            || self
                .last
                .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
        {
            return Err("flow_names_busy".into());
        }
        self.last = Some(Instant::now());
        self.running = Some(self.operation);
        self.error = None;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let hex = format!("{:032x}", stamp ^ std::process::id() as u128);
        let request_id = format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        );
        Ok(Ticket {
            operation: self.operation,
            query: Query {
                version: 1,
                request_id,
                instance_id: self.instance.clone().unwrap(),
                enabled: true,
            },
            network: self.network.clone().unwrap(),
        })
    }
    pub fn finish(
        &mut self,
        t: &Ticket,
        result: Result<Reply, String>,
        mode: &str,
        network: &NetworkSnapshot,
        instance: Option<&str>,
        connections: &[Connection],
    ) {
        if self.running != Some(t.operation) {
            return;
        }
        self.running = None;
        if t.operation != self.operation
            || !self.context(mode, network, instance)
            || self.network.as_deref() != Some(t.network.as_str())
        {
            return;
        }
        let reply = match result {
            Ok(r)
                if r.version == 1
                    && r.request_id == t.query.request_id
                    && r.instance_id == t.query.instance_id
                    && r.rows.len() <= 128 =>
            {
                r
            }
            _ => {
                self.error = Some("flow_names_unavailable".into());
                return;
            }
        };
        let mut values: Vec<Row> = vec![];
        for r in reply.rows {
            if r.transport != "TCP"
                || !r.identity.valid()
                || !["inbound", "outbound"].contains(&r.direction.as_str())
                || r.observed_at > now()
                || now() - r.observed_at > 15
                || crate::tls_details::hostname(&r.name)
                    .ok()
                    .flatten()
                    .is_none()
            {
                continue;
            }
            let (Ok(local), Ok(remote)) = (
                r.local_address.parse::<IpAddr>(),
                r.remote_address.parse::<IpAddr>(),
            ) else {
                continue;
            };
            for c in connections {
                let Ok(endpoint) = c.local.parse::<SocketAddr>() else {
                    continue;
                };
                if c.protocol != "TCP"
                    || c.identity.as_ref() != Some(&r.identity)
                    || endpoint.ip().to_canonical() != local.to_canonical()
                    || endpoint.port() != r.local_port
                    || c.destination
                        .parse::<IpAddr>()
                        .ok()
                        .map(|a| a.to_canonical())
                        != Some(remote.to_canonical())
                    || c.port != r.remote_port
                    || c.observed_at.abs_diff(r.observed_at) > 3
                    || c.direction != "unknown" && c.direction != r.direction
                {
                    continue;
                }
                let row = if let Some(i) = values.iter().position(|v| v.connection_id == c.id) {
                    &mut values[i]
                } else {
                    if values.len() >= 64 {
                        continue;
                    }
                    values.push(Row {
                        connection_id: c.id.clone(),
                        values: vec![],
                    });
                    values.last_mut().unwrap()
                };
                if row.values.len() < 4
                    && !row
                        .values
                        .iter()
                        .any(|v| v.value.eq_ignore_ascii_case(&r.name))
                {
                    row.values.push(Name {
                        value: r.name.to_lowercase(),
                        observed_at: r.observed_at,
                        source: "network_extension_remote_hostname",
                        trust: "unverified_tuple_time_candidate",
                    });
                }
            }
        }
        self.rows = values;
    }
}
pub fn query(contents: &std::path::Path, t: &Ticket) -> Result<Reply, String> {
    let binary = contents.join("MacOS/FlowGuardBridge");
    let input = serde_json::to_vec(&t.query).map_err(|_| "invalid_name_query")?;
    let (ok, output) = crate::observer::run_bounded_input(
        binary.to_str().ok_or("bundle_path")?,
        &["--flow-names"],
        Some(input),
    )?;
    if !ok || output.len() > 128 * 1024 {
        return Err("flow_names_unavailable".into());
    }
    serde_json::from_str(&output).map_err(|_| "invalid_flow_names".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn network() -> NetworkSnapshot {
        NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.0.2.2".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        }
    }
    #[test]
    fn names_require_signed_full_tuple_near_time_and_explicit_search_and_revocation_wins() {
        let network = network();
        let instance = "11111111-1111-4111-8111-111111111111";
        let mut m = Machine::default();
        m.configure(true, false, 0, "live", &network, Some(instance))
            .unwrap();
        let ticket = m.begin(1, "live", &network, Some(instance)).unwrap();
        let identity = SigningIdentity {
            authority: "developer".into(),
            team_id: "TESTTEAM01".into(),
            identifier: "com.example.flow".into(),
        };
        let mut c = crate::model::demo_connections().remove(0);
        c.identity = Some(identity.clone());
        c.protocol = "TCP".into();
        c.local = "192.0.2.2:50000".into();
        c.destination = "192.0.2.1".into();
        c.port = 443;
        c.observed_at = now();
        let record = Record {
            local_address: "192.0.2.2".into(),
            local_port: 50000,
            remote_address: "192.0.2.1".into(),
            remote_port: 443,
            transport: "TCP".into(),
            direction: "outbound".into(),
            identity,
            name: "one.example".into(),
            observed_at: now(),
        };
        let mut second = record.clone();
        second.name = "two.example".into();
        let mut wrong = record.clone();
        wrong.local_port = 50001;
        m.finish(
            &ticket,
            Ok(Reply {
                version: 1,
                request_id: ticket.query.request_id.clone(),
                instance_id: instance.into(),
                rows: vec![record.clone(), second, wrong],
            }),
            "live",
            &network,
            Some(instance),
            &[c.clone()],
        );
        let snapshot = m.snapshot("live", &network, Some(instance));
        assert_eq!(snapshot.rows[0].values.len(), 2);
        assert!(!snapshot.search_enabled);
        m.configure(true, true, 1, "live", &network, Some(instance))
            .unwrap();
        assert!(m.snapshot("live", &network, Some(instance)).search_enabled);
        assert_eq!(
            m.snapshot("live", &network, Some(instance)).rows[0]
                .values
                .len(),
            2
        );
        m.last = None;
        let stale = m.begin(2, "live", &network, Some(instance)).unwrap();
        m.configure(false, false, 2, "live", &network, Some(instance))
            .unwrap();
        m.finish(
            &stale,
            Ok(Reply {
                version: 1,
                request_id: stale.query.request_id.clone(),
                instance_id: instance.into(),
                rows: vec![record],
            }),
            "live",
            &network,
            Some(instance),
            &[c],
        );
        assert!(!m.snapshot("live", &network, Some(instance)).busy);
        assert!(m.snapshot("live", &network, Some(instance)).rows.is_empty());
        assert!(m.snapshot("demo", &network, Some(instance)).rows.is_empty());
    }
    #[test]
    fn no_implicit_consent_or_stale_reply_can_publish_names() {
        let mut m = Machine::default();
        let network = NetworkSnapshot::default();
        assert!(!m.snapshot("live", &network, None).enabled);
        assert!(m.begin(0, "live", &network, None).is_err());
        assert!(m.configure(true, false, 0, "demo", &network, None).is_err());
        m.configure(false, false, 0, "live", &network, None)
            .unwrap();
        assert_eq!(m.snapshot("live", &network, None).operation, 1);
        assert!(m
            .configure(false, false, 0, "live", &network, None)
            .is_err());
    }
}
