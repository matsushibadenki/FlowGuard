//! Read-only assessment of existing evidence. No network requests or rule changes.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub port: u16,
    pub category: String,
    pub severity: String,
    pub certainty: String,
    pub evidence: String,
    pub recommendation: String,
    pub observed_at: u64,
    pub change: String,
    pub disposition: Disposition,
    pub reason: String,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    #[default]
    Open,
    Resolved,
    Recheck,
    OutOfScope,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub id: u64,
    pub service_operation: u64,
    pub generation: u64,
    pub target: String,
    pub interface: String,
    pub created_at: u64,
    pub findings: Vec<Finding>,
}
#[derive(Clone, Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub reports: Vec<Report>,
    pub selected_report: Option<u64>,
    pub selected_finding: Option<String>,
}
impl Snapshot {
    pub fn assess(&mut self, s: &crate::services::Snapshot, now: u64) -> Result<(), String> {
        if s.operation == 0
            || s.stage != "completed"
            || s.error.is_some()
            || s.rows.is_empty()
            || s.rows.len() > 64
        {
            return Err("unfinished_services".into());
        }
        let id = self.reports.first().map_or(1, |r| r.id + 1);
        let mut findings = vec![];
        for row in &s.rows {
            let mut add = |category: &str,
                           severity: &str,
                           certainty: &str,
                           evidence: String,
                           recommendation: &str| {
                findings.push(Finding {
                    id: format!("{}:{category}", row.port),
                    port: row.port,
                    category: category.into(),
                    severity: severity.into(),
                    certainty: certainty.into(),
                    evidence,
                    recommendation: recommendation.into(),
                    observed_at: row.observed_at,
                    change: "uncompared".into(),
                    disposition: Disposition::Open,
                    reason: String::new(),
                })
            };
            let confirmed = row.status == "confirmed";
            add(
                "service",
                "info",
                if confirmed { "observed" } else { "unverified" },
                format!(
                    "method={};status={};service={}",
                    row.method,
                    row.status,
                    row.service.as_deref().unwrap_or("unknown")
                ),
                "review_service_scope",
            );
            if row.method == "https" {
                let details = row.tls_details.as_ref();
                let verified = details.is_some_and(|t| t.verification == "verified");
                add(
                    "tls",
                    if verified { "info" } else { "warning" },
                    if verified { "observed" } else { "unverified" },
                    format!(
                        "server_name={};verification={};reason={}",
                        details.map_or(s.target.as_str(), |t| t.server_name.as_str()),
                        details.map_or("not_completed", |t| t.verification.as_str()),
                        details.map_or("not_completed", |t| t.reason.as_str())
                    ),
                    "review_tls",
                );
                if let Some(end) = details
                    .and_then(|d| d.certificate.as_ref())
                    .and_then(|c| c.not_after)
                {
                    let days = (end as i128 - now as i128) / 86400;
                    add(
                        "certificate_expiry",
                        if days <= 30 { "warning" } else { "info" },
                        "observed",
                        format!("not_after={end};days_remaining={days}"),
                        "review_certificate_expiry",
                    );
                } else {
                    add(
                        "certificate_expiry",
                        "info",
                        "not_performed",
                        "certificate_metadata_unavailable".into(),
                        "recheck_tls",
                    );
                }
            }
            if row.method == "http" || row.method == "https" {
                add(
                    "http_headers",
                    "info",
                    "not_performed",
                    "service_observation_does_not_preserve_all_headers".into(),
                    "collect_http_evidence",
                );
            }
            if confirmed
                && matches!(
                    row.service.as_deref(),
                    Some("SSH" | "ssh" | "SMTP" | "smtp")
                )
            {
                add(
                    "management_reachability",
                    "info",
                    "observed",
                    format!(
                        "reachable_from={};interface={};service={}",
                        s.source_address,
                        s.interface,
                        row.service.as_deref().unwrap()
                    ),
                    "review_intended_exposure",
                );
            }
        }
        if let Some(previous) = self.reports.iter().find(|r| {
            r.target == s.target && r.interface == s.interface && r.generation == s.generation
        }) {
            for f in &mut findings {
                f.change = match previous.findings.iter().find(|p| p.id == f.id) {
                    Some(p) if p.evidence == f.evidence => "unchanged",
                    Some(_) => "changed",
                    None => "new",
                }
                .into();
            }
        }
        self.reports.insert(
            0,
            Report {
                id,
                service_operation: s.operation,
                generation: s.generation,
                target: s.target.clone(),
                interface: s.interface.clone(),
                created_at: now,
                findings,
            },
        );
        self.reports.truncate(20);
        self.selected_report = Some(id);
        self.selected_finding = None;
        Ok(())
    }
    pub fn select(&mut self, report: u64, finding: Option<String>) -> Result<(), String> {
        let r = self
            .reports
            .iter()
            .find(|r| r.id == report)
            .ok_or("stale_report")?;
        if finding
            .as_ref()
            .is_some_and(|id| !r.findings.iter().any(|f| &f.id == id))
        {
            return Err("stale_finding".into());
        }
        self.selected_report = Some(report);
        self.selected_finding = finding;
        Ok(())
    }
    pub fn disposition(
        &mut self,
        report: u64,
        finding: &str,
        value: Disposition,
        reason: &str,
    ) -> Result<(), String> {
        if reason.len() > 1024
            || reason.chars().any(|c| c.is_control())
            || value == Disposition::OutOfScope && reason.trim().is_empty()
        {
            return Err("diagnostic_reason".into());
        }
        let f = self
            .reports
            .iter_mut()
            .find(|r| r.id == report)
            .and_then(|r| r.findings.iter_mut().find(|f| f.id == finding))
            .ok_or("stale_finding")?;
        f.disposition = value;
        f.reason = reason.into();
        Ok(())
    }
    pub fn report(&self, id: u64) -> Result<&Report, String> {
        self.reports
            .iter()
            .find(|r| r.id == id)
            .ok_or("stale_report".into())
    }
}
fn csv_cell(s: &str) -> String {
    let protected = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", protected.replace('"', "\"\""))
}
fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn export(r: &Report, format: &str) -> Result<String, String> {
    let headers = [
        "target",
        "interface",
        "report",
        "service_operation",
        "port",
        "category",
        "severity",
        "certainty",
        "evidence",
        "recommendation",
        "observed_at",
        "change",
        "disposition",
        "reason",
    ];
    let rows: Vec<Vec<String>> = r
        .findings
        .iter()
        .map(|f| {
            vec![
                r.target.clone(),
                r.interface.clone(),
                r.id.to_string(),
                r.service_operation.to_string(),
                f.port.to_string(),
                f.category.clone(),
                f.severity.clone(),
                f.certainty.clone(),
                f.evidence.clone(),
                f.recommendation.clone(),
                f.observed_at.to_string(),
                f.change.clone(),
                serde_json::to_value(f.disposition)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
                f.reason.clone(),
            ]
        })
        .collect();
    match format {"csv"=>Ok(std::iter::once(headers.iter().map(|s|csv_cell(s)).collect::<Vec<_>>().join(",")).chain(rows.iter().map(|row|row.iter().map(|s|csv_cell(s)).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join("\r\n")),"html"=>Ok(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>FlowGuard observations</title><style>body{{font:14px system-ui}}td,th{{border:1px solid #888;padding:8px;overflow-wrap:anywhere}}table{{border-collapse:collapse}}</style><h1>FlowGuard observations</h1><p>Read-only evidence assessment. Unverified and not-performed checks are not proof of safety. Maximum 20 session reports; no automatic policy changes.</p><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></html>",headers.iter().map(|h|format!("<th>{}</th>",html(h))).collect::<String>(),rows.iter().map(|row|format!("<tr>{}</tr>",row.iter().map(|s|format!("<td>{}</td>",html(s))).collect::<String>())).collect::<String>())),_=>Err("invalid_report_format".into())}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unperformed_is_never_safe_and_reports_are_bounded() {
        let mut s = Snapshot::default();
        let mut job = crate::services::Snapshot {
            operation: 1,
            stage: "completed".into(),
            target: "192.0.2.1".into(),
            ..Default::default()
        };
        job.rows.push(crate::services::Row {
            port: 443,
            candidate: "https".into(),
            service: None,
            status: "tls_error".into(),
            method: "https".into(),
            product: None,
            response: None,
            tls: None,
            tls_details: None,
            observed_at: 1,
        });
        for _ in 0..25 {
            s.assess(&job, 2).unwrap();
        }
        assert_eq!(s.reports.len(), 20);
        let r = &s.reports[0];
        assert!(r.findings.iter().all(|f| f.certainty != "safe"));
        assert!(r.findings.iter().any(|f| f.certainty == "not_performed"));
        assert!(r.findings.iter().all(|f| f.change == "unchanged"));
        job.stage = "cancelled".into();
        assert!(s.assess(&job, 2).is_err());
    }
    #[test]
    fn export_escapes_untrusted_evidence_and_edits_are_checked() {
        let mut s = Snapshot::default();
        s.reports.push(Report {
            id: 1,
            service_operation: 1,
            generation: 1,
            target: "<script>x</script>".into(),
            interface: "en0".into(),
            created_at: 0,
            findings: vec![Finding {
                id: "f".into(),
                port: 1,
                category: "service".into(),
                severity: "info".into(),
                certainty: "unverified".into(),
                evidence: "=SUM(1)".into(),
                recommendation: "review".into(),
                observed_at: 0,
                change: "new".into(),
                disposition: Disposition::Open,
                reason: String::new(),
            }],
        });
        assert!(s.disposition(1, "f", Disposition::OutOfScope, "").is_err());
        s.disposition(1, "f", Disposition::OutOfScope, "accepted scope")
            .unwrap();
        assert!(export(&s.reports[0], "html")
            .unwrap()
            .contains("&lt;script&gt;"));
        assert!(export(&s.reports[0], "csv").unwrap().contains("'=SUM"));
        assert!(s.select(2, None).is_err());
    }
}
