use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub app: String,
    pub app_key: String,
    pub pid: u32,
    pub local: String,
    pub destination: String,
    pub port: u16,
    pub protocol: String,
    pub direction: String,
    pub observed_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub app_key: String,
    pub app: String,
    pub destination: String,
    pub port: u16,
    pub direction: String,
    pub action: String,
    pub updated_at: u64,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub locale: String,
    pub view: String,
    pub panel: String,
    pub mode: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            locale: "ja".into(),
            view: "map".into(),
            panel: "right".into(),
            mode: "live".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub rules: Vec<Rule>,
    pub preferences: Preferences,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallStatus {
    pub enabled: Option<bool>,
    pub enforcement: String,
    pub observer: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLayout {
    pub id: String,
    pub y: f32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationGroup {
    pub id: String,
    pub app: String,
    pub app_key: String,
    pub connection_ids: Vec<String>,
    pub process_ids: Vec<u32>,
    pub destinations: Vec<String>,
}
pub fn group_applications(connections: &[Connection]) -> Vec<ApplicationGroup> {
    let mut groups = std::collections::BTreeMap::<String, ApplicationGroup>::new();
    for c in connections {
        let id = if c.app_key.is_empty() {
            format!("unknown:{}", c.pid)
        } else {
            c.app_key.clone()
        };
        let group = groups
            .entry(id.clone())
            .or_insert_with(|| ApplicationGroup {
                id,
                app: c.app.clone(),
                app_key: c.app_key.clone(),
                connection_ids: vec![],
                process_ids: vec![],
                destinations: vec![],
            });
        group.connection_ids.push(c.id.clone());
        if !group.process_ids.contains(&c.pid) {
            group.process_ids.push(c.pid);
        }
        let destination = format!("{}:{}", c.destination, c.port);
        if !group.destinations.contains(&destination) {
            group.destinations.push(destination);
        }
    }
    let mut result: Vec<_> = groups.into_values().collect();
    result.sort_by(|a, b| {
        b.connection_ids
            .len()
            .cmp(&a.connection_ids.len())
            .then(a.id.cmp(&b.id))
    });
    result
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub network: crate::network::NetworkSnapshot,
    pub discovery: crate::discovery::DiscoverySnapshot,
    pub inventory: crate::inventory::InventorySnapshot,
    pub revision: u64,
    pub extension: crate::enforcement::ExtensionStatus,
    pub document: Document,
    pub connections: Vec<Connection>,
    pub applications: Vec<ApplicationGroup>,
    pub history: Vec<Connection>,
    pub selected_id: Option<String>,
    pub firewall: FirewallStatus,
    pub last_observed: Option<u64>,
    pub layout: Vec<NodeLayout>,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    Select {
        id: String,
    },
    SetView {
        view: String,
    },
    SetLocale {
        locale: String,
    },
    SetMode {
        mode: String,
    },
    SetPanel {
        panel: String,
    },
    SaveRule {
        #[serde(rename = "connectionId")]
        connection_id: String,
        action: String,
        direction: String,
    },
    DeleteRule {
        id: String,
    },
    RestoreRules {
        rules: Vec<Rule>,
    },
}

#[cfg(test)]
pub fn rule_for<'a>(rules: &'a [Rule], connection: &Connection, source: &str) -> Option<&'a Rule> {
    rules.iter().find(|r| {
        r.source == source
            && r.app_key == connection.app_key
            && r.destination == connection.destination
            && r.port == connection.port
            && (r.direction == "both" || r.direction == connection.direction)
    })
}
pub fn validate_rule(rule: &Rule) -> bool {
    ["allow", "block", "ask"].contains(&rule.action.as_str())
        && ["outbound", "inbound", "both"].contains(&rule.direction.as_str())
        && ["live", "demo"].contains(&rule.source.as_str())
        && !rule.app_key.is_empty()
        && rule.app_key.len() <= 4096
        && !rule.destination.is_empty()
        && rule.destination.len() <= 512
        && rule.port > 0
        && rule.app.len() <= 256
        && rule.id.len() <= 8192
}
pub fn demo_connections() -> Vec<Connection> {
    [
        ("Safari", "apple.com", 443),
        ("Mail", "icloud.com", 993),
        ("Dropbox", "dropbox.com", 443),
        ("Terminal", "192.168.1.12", 22),
    ]
    .iter()
    .enumerate()
    .map(|(i, (app, host, port))| Connection {
        id: format!("demo-{i}"),
        app: (*app).into(),
        app_key: format!("/Applications/{app}.app"),
        pid: 0,
        local: "127.0.0.1".into(),
        destination: (*host).into(),
        port: *port,
        protocol: "TCP".into(),
        direction: "outbound".into(),
        observed_at: now() - i as u64 * 60,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grouping_uses_executable_not_display_name_and_keeps_unknown_processes_separate() {
        let mut connections = demo_connections();
        let mut duplicate = connections[0].clone();
        duplicate.id = "second-safari".into();
        duplicate.pid = 123;
        connections.push(duplicate);
        let mut impostor = connections[0].clone();
        impostor.id = "different-binary".into();
        impostor.app_key = "/tmp/Safari".into();
        connections.push(impostor);
        let mut unknown = connections[0].clone();
        unknown.app_key.clear();
        unknown.pid = 91;
        unknown.id = "unknown-91".into();
        connections.push(unknown.clone());
        unknown.pid = 92;
        unknown.id = "unknown-92".into();
        connections.push(unknown);
        let groups = group_applications(&connections);
        assert_eq!(groups.len(), 7);
        assert_eq!(groups[0].connection_ids.len(), 2);
        assert_eq!(groups[0].process_ids.len(), 2);
        assert_eq!(groups[0].destinations.len(), 1);
    }
    #[test]
    fn rule_scope_includes_port_direction_and_source() {
        let c = demo_connections().remove(0);
        let mut r = Rule {
            id: "r".into(),
            app_key: c.app_key.clone(),
            app: c.app.clone(),
            destination: c.destination.clone(),
            port: c.port,
            direction: "inbound".into(),
            action: "block".into(),
            updated_at: 0,
            source: "demo".into(),
        };
        assert!(rule_for(&[r.clone()], &c, "demo").is_none());
        r.direction = "both".into();
        assert!(rule_for(&[r.clone()], &c, "demo").is_some());
        assert!(rule_for(&[r.clone()], &c, "live").is_none());
        r.port = 80;
        assert!(rule_for(&[r], &c, "demo").is_none());
    }
}
