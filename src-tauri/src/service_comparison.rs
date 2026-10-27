//! Compare bounded service observations only within an identical collection scope.
use crate::services::{Plan, Row, Snapshot};
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub status: String,
    pub baseline_operation: Option<u64>,
    pub baseline_port_operation: Option<u64>,
    pub rows: Vec<Change>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub port: u16,
    pub kind: String,
    pub fields: Vec<String>,
    pub previous: Option<Row>,
}
#[derive(Clone, PartialEq, Eq)]
struct Scope {
    target: std::net::IpAddr,
    local: std::net::IpAddr,
    interface: String,
    netmask: String,
    generation: u64,
    ports: Vec<u16>,
    mode: String,
    hostname: Option<String>,
}
impl Scope {
    fn new(p: &Plan) -> Self {
        let mut ports = p.tcp.ports.clone();
        ports.sort_unstable();
        Self {
            target: p.tcp.target,
            local: p.tcp.local,
            interface: p.tcp.interface.clone(),
            netmask: p.tcp.netmask.clone(),
            generation: p.tcp.generation,
            ports,
            mode: p.mode.clone(),
            hostname: p.hostname.clone(),
        }
    }
}
#[derive(Default)]
pub struct Comparator {
    baseline: Option<(Scope, Snapshot)>,
    scope_changed: bool,
    invalidated: bool,
}
fn fields(old: &Row, row: &Row) -> Vec<String> {
    [
        ("status", old.status != row.status),
        ("method", old.method != row.method),
        ("service", old.service != row.service),
        ("product", old.product != row.product),
        ("response", old.response != row.response),
        ("tls", old.tls != row.tls),
        ("tls_details", old.tls_details != row.tls_details),
        (
            "http_headers",
            old.http_headers
                .as_ref()
                .zip(row.http_headers.as_ref())
                .is_some_and(|(a, b)| {
                    a.content_security_policy != b.content_security_policy
                        || a.x_content_type_options != b.x_content_type_options
                        || a.x_content_type_options_value != b.x_content_type_options_value
                        || a.referrer_policy != b.referrer_policy
                        || a.strict_transport_security != b.strict_transport_security
                }),
        ),
        (
            "http_redirect",
            row.method == "http"
                && old
                    .http_headers
                    .as_ref()
                    .zip(row.http_headers.as_ref())
                    .is_some_and(|(a, b)| a.redirect != b.redirect),
        ),
    ]
    .into_iter()
    .filter(|(_, changed)| *changed)
    .map(|(field, _)| field.into())
    .collect()
}
fn confirmed(row: &Row) -> bool {
    row.status == "confirmed"
        && row.service.is_some()
        && (!matches!(row.method.as_str(), "http" | "https")
            || row.http_headers.as_ref().is_some_and(|h| {
                h.x_content_type_options_value != "ambiguous"
                    && (row.method != "http" || h.redirect != "unknown")
            }))
        && (row.method != "https"
            || row
                .tls_details
                .as_ref()
                .is_some_and(|t| t.verification == "verified"))
}
impl Comparator {
    pub fn begin(&mut self, p: &Plan) -> Comparison {
        self.scope_changed = self
            .baseline
            .as_ref()
            .is_some_and(|(scope, _)| *scope != Scope::new(p));
        if self.scope_changed {
            self.baseline = None;
        }
        self.invalidated = false;
        Comparison {
            status: "running".into(),
            baseline_operation: self.baseline.as_ref().map(|(_, s)| s.operation),
            baseline_port_operation: self.baseline.as_ref().map(|(_, s)| s.port_operation),
            rows: vec![],
        }
    }
    pub fn invalidate(&mut self) {
        self.baseline = None;
        self.invalidated = true;
    }
    pub fn update(&mut self, p: &Plan, s: &mut Snapshot) {
        if self.invalidated || s.error.as_deref() == Some("network_changed") {
            self.invalidate();
            s.comparison = Comparison {
                status: "invalidated".into(),
                ..Default::default()
            };
            return;
        }
        let expected = p.snapshot();
        let full = s.stage == "completed"
            && s.error.is_none()
            && s.operation > 0
            && s.port_operation == p.port_operation
            && s.completed == p.tcp.ports.len()
            && s.total == p.tcp.ports.len()
            && (1..=64).contains(&s.total)
            && s.rows.len() == s.total
            && s.target == expected.target
            && s.source_address == expected.source_address
            && s.source_netmask == expected.source_netmask
            && s.interface == expected.interface
            && s.family == expected.family
            && s.generation == expected.generation
            && s.mode == expected.mode
            && s.hostname == expected.hostname
            && s.requested_ports == expected.requested_ports
            && p.tcp
                .ports
                .iter()
                .all(|port| s.rows.iter().filter(|r| r.port == *port).count() == 1);
        let previous = self
            .baseline
            .as_ref()
            .filter(|(scope, _)| *scope == Scope::new(p))
            .map(|(_, s)| s);
        let changes = s
            .rows
            .iter()
            .map(|r| {
                let old = previous.and_then(|s| s.rows.iter().find(|old| old.port == r.port));
                let fields = if full {
                    old.map(|old| fields(old, r)).unwrap_or_default()
                } else {
                    vec![]
                };
                let kind = if !full {
                    "uncompared"
                } else if let Some(old) = old {
                    if !confirmed(old) || !confirmed(r) {
                        "uncertain"
                    } else if fields.is_empty() {
                        "unchanged"
                    } else {
                        "changed"
                    }
                } else {
                    "uncompared"
                };
                Change {
                    port: r.port,
                    kind: kind.into(),
                    fields,
                    previous: old.cloned(),
                }
            })
            .collect::<Vec<_>>();
        let status = if matches!(s.stage.as_str(), "running" | "cancelling") {
            "running"
        } else if !full {
            "incomplete"
        } else if previous.is_none() {
            if self.scope_changed {
                "scope_changed"
            } else {
                "first"
            }
        } else if changes.iter().any(|r| r.kind == "uncertain") {
            "uncertain"
        } else {
            "ready"
        };
        s.comparison = Comparison {
            status: status.into(),
            baseline_operation: previous.map(|s| s.operation),
            baseline_port_operation: previous.map(|s| s.port_operation),
            rows: changes,
        };
        if full {
            let mut baseline = s.clone();
            baseline.comparison = Default::default();
            baseline.selected_port = None;
            self.baseline = Some((Scope::new(p), baseline));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> Plan {
        Plan {
            tcp: crate::ports::Plan {
                settings: Default::default(),
                target: "192.168.1.30".parse().unwrap(),
                local: "192.168.1.20".parse().unwrap(),
                interface: "en0".into(),
                netmask: "255.255.255.0".into(),
                generation: 3,
                ports: vec![22, 443],
                selection: "custom".into(),
            },
            port_operation: 7,
            mode: "auto".into(),
            hostname: Some("service.example.test".into()),
        }
    }
    fn complete(p: &Plan, op: u64) -> Snapshot {
        let mut s = p.snapshot();
        s.operation = op;
        s.stage = "completed".into();
        s.completed = p.tcp.ports.len();
        s.rows = p
            .tcp
            .ports
            .iter()
            .map(|port| Row {
                port: *port,
                candidate: "candidate".into(),
                service: Some(if *port == 22 { "SSH" } else { "HTTPS" }.into()),
                status: "confirmed".into(),
                method: if *port == 22 { "ssh" } else { "https" }.into(),
                product: Some("Example/1".into()),
                response: Some("response".into()),
                tls: if *port == 443 {
                    Some("TLS 1.3".into())
                } else {
                    None
                },
                tls_details: if *port == 443 {
                    Some(crate::tls_details::Details {
                        server_name: "service.example.test".into(),
                        verification: "verified".into(),
                        reason: "verified".into(),
                        certificate: Some(crate::tls_details::Certificate {
                            issuer: Some("Example CA".into()),
                            not_before: Some(100),
                            not_after: Some(1000),
                        }),
                    })
                } else {
                    None
                },
                http_headers: if *port == 443 {
                    Some(crate::services::HeaderPresence {
                        content_security_policy: true,
                        x_content_type_options: true,
                        x_content_type_options_value: "nosniff".into(),
                        referrer_policy: false,
                        strict_transport_security: true,
                        response_status: 200,
                        redirect: "none".into(),
                    })
                } else {
                    None
                },
                observed_at: 100,
            })
            .collect();
        s
    }
    fn primed(p: &Plan) -> Comparator {
        let mut c = Comparator::default();
        c.begin(p);
        c.update(p, &mut complete(p, 1));
        c
    }
    #[test]
    fn compares_observed_fields_not_timestamps_and_exports_previous_evidence() {
        let p = plan();
        let mut c = primed(&p);
        c.begin(&p);
        let mut s = complete(&p, 2);
        s.rows.iter_mut().for_each(|r| r.observed_at = 200);
        c.update(&p, &mut s);
        assert_eq!(s.comparison.status, "ready");
        assert!(s.comparison.rows.iter().all(|r| r.kind == "unchanged"));
        assert_eq!(s.comparison.baseline_operation, Some(1));
        c.begin(&p);
        let mut s = complete(&p, 3);
        s.rows[0].product = Some("Example/2".into());
        s.rows[0].response = Some("=formula,<script>".into());
        s.rows[1]
            .tls_details
            .as_mut()
            .unwrap()
            .certificate
            .as_mut()
            .unwrap()
            .not_after = Some(2000);
        c.update(&p, &mut s);
        assert_eq!(s.comparison.rows[0].fields, vec!["product", "response"]);
        assert_eq!(s.comparison.rows[1].fields, vec!["tls_details"]);
        assert!(s.comparison.rows.iter().all(|r| r.kind == "changed"));
        c.begin(&p);
        let mut s = complete(&p, 4);
        c.update(&p, &mut s);
        let csv = crate::services::csv(&s, 3, 7);
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("previous_tls_details_json"));
        assert!(csv.contains("\"'=formula,<script>\""));
        assert!(csv.contains("Example/2"));
        assert!(csv.contains("notAfter"));
        assert_eq!(csv.lines().count(), 3);
        assert!(c.baseline.as_ref().unwrap().1.comparison.rows.is_empty());
    }
    #[test]
    fn http_evidence_changes_and_unknowns_are_not_misreported_as_unchanged() {
        let p = plan();
        let mut c = primed(&p);
        c.begin(&p);
        let mut s = complete(&p, 2);
        s.rows[1]
            .http_headers
            .as_mut()
            .unwrap()
            .content_security_policy = false;
        c.update(&p, &mut s);
        assert_eq!(s.comparison.rows[1].kind, "changed");
        assert_eq!(s.comparison.rows[1].fields, vec!["http_headers"]);
        let csv = crate::services::csv(&s, 3, 7);
        assert!(csv.contains("http_evidence_json,previous_http_evidence_json"));
        assert!(csv.contains("contentSecurityPolicy"));
        c.begin(&p);
        let mut missing = complete(&p, 3);
        missing.rows[1].http_headers = None;
        c.update(&p, &mut missing);
        assert_eq!(missing.comparison.rows[1].kind, "uncertain");
        c.begin(&p);
        let mut recovered = complete(&p, 4);
        c.update(&p, &mut recovered);
        assert_eq!(recovered.comparison.rows[1].kind, "uncertain");
        let mut old = complete(&p, 5).rows.remove(1);
        old.method = "http".into();
        let mut new = old.clone();
        new.http_headers.as_mut().unwrap().redirect = "https".into();
        assert_eq!(fields(&old, &new), vec!["http_redirect"]);
        let mut mime_changed = old.clone();
        mime_changed
            .http_headers
            .as_mut()
            .unwrap()
            .x_content_type_options_value = "other".into();
        assert_eq!(fields(&old, &mime_changed), vec!["http_headers"]);
        mime_changed
            .http_headers
            .as_mut()
            .unwrap()
            .x_content_type_options_value = "ambiguous".into();
        assert!(!confirmed(&mime_changed));

        assert!(confirmed(&new));
        new.http_headers.as_mut().unwrap().redirect = "unknown".into();
        assert!(!confirmed(&new));
    }
    #[test]
    fn uncertain_responses_are_never_unchanged_or_health_proof() {
        let p = plan();
        for status in [
            "timeout",
            "tls_error",
            "denied",
            "error",
            "unsupported",
            "unrecognized",
        ] {
            let mut c = primed(&p);
            c.begin(&p);
            let mut s = complete(&p, 2);
            s.rows[1].status = status.into();
            c.update(&p, &mut s);
            assert_eq!(s.comparison.status, "uncertain");
            assert_eq!(s.comparison.rows[1].kind, "uncertain");
            c.begin(&p);
            s.operation = 3;
            c.update(&p, &mut s);
            assert_eq!(s.comparison.rows[1].kind, "uncertain");
            c.begin(&p);
            let mut recovered = complete(&p, 4);
            c.update(&p, &mut recovered);
            assert_eq!(recovered.comparison.rows[1].kind, "uncertain");
        }
        let mut c = primed(&p);
        c.begin(&p);
        let mut s = complete(&p, 2);
        s.rows[1].tls_details = None;
        c.update(&p, &mut s);
        assert_eq!(s.comparison.rows[1].kind, "uncertain");
    }
    #[test]
    fn partial_cancelled_failed_and_malformed_results_do_not_replace_baseline() {
        let p = plan();
        for case in 0..8 {
            let mut c = primed(&p);
            c.begin(&p);
            let mut partial = complete(&p, 2);
            match case {
                0 => partial.stage = "cancelled".into(),
                1 => partial.stage = "failed".into(),
                2 => {
                    partial.rows.pop();
                    partial.completed = 1;
                }
                3 => partial.rows[1] = partial.rows[0].clone(),
                4 => partial.target = "192.168.1.99".into(),
                5 => partial.operation = 0,
                6 => partial.port_operation += 1,
                _ => partial.error = Some("collection_failed".into()),
            };
            c.update(&p, &mut partial);
            assert_eq!(partial.comparison.status, "incomplete");
            assert!(partial
                .comparison
                .rows
                .iter()
                .all(|r| r.kind == "uncompared" && r.fields.is_empty()));
            c.begin(&p);
            let mut next = complete(&p, 3);
            c.update(&p, &mut next);
            assert_eq!(next.comparison.baseline_operation, Some(1));
        }
    }
    #[test]
    fn scope_changes_and_invalidations_forget_the_old_baseline() {
        let p = plan();
        for case in 0..8 {
            let mut c = primed(&p);
            let mut changed = p.clone();
            match case {
                0 => changed.tcp.target = "192.168.1.31".parse().unwrap(),
                1 => changed.tcp.local = "192.168.1.21".parse().unwrap(),
                2 => changed.tcp.interface = "en1".into(),
                3 => changed.tcp.netmask = "255.255.255.128".into(),
                4 => changed.tcp.generation += 1,
                5 => changed.tcp.ports = vec![443],
                6 => changed.mode = "https".into(),
                _ => changed.hostname = Some("other.example.test".into()),
            };
            assert_eq!(c.begin(&changed).baseline_operation, None);
            let mut s = complete(&changed, 2);
            c.update(&changed, &mut s);
            assert_eq!(s.comparison.status, "scope_changed");
            assert!(s.comparison.rows.iter().all(|r| r.previous.is_none()));
        }
        let mut c = primed(&p);
        c.begin(&p);
        c.invalidate();
        let mut late = complete(&p, 2);
        c.update(&p, &mut late);
        assert_eq!(late.comparison.status, "invalidated");
        assert!(c.baseline.is_none());
        assert_eq!(c.begin(&p).baseline_operation, None);
        // A new TCP check is comparable only when its full service scope still matches.
        let mut c = primed(&p);
        let mut next = p.clone();
        next.port_operation = 8;
        next.tcp.ports.reverse();
        c.begin(&next);
        let mut s = complete(&next, 2);
        c.update(&next, &mut s);
        assert_eq!(s.comparison.baseline_port_operation, Some(7));
        assert_eq!(s.comparison.status, "ready");
    }
}
