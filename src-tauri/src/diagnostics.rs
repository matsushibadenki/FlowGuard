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
    pub evidence_source: String,
    pub impact: String,
    pub recommendation: String,
    pub observed_at: u64,
    pub change: String,
    pub disposition: Disposition,
    pub reason: String,
}
fn presence(value: bool) -> &'static str {
    if value {
        "present"
    } else {
        "not_observed"
    }
}
fn source_for(category: &str) -> &'static str {
    match category {
        "service" | "management_reachability" => "service_observation",
        "tls" => "tls_verification",
        "certificate_expiry" => "certificate_metadata",
        "http_headers" => "headers_not_collected",
        "http_redirect" | "mime_nosniff" => "headers_not_collected",
        _ => "unknown_source",
    }
}
fn impact_for(category: &str, severity: &str, certainty: &str) -> &'static str {
    match (category, severity, certainty) {
        ("service", _, "observed") => "service_scope_review",
        ("service", _, _) => "service_unverified",
        ("tls", _, "observed") => "tls_scope_limited",
        ("tls", _, _) => "tls_unverified_impact",
        ("certificate_expiry", _, "not_performed") => "expiry_unknown",
        ("certificate_expiry", "warning", _) => "expiry_attention",
        ("certificate_expiry", _, _) => "expiry_observed",
        ("mime_nosniff", _, "observed") => "mime_scope_limited",
        ("mime_nosniff", _, _) => "mime_unverified",
        ("http_redirect", _, "observed") => "redirect_scope_limited",
        ("http_redirect", _, _) => "redirect_unverified",
        ("http_headers", _, "observed") => "headers_presence_only",
        ("http_headers", _, _) => "headers_unknown",
        ("management_reachability", _, _) => "management_scope_review",
        _ => "impact_unknown",
    }
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonScope {
    target: String,
    source: String,
    interface: String,
    netmask: String,
    family: String,
    generation: u64,
    ports: Vec<u16>,
    mode: String,
    hostname: Option<String>,
}
impl ComparisonScope {
    fn from_job(s: &crate::services::Snapshot) -> Option<Self> {
        if s.target.is_empty()
            || s.source_address.is_empty()
            || s.interface.is_empty()
            || s.source_netmask.is_empty()
            || s.family.is_empty()
            || s.generation == 0
            || !["auto", "http", "https", "ssh", "smtp"].contains(&s.mode.as_str())
            || s.total == 0
            || s.total != s.completed
            || s.total != s.rows.len()
            || s.requested_ports.len() != s.total
        {
            return None;
        }
        let mut ports = s.requested_ports.clone();
        ports.sort_unstable();
        if ports.windows(2).any(|w| w[0] == w[1])
            || ports
                .iter()
                .any(|port| s.rows.iter().filter(|r| r.port == *port).count() != 1)
        {
            return None;
        }
        Some(Self {
            target: s.target.clone(),
            source: s.source_address.clone(),
            interface: s.interface.clone(),
            netmask: s.source_netmask.clone(),
            family: s.family.clone(),
            generation: s.generation,
            ports,
            mode: s.mode.clone(),
            hostname: s.hostname.clone(),
        })
    }
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
    pub baseline_report: Option<u64>,
    #[serde(skip)]
    pub comparison_scope: Option<ComparisonScope>,
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
    pub fn for_ui(&self) -> Self {
        Self {
            reports: self
                .reports
                .iter()
                .map(|r| Report {
                    id: r.id,
                    service_operation: r.service_operation,
                    generation: r.generation,
                    target: r.target.clone(),
                    interface: r.interface.clone(),
                    created_at: r.created_at,
                    baseline_report: r.baseline_report,
                    comparison_scope: None,
                    findings: if Some(r.id) == self.selected_report {
                        r.findings.clone()
                    } else {
                        vec![]
                    },
                })
                .collect(),
            selected_report: self.selected_report,
            selected_finding: self.selected_finding.clone(),
        }
    }
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
                    evidence_source: if (category == "http_headers" && certainty == "observed")
                        || (matches!(category, "http_redirect" | "mime_nosniff")
                            && certainty != "not_performed")
                    {
                        "http_response_headers"
                    } else {
                        source_for(category)
                    }
                    .into(),
                    impact: impact_for(category, severity, certainty).into(),
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
                if let Some(h) = &row.http_headers {
                    add(
                        "http_headers",
                        "info",
                        "observed",
                        format!(
                            "CSP={}; X-Content-Type-Options={}; Referrer-Policy={}; HSTS={}",
                            presence(h.content_security_policy),
                            presence(h.x_content_type_options),
                            presence(h.referrer_policy),
                            presence(h.strict_transport_security)
                        ),
                        "review_http_headers",
                    );
                } else {
                    add(
                        "http_headers",
                        "info",
                        "not_performed",
                        "complete_final_http_headers_unavailable".into(),
                        "collect_http_evidence",
                    );
                }
            }
            if matches!(row.method.as_str(), "http" | "https") {
                if let Some(h) = &row.http_headers {
                    let certainty = if h.x_content_type_options_value == "ambiguous" {
                        "unverified"
                    } else {
                        "observed"
                    };
                    add(
                        "mime_nosniff",
                        "info",
                        certainty,
                        format!("value={}", h.x_content_type_options_value),
                        "review_mime_nosniff",
                    );
                } else {
                    add(
                        "mime_nosniff",
                        "info",
                        "not_performed",
                        "complete_final_http_headers_unavailable".into(),
                        "review_mime_nosniff",
                    );
                }
            }
            if row.method == "http" {
                if let Some(h) = &row.http_headers {
                    let certainty = if h.redirect == "unknown" {
                        "unverified"
                    } else {
                        "observed"
                    };
                    add(
                        "http_redirect",
                        "info",
                        certainty,
                        format!("status={};redirect={}", h.response_status, h.redirect),
                        "review_http_redirect",
                    );
                } else {
                    add(
                        "http_redirect",
                        "info",
                        "not_performed",
                        "complete_final_http_response_unavailable".into(),
                        "review_http_redirect",
                    );
                }
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
        let comparison_scope = ComparisonScope::from_job(s);
        let previous = comparison_scope.as_ref().and_then(|scope| {
            self.reports
                .iter()
                .find(|r| r.comparison_scope.as_ref() == Some(scope))
        });
        let baseline_report = previous.map(|r| r.id);
        if let Some(previous) = previous {
            for f in &mut findings {
                f.change = match previous.findings.iter().find(|p| p.id == f.id) {
                    Some(p) if p.certainty != "observed" || f.certainty != "observed" => {
                        "uncertain"
                    }
                    Some(p)
                        if p.evidence == f.evidence
                            && p.severity == f.severity
                            && p.impact == f.impact
                            && p.evidence_source == f.evidence_source =>
                    {
                        "unchanged"
                    }
                    Some(_) => "changed",
                    None if f.certainty == "observed" => "new",
                    None => "uncertain",
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
                baseline_report,
                comparison_scope,
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
    crate::inventory::cell(s)
}
fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[cfg(test)]
fn export(r: &Report, format: &str) -> Result<String, String> {
    export_locale(r, format, "en")
}
pub fn export_locale(r: &Report, format: &str, locale: &str) -> Result<String, String> {
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
        "evidence_source",
        "impact",
        "baseline_report",
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
                f.evidence_source.clone(),
                f.impact.clone(),
                r.baseline_report
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();
    if format == "csv" {
        return Ok(std::iter::once(
            headers
                .iter()
                .map(|s| csv_cell(s))
                .collect::<Vec<_>>()
                .join(","),
        )
        .chain(rows.iter().map(|row| {
            row.iter()
                .map(|s| csv_cell(s))
                .collect::<Vec<_>>()
                .join(",")
        }))
        .collect::<Vec<_>>()
        .join("\r\n"));
    }
    if format != "html" {
        return Err("invalid_report_format".into());
    }
    let (lang,title,help,labels) = match locale {
        "ja" => ("ja","FlowGuard 観測レポート","既存の観測根拠を読み取り専用で評価。未検証・未実施は安全の証明ではありません。セッション内最大20件、ポリシーの自動変更なし。CSVは共通の機械可読コードを使用します。",["対象","インターフェース","レポート","サービス操作","ポート","分類","重要度","確かさ","観測根拠","推奨対応コード","確認時刻（Unix秒）","差分","対応状況","理由","根拠の取得元コード","影響コード","比較元レポート"]),
        "zh-CN" => ("zh-CN","FlowGuard 观测报告","只读评估已有观测依据。未验证或未执行不代表安全。会话最多20条，不自动更改策略。CSV使用通用机器可读代码。",["目标","接口","报告","服务操作","端口","分类","严重程度","确定性","观测依据","建议代码","确认时间（Unix秒）","差异","处理状态","原因","证据来源代码","影响代码","基准报告"]),
        _ => ("en","FlowGuard observations","Read-only evidence assessment. Unverified and not-performed checks are not proof of safety. Maximum 20 session reports; no automatic policy changes. CSV uses stable machine-readable codes.",headers),
    };
    Ok(format!("<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>{title}</title><style>body{{font:14px system-ui;padding:16px}}td,th{{border:1px solid #888;padding:8px;overflow-wrap:anywhere}}table{{border-collapse:collapse}}</style><h1>{title}</h1><p>{help}</p><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></html>",labels.iter().map(|h|format!("<th>{}</th>",html(h))).collect::<String>(),rows.iter().map(|row|format!("<tr>{}</tr>",row.iter().map(|s|format!("<td>{}</td>",html(s))).collect::<String>())).collect::<String>()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_and_impact_keep_unperformed_and_unverified_distinct() {
        assert_eq!(source_for("http_headers"), "headers_not_collected");
        assert_eq!(
            impact_for("http_headers", "info", "not_performed"),
            "headers_unknown"
        );
        assert_eq!(
            impact_for("tls", "warning", "unverified"),
            "tls_unverified_impact"
        );
        assert_eq!(impact_for("tls", "info", "observed"), "tls_scope_limited");
        assert_eq!(
            impact_for("certificate_expiry", "info", "not_performed"),
            "expiry_unknown"
        );
        assert_eq!(
            impact_for("certificate_expiry", "warning", "observed"),
            "expiry_attention"
        );
        assert_eq!(
            impact_for("future", "warning", "observed"),
            "impact_unknown"
        );
    }
    #[test]
    fn observed_http_headers_are_reported_without_claiming_effectiveness() {
        let mut state = Snapshot::default();
        let mut job = crate::services::Snapshot {
            operation: 1,
            stage: "completed".into(),
            rows: vec![crate::services::Row {
                port: 80,
                candidate: "HTTP".into(),
                service: Some("HTTP".into()),
                status: "confirmed".into(),
                method: "http".into(),
                product: None,
                response: None,
                tls: None,
                tls_details: None,
                http_headers: Some(crate::services::HeaderPresence {
                    content_security_policy: true,
                    x_content_type_options: false,
                    x_content_type_options_value: "missing".into(),
                    referrer_policy: false,
                    strict_transport_security: false,
                    response_status: 200,
                    redirect: "none".into(),
                }),
                observed_at: 1,
            }],
            ..Default::default()
        };
        state.assess(&job, 10).unwrap();
        let f = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "http_headers")
            .unwrap();
        assert_eq!(f.certainty, "observed");
        assert_eq!(f.evidence_source, "http_response_headers");
        assert_eq!(f.impact, "headers_presence_only");
        let mime = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "mime_nosniff")
            .unwrap();
        assert_eq!(mime.certainty, "observed");
        assert_eq!(mime.evidence, "value=missing");
        assert_eq!(mime.impact, "mime_scope_limited");

        assert!(f.evidence.contains("CSP=present"));
        assert!(f.evidence.contains("HSTS=not_observed"));
        assert!(export(&state.reports[0], "csv")
            .unwrap()
            .contains("http_response_headers"));
        let redirect = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "http_redirect")
            .unwrap();
        assert_eq!(redirect.certainty, "observed");
        assert_eq!(redirect.evidence, "status=200;redirect=none");
        job.rows[0]
            .http_headers
            .as_mut()
            .unwrap()
            .x_content_type_options_value = "ambiguous".into();
        job.rows[0].http_headers.as_mut().unwrap().redirect = "unknown".into();
        job.operation = 2;
        state.assess(&job, 11).unwrap();
        let redirect = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "http_redirect")
            .unwrap();
        assert_eq!(redirect.certainty, "unverified");
        assert_eq!(redirect.impact, "redirect_unverified");
        let mime = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "mime_nosniff")
            .unwrap();
        assert_eq!(mime.certainty, "unverified");
        assert_eq!(mime.evidence, "value=ambiguous");

        job.rows[0].http_headers = None;
        job.operation = 3;
        state.assess(&job, 12).unwrap();
        let redirect = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "http_redirect")
            .unwrap();
        assert_eq!(redirect.certainty, "not_performed");
        assert_eq!(redirect.evidence_source, "headers_not_collected");
        let mime = state.reports[0]
            .findings
            .iter()
            .find(|f| f.category == "mime_nosniff")
            .unwrap();
        assert_eq!(mime.certainty, "not_performed");
    }
    #[test]
    fn comparison_requires_same_complete_scope_and_never_claims_unknown_unchanged() {
        let mut state = Snapshot::default();
        let mut job = crate::services::Snapshot {
            operation: 1,
            stage: "completed".into(),
            target: "192.168.1.1".into(),
            source_address: "192.168.1.2".into(),
            source_netmask: "255.255.255.0".into(),
            interface: "en0".into(),
            family: "IPv4".into(),
            generation: 1,
            mode: "http".into(),
            requested_ports: vec![80],
            total: 1,
            completed: 1,
            rows: vec![crate::services::Row {
                port: 80,
                candidate: "http".into(),
                service: Some("HTTP".into()),
                status: "confirmed".into(),
                method: "http".into(),
                product: None,
                response: None,
                tls: None,
                tls_details: None,
                http_headers: None,
                observed_at: 1,
            }],
            ..Default::default()
        };
        state.assess(&job, 10).unwrap();
        job.operation = 2;
        state.assess(&job, 11).unwrap();
        let r = &state.reports[0];
        assert_eq!(r.baseline_report, Some(1));
        assert!(export(r, "csv").unwrap().contains("baseline_report"));
        assert!(export_locale(r, "html", "ja")
            .unwrap()
            .contains("比較元レポート"));
        assert_eq!(
            r.findings
                .iter()
                .find(|f| f.category == "service")
                .unwrap()
                .change,
            "unchanged"
        );
        assert_eq!(
            r.findings
                .iter()
                .find(|f| f.category == "http_headers")
                .unwrap()
                .change,
            "uncertain"
        );
        for mutate in [0, 1, 2, 3, 4, 5, 6, 7, 8] {
            let mut changed = job.clone();
            match mutate {
                0 => changed.mode = "https".into(),
                1 => changed.source_address = "192.168.1.3".into(),
                2 => changed.source_netmask = "255.255.0.0".into(),
                3 => changed.interface = "en1".into(),
                4 => changed.generation = 2,
                5 => changed.hostname = Some("example.com".into()),
                6 => changed.target = "192.168.1.4".into(),
                7 => changed.family = "IPv6".into(),
                _ => {
                    changed.requested_ports = vec![81];
                    changed.rows[0].port = 81;
                }
            }
            let scope = ComparisonScope::from_job(&changed).unwrap();
            assert_ne!(Some(scope), state.reports[0].comparison_scope);
        }
        let mut different = job.clone();
        different.mode = "https".into();
        state.assess(&different, 12).unwrap();
        assert_eq!(state.reports[0].baseline_report, None);
        assert!(state.reports[0]
            .findings
            .iter()
            .all(|f| f.change == "uncompared"));
        job.rows[0].status = "timeout".into();
        job.rows[0].service = None;
        state.assess(&job, 13).unwrap();
        assert_eq!(
            state.reports[0]
                .findings
                .iter()
                .find(|f| f.category == "service")
                .unwrap()
                .change,
            "uncertain"
        );
        job.completed = 0;
        state.assess(&job, 14).unwrap();
        assert_eq!(state.reports[0].baseline_report, None);
        assert!(state.reports[0]
            .findings
            .iter()
            .all(|f| f.change == "uncompared"));
    }
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
            http_headers: None,
            observed_at: 1,
        });
        for _ in 0..25 {
            s.assess(&job, 2).unwrap();
        }
        assert_eq!(s.reports.len(), 20);
        let r = &s.reports[0];
        assert!(r.findings.iter().all(|f| f.certainty != "safe"));
        assert!(r.findings.iter().any(|f| f.certainty == "not_performed"));
        assert!(r.findings.iter().all(|f| f.change == "uncompared"));
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
            baseline_report: None,
            comparison_scope: None,
            findings: vec![Finding {
                id: "f".into(),
                port: 1,
                category: "service".into(),
                severity: "info".into(),
                certainty: "unverified".into(),
                evidence_source: "service_observation".into(),
                impact: "service_unverified".into(),
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
        let csv = export(&s.reports[0], "csv").unwrap();
        assert!(csv.contains("'=SUM"));
        assert!(csv.contains("evidence_source"));
        assert!(csv.contains("service_observation"));
        assert!(csv.contains("service_unverified"));
        assert_eq!(csv_cell("  =SUM(1)"), "\"'  =SUM(1)\"");
        for (locale, title) in [
            ("ja", "観測レポート"),
            ("en", "observations"),
            ("zh-CN", "观测报告"),
        ] {
            let report = export_locale(&s.reports[0], "html", locale).unwrap();
            assert!(report.contains(title));
            assert!(report.contains("&lt;script&gt;"));
        }
        assert!(s.select(2, None).is_err());
    }
}
