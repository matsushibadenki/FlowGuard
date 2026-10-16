use crate::model::Snapshot;
use std::collections::HashSet;
/// Session-only, bounded observation classifier. Never makes a policy decision.
#[derive(Default)]
pub struct Tracker {
    last: Option<u64>,
    network: Vec<String>,
    baseline: bool,
    seen: HashSet<String>,
    candidates: HashSet<String>,
}
impl Tracker {
    pub fn tick(&mut self, s: &Snapshot) -> usize {
        if !s.document.preferences.unknown_notifications
            || s.document.preferences.mode != "live"
            || s.firewall.error.is_some()
            || s.firewall.observer != "limited"
        {
            self.last = None;
            self.baseline = false;
            self.candidates.clear();
            return 0;
        }
        let mut network: Vec<_> = s
            .network
            .addresses
            .iter()
            .map(|a| format!("{}:{}:{}:{}", a.interface, a.address, a.netmask, a.up))
            .collect();
        network.sort();
        if network != self.network {
            self.baseline = false;
            self.candidates.clear();
            self.network = network;
        }
        let Some(sample) = s.last_observed else {
            return 0;
        };
        if self.last == Some(sample) {
            return 0;
        }
        self.last = Some(sample);
        let current: HashSet<_> = s
            .connections
            .iter()
            .filter_map(|c| {
                let identity = c.identity.as_ref().filter(|i| i.valid())?;
                let address = c.destination.parse::<std::net::IpAddr>().ok()?;
                if c.protocol != "TCP"
                    || address.is_loopback()
                    || address.is_multicast()
                    || address.is_unspecified()
                {
                    return None;
                }
                let key = format!(
                    "{}:{}:{}:{}",
                    identity.key(),
                    c.destination,
                    c.port,
                    c.direction
                );
                if crate::system_protection::preset(
                    Some(identity),
                    c.port,
                    &c.direction,
                    &c.protocol,
                )
                .is_some()
                {
                    if self.seen.len() < 1024 {
                        self.seen.insert(key);
                    }
                    return None;
                }
                // Observation cannot infer direction: any stored direction counts as an existing intention.
                if s.document.rules.iter().any(|r| {
                    crate::model::enforceable(r)
                        && r.identity.as_ref() == Some(identity)
                        && r.destination == c.destination
                        && r.port == c.port
                        && (c.direction == "unknown"
                            || r.direction == "both"
                            || r.direction == c.direction)
                }) {
                    if self.seen.len() < 1024 {
                        self.seen.insert(key);
                    }
                    return None;
                }
                Some(key)
            })
            .take(1024)
            .collect();
        if !self.baseline {
            for key in current {
                if self.seen.len() < 1024 {
                    self.seen.insert(key);
                }
            }
            self.baseline = true;
            self.candidates.clear();
            return 0;
        }
        let stable: Vec<_> = current
            .intersection(&self.candidates)
            .filter(|k| !self.seen.contains(*k))
            .cloned()
            .collect();
        self.candidates = current
            .into_iter()
            .filter(|k| !self.seen.contains(k))
            .collect();
        let mut count = 0;
        for key in stable {
            if self.seen.len() >= 1024 {
                break;
            }
            self.seen.insert(key);
            count += 1;
        }
        count
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        let d = tempfile::tempdir().unwrap();
        let st = crate::state::SharedState::load(d.path().join("doc.json")).unwrap();
        let mut s = st.snapshot().unwrap();
        s.document.preferences.mode = "live".into();
        s.document.preferences.unknown_notifications = true;
        s.firewall.error = None;
        s.firewall.observer = "limited".into();
        s.last_observed = Some(1);
        s.connections = vec![];
        s
    }
    fn connection() -> crate::model::Connection {
        serde_json::from_value(serde_json::json!({"identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"},"id":"1","app":"Test","appKey":"/app","pid":1,"local":"127.0.0.1:12","destination":"1.1.1.1","port":443,"protocol":"TCP","direction":"unknown","observedAt":1})).unwrap()
    }
    #[test]
    fn capacity_and_interface_change_rebaseline_silently() {
        let mut t = Tracker::default();
        let mut s = fixture();
        s.connections = (0..1024)
            .map(|i| {
                let mut c = connection();
                c.port = (i + 1) as u16;
                c
            })
            .collect();
        assert_eq!(t.tick(&s), 0);
        s.connections = vec![connection()];
        s.connections[0].port = 60000;
        s.last_observed = Some(2);
        t.tick(&s);
        s.last_observed = Some(3);
        assert_eq!(t.tick(&s), 0);
        assert!(t.seen.len() <= 1024);
        let mut t = Tracker::default();
        s.connections.clear();
        t.tick(&s);
        s.connections = vec![connection()];
        s.last_observed = Some(4);
        t.tick(&s);
        s.network.addresses.push(crate::network::InterfaceAddress {
            interface: "en0".into(),
            address: "192.168.1.2".into(),
            family: "ipv4".into(),
            netmask: "255.255.255.0".into(),
            up: true,
        });
        s.last_observed = Some(5);
        assert_eq!(t.tick(&s), 0);
        s.last_observed = Some(6);
        assert_eq!(t.tick(&s), 0);
    }
    #[test]
    fn baseline_two_samples_dedupe_and_same_snapshot_do_not_alert() {
        let mut t = Tracker::default();
        let mut s = fixture();
        assert_eq!(t.tick(&s), 0);
        s.connections = vec![connection()];
        s.last_observed = Some(2);
        assert_eq!(t.tick(&s), 0);
        assert_eq!(t.tick(&s), 0);
        s.last_observed = Some(3);
        assert_eq!(t.tick(&s), 1);
        s.last_observed = Some(4);
        assert_eq!(t.tick(&s), 0);
    }
    #[test]
    fn errors_and_muting_rebaseline_without_a_flood() {
        let mut t = Tracker::default();
        let mut s = fixture();
        t.tick(&s);
        s.connections = vec![connection()];
        s.last_observed = Some(2);
        t.tick(&s);
        s.firewall.error = Some("failed".into());
        assert_eq!(t.tick(&s), 0);
        s.firewall.error = None;
        s.last_observed = Some(3);
        assert_eq!(t.tick(&s), 0);
        s.last_observed = Some(4);
        assert_eq!(t.tick(&s), 0);
        s.document.preferences.unknown_notifications = false;
        assert_eq!(t.tick(&s), 0);
    }
    #[test]
    fn unsigned_loopback_and_existing_rules_are_not_unknown() {
        let mut t = Tracker::default();
        let mut s = fixture();
        t.tick(&s);
        let mut c = connection();
        c.identity = None;
        s.connections = vec![c];
        s.last_observed = Some(2);
        t.tick(&s);
        s.last_observed = Some(3);
        assert_eq!(t.tick(&s), 0);
        s.connections = vec![connection()];
        let c = &s.connections[0];
        s.document.rules=serde_json::from_value(serde_json::json!([{"id":"r","appKey":c.identity.as_ref().unwrap().key(),"identity":c.identity,"app":"Test","destination":"1.1.1.1","port":443,"direction":"outbound","action":"block","source":"live","updatedAt":1}])).unwrap();
        s.last_observed = Some(4);
        t.tick(&s);
        s.last_observed = Some(5);
        assert_eq!(t.tick(&s), 0);
        s.document.rules.clear();
        s.last_observed = Some(6);
        t.tick(&s);
        s.last_observed = Some(7);
        assert_eq!(t.tick(&s), 0);
    }
    #[test]
    fn loopback_protected_and_short_lived_are_silent() {
        let mut t = Tracker::default();
        let mut s = fixture();
        t.tick(&s);
        let mut c = connection();
        c.destination = "127.0.0.1".into();
        s.connections = vec![c];
        s.last_observed = Some(2);
        t.tick(&s);
        s.last_observed = Some(3);
        assert_eq!(t.tick(&s), 0);
        let mut c = connection();
        c.identity = Some(crate::identity::SigningIdentity {
            authority: "apple".into(),
            team_id: "".into(),
            identifier: "com.apple.mDNSResponder".into(),
        });
        c.port = 53;
        s.connections = vec![c];
        s.last_observed = Some(4);
        t.tick(&s);
        s.last_observed = Some(5);
        assert_eq!(t.tick(&s), 0);
        s.connections = vec![connection()];
        s.last_observed = Some(6);
        t.tick(&s);
        s.connections.clear();
        s.last_observed = Some(7);
        assert_eq!(t.tick(&s), 0);
    }
}
