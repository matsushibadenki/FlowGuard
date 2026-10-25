//! Display-only projection. Never changes policy, observations or shared selection.
use crate::model::{enforceable, Connection, Rule};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Intent {
    #[default]
    All,
    None,
    Allow,
    Block,
    Ask,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Demo,
    Live,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    #[serde(default)]
    pub search_names: bool,
    pub search: String,
    pub intent: Intent,
    pub hide_loopback: bool,
    pub source: Source,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            search_names: false,
            search: String::new(),
            intent: Intent::All,
            hide_loopback: true,
            source: Source::Demo,
        }
    }
}
impl Filter {
    pub fn valid(&self) -> bool {
        self.search.encode_utf16().count() <= 512 && !self.search.contains('\0')
    }
    pub fn matches_source(&self, mode: &str) -> bool {
        mode == self.source.as_str()
    }
    pub fn project(&self, connections: Vec<Connection>, rules: &[Rule]) -> Vec<Connection> {
        self.project_with_names(connections, rules, &[])
    }
    pub fn project_with_names(
        &self,
        connections: Vec<Connection>,
        rules: &[Rule],
        names: &[crate::flow_names::Row],
    ) -> Vec<Connection> {
        let search = self.search.to_lowercase();
        connections
            .into_iter()
            .filter(|c| {
                (!self.hide_loopback || !loopback(&c.destination))
                    && (search.is_empty() || {
                        let base = format!("{} {} {}", c.app, c.destination, c.app_key);
                        let extra = if self.search_names {
                            names
                                .iter()
                                .filter(|r| r.connection_id == c.id)
                                .flat_map(|r| r.values.iter().map(|v| v.value.as_str()))
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            String::new()
                        };
                        if extra.is_empty() {
                            base
                        } else {
                            format!("{base} {extra}")
                        }
                    }
                    .to_lowercase()
                    .contains(&search))
                    && (self.intent == Intent::All
                        || self.intent.as_str() == intent_for(rules, c, self.source.as_str()))
            })
            .collect()
    }
}
impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Self::Demo => "demo",
            Self::Live => "live",
        }
    }
}
impl Intent {
    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::None => "none",
            Self::Allow => "allow",
            Self::Block => "block",
            Self::Ask => "ask",
        }
    }
}
fn intent_for<'a>(rules: &'a [Rule], c: &Connection, source: &str) -> &'a str {
    if c.protocol != "TCP" {
        return "none";
    }
    rules
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.source == source
                && (if source == "demo" {
                    r.app_key == c.app_key
                } else {
                    enforceable(r) && r.identity == c.identity
                })
                && r.destination == c.destination
                && r.port == c.port
                && (r.direction == "both" || r.direction == c.direction)
        })
        .max_by_key(|(i, r)| (r.direction != "both", r.updated_at, std::cmp::Reverse(*i)))
        .map_or("none", |(_, r)| r.action.as_str())
}
fn v4(value: &str) -> Option<[u8; 4]> {
    let mut result = [0; 4];
    let mut parts = value.split('.');
    for n in &mut result {
        let p = parts.next()?;
        if p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *n = p.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(result)
}
/// Mirrors the normal map's loopback classification, including mapped IPv4 and zones.
fn loopback(address: &str) -> bool {
    let mut value = address;
    if value.starts_with('[') && value.ends_with(']') {
        value = &value[1..value.len() - 1];
    }
    if let Some((host, zone)) = value.split_once('%') {
        if !host.contains(':')
            || zone.is_empty()
            || !zone
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        {
            return false;
        }
        value = host;
    }
    if let Some(v) = v4(value) {
        return v[0] == 127;
    }
    let normalized = if value.contains('.') {
        let Some((prefix, suffix)) = value.rsplit_once(':') else {
            return false;
        };
        let Some(v) = v4(suffix) else {
            return false;
        };
        format!(
            "{prefix}:{:x}:{:x}",
            u16::from_be_bytes([v[0], v[1]]),
            u16::from_be_bytes([v[2], v[3]])
        )
    } else {
        value.to_owned()
    };
    normalized.parse::<std::net::Ipv6Addr>().is_ok_and(|ip| {
        ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|v| v.octets()[0] == 127)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_fixtures_match_the_normal_web_map() {
        #[derive(Deserialize)]
        struct Case {
            filter: Filter,
            expected: Vec<String>,
        }
        #[derive(Deserialize)]
        struct Fixture {
            connections: Vec<Connection>,
            rules: Vec<Rule>,
            cases: Vec<Case>,
        }
        let fixture: Fixture =
            serde_json::from_str(include_str!("../../src/core/fixtures/gpu-filters.json")).unwrap();
        for case in fixture.cases {
            assert_eq!(
                case.filter
                    .project(fixture.connections.clone(), &fixture.rules)
                    .iter()
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>(),
                case.expected,
                "{:?}",
                case.filter
            );
        }
    }
    #[test]
    fn loopbacks_match_normal_map_variants_and_preserve_lan() {
        for a in [
            "127.0.0.1",
            "127.255.2.3",
            "127.000.0.1",
            "::1",
            "[::1]",
            "::1%lo0",
            "[::ffff:127.0.0.1]",
            "::ffff:7f00:1",
            "::ffff:127.000.0.1",
        ] {
            assert!(loopback(a), "{a}");
        }
        for a in [
            "192.168.1.1",
            "localhost",
            "127.1",
            "127.0.0.999",
            "::127.0.0.1",
            "fe80::1%en0",
            "::1%",
            "::1%bad zone",
            "[::1",
            "::1::",
            "127.0.0.1%en0",
        ] {
            assert!(!loopback(a), "{a}");
        }
    }
    #[test]
    fn validates_closed_commands_and_search_bounds() {
        let mut f = Filter::default();
        assert!(f.valid());
        f.search = "😀".repeat(256);
        assert!(f.valid());
        f.search.push('x');
        assert!(!f.valid());
        f.search = "a\0b".into();
        assert!(!f.valid());
        let value = serde_json::to_value(Filter::default()).unwrap();
        for (key, bad) in [
            ("intent", serde_json::json!("bogus")),
            ("source", serde_json::json!("other")),
            ("hideLoopback", serde_json::json!("true")),
            ("ids", serde_json::json!([])),
        ] {
            let mut v = value.clone();
            v[key] = bad;
            assert!(serde_json::from_value::<Filter>(v).is_err());
        }
    }
    #[test]
    fn projects_search_intent_and_loopback_as_intersection() {
        let mut rows = crate::model::demo_connections();
        for c in &mut rows {
            c.destination = "192.0.2.1".into();
        }
        rows[0].destination = "127.0.0.1".into();
        let c = &rows[1];
        let rule = Rule {
            transport: "TCP".into(),
            id: "rule".into(),
            app_key: c.app_key.clone(),
            app: c.app.clone(),
            destination: c.destination.clone(),
            port: c.port,
            direction: "both".into(),
            action: "block".into(),
            source: "demo".into(),
            updated_at: 1,
            identity: None,
            timeout_action: "allow".into(),
            system_exception: false,
        };
        let f = Filter {
            intent: Intent::Block,
            ..Default::default()
        };
        assert_eq!(f.project(rows.clone(), &[rule.clone()]).len(), 1);
        let f = Filter {
            search: c.app.to_uppercase(),
            ..f
        };
        assert_eq!(f.project(rows.clone(), &[rule.clone()]).len(), 1);
        assert!(Filter {
            search: "no match".into(),
            ..f
        }
        .project(rows.clone(), &[rule.clone()])
        .is_empty());
        assert_eq!(Filter::default().project(rows.clone(), &[]).len(), 3);
        assert_eq!(
            Filter {
                hide_loopback: false,
                ..Default::default()
            }
            .project(rows.clone(), &[])
            .len(),
            4
        );
        assert_eq!(
            Filter {
                intent: Intent::None,
                ..Default::default()
            }
            .project(rows, &[rule])
            .len(),
            2
        );
    }
    #[test]
    fn intent_priority_is_specific_then_newest_then_first_and_never_enforcement() {
        let mut c = crate::model::demo_connections().remove(0);
        c.direction = "outbound".into();
        let mut r = Rule {
            transport: "TCP".into(),
            id: "r".into(),
            app_key: c.app_key.clone(),
            app: c.app.clone(),
            destination: c.destination.clone(),
            port: c.port,
            direction: "both".into(),
            action: "block".into(),
            source: "demo".into(),
            updated_at: 100,
            identity: None,
            timeout_action: "allow".into(),
            system_exception: false,
        };
        let mut specific = r.clone();
        specific.direction = "outbound".into();
        specific.action = "ask".into();
        specific.updated_at = 1;
        assert_eq!(
            intent_for(&[r.clone(), specific.clone()], &c, "demo"),
            "ask"
        );
        r.direction = "outbound".into();
        assert_eq!(
            intent_for(&[r.clone(), specific.clone()], &c, "demo"),
            "block"
        );
        specific.updated_at = 100;
        assert_eq!(
            intent_for(&[specific.clone(), r.clone()], &c, "demo"),
            "ask"
        );
        c.protocol = "UDP".into();
        assert_eq!(intent_for(&[r.clone()], &c, "demo"), "none");
        c.protocol = "TCP".into();
        assert_eq!(intent_for(&[r.clone()], &c, "live"), "none");
        r.source = "live".into();
        assert_eq!(intent_for(&[r], &c, "live"), "none");
    }
}
