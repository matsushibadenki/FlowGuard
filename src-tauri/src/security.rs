//! Read-only normalization. Socket samples are not packet/byte counters or proof of attack.
use crate::model::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Available,
    Unavailable,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterCapabilities {
    pub device_discovery: Capability,
    pub traffic_counters: Capability,
    pub flow_export: Capability,
    pub security_logs: Capability,
    pub firewall_read: Capability,
    pub firewall_write: Capability,
    pub device_quarantine: Capability,
    pub config_backup: Capability,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    MacSocket,
    Router,
    Demo,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFlow {
    pub id: String,
    pub source: ObservationSource,
    pub source_id: String,
    pub local_endpoint: String,
    pub remote_address: String,
    pub remote_port: u16,
    pub protocol: String,
    // Local/remote do not imply initiator/responder. lsof cannot establish direction.
    pub direction: String,
    pub app_key: Option<String>,
    pub pid: Option<u32>,
    pub first_observed_at: u64,
    pub sampled_at: Option<u64>,
    // None means unavailable, never zero traffic.
    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,
    pub counter_basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityEvent {
    pub id: String,
    pub source: ObservationSource,
    pub source_id: String,
    pub observed_at: u64,
    pub kind: String,
    pub evidence_flow_ids: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySnapshot {
    pub router_sources: Vec<crate::routers::SourceStatus>,
    pub correlations: Vec<crate::routers::Correlation>,
    pub flows: Vec<SecurityFlow>,
    pub events: Vec<SecurityEvent>,
    pub router_capabilities: RouterCapabilities,
}
pub fn normalize(
    connections: &[Connection],
    demo: bool,
    sampled_at: Option<u64>,
) -> SecuritySnapshot {
    let source_id = if demo { "demo" } else { "local-mac" };
    let mut seen = std::collections::HashSet::new();
    let flows = connections
        .iter()
        .filter(|c| seen.insert(c.id.clone()))
        .take(1024)
        .map(|c| SecurityFlow {
            id: format!("{source_id}:{}", c.id),
            source: if demo {
                ObservationSource::Demo
            } else {
                ObservationSource::MacSocket
            },
            source_id: source_id.into(),
            local_endpoint: c.local.clone(),
            remote_address: c.destination.clone(),
            remote_port: c.port,
            protocol: c.protocol.clone(),
            direction: "unknown".into(),
            app_key: (!c.app_key.is_empty()).then(|| c.app_key.clone()),
            pid: Some(c.pid),
            first_observed_at: c.observed_at,
            sampled_at: if demo { None } else { sampled_at },
            bytes_sent: None,
            bytes_received: None,
            counter_basis: "unavailable".into(),
        })
        .collect();
    SecuritySnapshot {
        flows,
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        Connection {
            identity: None,
            id: "c1".into(),
            app: "test".into(),
            app_key: "".into(),
            pid: 42,
            local: "[::1]:1000".into(),
            destination: "::1".into(),
            port: 443,
            protocol: "TCP".into(),
            direction: "outbound".into(),
            observed_at: 10,
        }
    }
    #[test]
    fn socket_limitations_and_dedup_are_preserved() {
        let c = connection();
        let s = normalize(&[c.clone(), c], false, Some(20));
        assert_eq!(s.flows.len(), 1);
        let f = &s.flows[0];
        assert_eq!(f.direction, "unknown");
        assert_eq!(f.bytes_sent, None);
        assert_eq!(f.app_key, None);
        assert_eq!(f.sampled_at, Some(20));
        assert_eq!(s.router_capabilities.firewall_write, Capability::Unknown);
        assert!(s.events.is_empty());
    }
    #[test]
    fn demo_is_separate_and_failed_observation_is_empty() {
        let live = normalize(&[connection()], false, Some(20));
        let demo = normalize(&[connection()], true, Some(20));
        assert_ne!(live.flows[0].id, demo.flows[0].id);
        assert_eq!(demo.flows[0].source, ObservationSource::Demo);
        assert_eq!(demo.flows[0].sampled_at, None);
        assert!(normalize(&[], false, Some(20)).flows.is_empty());
    }
}
