use crate::{model::now, ports};
use serde::Serialize;
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub panel: crate::port_layout::Panel,
    pub notes: std::collections::BTreeMap<u16, crate::service_notes::Note>,
    pub comparison: crate::service_comparison::Comparison,
    pub requested_ports: Vec<u16>,
    pub source_netmask: String,
    pub operation: u64,
    pub port_operation: u64,
    pub generation: u64,
    pub target: String,
    pub source_address: String,
    pub interface: String,
    pub family: String,
    pub mode: String,
    pub hostname: Option<String>,
    pub stage: String,
    pub total: usize,
    pub completed: usize,
    pub rows: Vec<Row>,
    pub error: Option<String>,
    pub selected_port: Option<u16>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub port: u16,
    pub candidate: String,
    pub service: Option<String>,
    pub status: String,
    pub method: String,
    pub product: Option<String>,
    pub response: Option<String>,
    pub tls: Option<String>,
    pub tls_details: Option<crate::tls_details::Details>,
    pub observed_at: u64,
}
#[derive(Clone)]
pub struct Plan {
    pub tcp: ports::Plan,
    pub port_operation: u64,
    pub mode: String,
    pub hostname: Option<String>,
}
impl Plan {
    pub fn new(
        s: &crate::model::Snapshot,
        n: &crate::network::NetworkSnapshot,
        operation: u64,
        mode: &str,
    ) -> Result<Self, String> {
        let job = &s.ports;
        if !["auto", "http", "https", "ssh", "smtp"].contains(&mode) {
            return Err("service_mode".into());
        }
        if job.operation != operation
            || job.stage != "completed"
            || job.error.is_some()
            || job.generation != s.inventory.generation
        {
            return Err("stale_ports".into());
        }
        let device = s
            .discovery
            .devices
            .iter()
            .find(|d| {
                d.interface == job.interface
                    && d.address
                        .split('%')
                        .next()
                        .and_then(|a| a.parse::<std::net::IpAddr>().ok())
                        == job
                            .target
                            .split('%')
                            .next()
                            .and_then(|a| a.parse::<std::net::IpAddr>().ok())
            })
            .ok_or("stale_target")?;
        let open: Vec<_> = job
            .rows
            .iter()
            .filter(|r| r.status == "open")
            .map(|r| r.port)
            .collect();
        if open.is_empty() || open.len() > 64 {
            return Err("service_limit".into());
        }
        let mut tcp = ports::Plan::new(s, n, &device.id, job.generation, open[0], open[0])?;
        if tcp.snapshot().source_address != job.source_address || tcp.netmask != job.source_netmask
        {
            return Err("network_changed".into());
        }
        tcp.ports = open;
        Ok(Self {
            tcp,
            port_operation: operation,
            mode: mode.into(),
            hostname: None,
        })
    }
    pub fn with_hostname(mut self, value: &str) -> Result<Self, String> {
        self.hostname = crate::tls_details::hostname(value)?;
        if self.hostname.is_some() && !["auto", "https"].contains(&self.mode.as_str()) {
            return Err("service_hostname_mode".into());
        }
        Ok(self)
    }
    pub fn snapshot(&self) -> Snapshot {
        let t = self.tcp.snapshot();
        Snapshot {
            requested_ports: self.tcp.ports.clone(),
            source_netmask: self.tcp.netmask.clone(),
            port_operation: self.port_operation,
            generation: t.generation,
            target: t.target,
            source_address: t.source_address,
            interface: t.interface,
            family: t.family,
            mode: self.mode.clone(),
            hostname: self.hostname.clone(),
            stage: "running".into(),
            total: self.tcp.ports.len(),
            ..Default::default()
        }
    }
}
fn candidate(p: u16) -> &'static str {
    match p {
        80 | 8080 | 8000 => "HTTP",
        443 | 8443 => "HTTPS",
        22 => "SSH",
        25 | 587 => "SMTP",
        465 => "SMTP/TLS",
        53 => "DNS",
        445 => "SMB",
        _ => "unknown",
    }
}
fn mode_for(p: u16, m: &str) -> &str {
    if m != "auto" {
        return m;
    }
    match p {
        80 | 8080 | 8000 => "http",
        443 | 8443 => "https",
        22 => "ssh",
        25 | 587 => "smtp",
        _ => "unsupported",
    }
}
fn clean(v: &str) -> String {
    v.chars().filter(|c| !c.is_control()).take(256).collect()
}
fn tls_config() -> Option<Arc<rustls::ClientConfig>> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return None;
    }
    Some(Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .ok()?
        .with_root_certificates(roots)
        .with_no_client_auth(),
    ))
}
// Reapply the remaining total budget on each read/write, including TLS handshakes.
fn deadline(s: &TcpStream, end: Instant) -> std::io::Result<()> {
    let d = end
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::TimedOut, "deadline"))?;
    s.set_read_timeout(Some(d))?;
    s.set_write_timeout(Some(d))
}
fn retry<T>(end: Instant, mut action: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    loop {
        if Instant::now() >= end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "deadline",
            ));
        }
        match action() {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5))
            }
            result => return result,
        }
    }
}
fn read_bounded(
    mut read: impl FnMut(&mut [u8]) -> std::io::Result<usize>,
    http: bool,
) -> std::io::Result<Vec<u8>> {
    let mut v = Vec::new();
    let mut b = [0; 512];
    while v.len() < 8192 {
        let cap = (8192 - v.len()).min(b.len());
        let n = read(&mut b[..cap])?;
        if n == 0 {
            break;
        }
        v.extend_from_slice(&b[..n]);
        if (http && v.windows(4).any(|w| w == b"\r\n\r\n")) || (!http && v.contains(&b'\n')) {
            return Ok(v);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "incomplete_response",
    ))
}
fn parse(r: &mut Row, b: &[u8], mode: &str) {
    let Ok(text) = std::str::from_utf8(b) else {
        r.status = "unrecognized".into();
        return;
    };
    let line = text.split(['\r', '\n']).next().unwrap_or("");
    if ["http", "https"].contains(&mode) {
        let mut w = line.split(' ');
        let version = w.next().unwrap_or("");
        let code = w.next().unwrap_or("");
        if ["HTTP/1.0", "HTTP/1.1"].contains(&version)
            && code.len() == 3
            && code.parse::<u16>().is_ok_and(|c| (100..=599).contains(&c))
            && text.contains("\r\n\r\n")
        {
            r.service = Some(if mode == "https" { "HTTPS" } else { "HTTP" }.into());
            r.status = "confirmed".into();
            r.response = Some(clean(line));
            r.product = text
                .split("\r\n")
                .skip(1)
                .filter_map(|h| h.split_once(':'))
                .find(|(n, _)| n.eq_ignore_ascii_case("server"))
                .map(|(_, v)| clean(v.trim()));
            return;
        }
    } else if mode == "ssh"
        && (line.starts_with("SSH-2.0-") || line.starts_with("SSH-1.99-"))
        && line
            .splitn(3, '-')
            .nth(2)
            .and_then(|v| v.split(' ').next())
            .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| (33..=126).contains(&b)))
    {
        r.service = Some("SSH".into());
        r.status = "confirmed".into();
        r.response = Some(clean(line));
        r.product = Some(clean(line.splitn(3, '-').nth(2).unwrap_or("")));
        return;
    } else if mode == "smtp"
        && (line.starts_with("220 ") || line.starts_with("220-"))
        && line
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.eq_ignore_ascii_case("smtp") || w.eq_ignore_ascii_case("esmtp"))
    {
        r.service = Some("SMTP".into());
        r.status = "confirmed".into();
        r.response = Some(clean(line));
        return;
    }
    r.status = "unrecognized".into();
}
fn io(e: std::io::Error) -> String {
    format!("io:{:?}", e.kind())
}
fn probe(p: &Plan, port: u16, config: Option<Arc<rustls::ClientConfig>>) -> Row {
    let mode = mode_for(port, &p.mode);
    let mut r = Row {
        port,
        candidate: candidate(port).into(),
        service: None,
        status: "unsupported".into(),
        method: mode.into(),
        product: None,
        response: None,
        tls: None,
        tls_details: None,
        observed_at: now(),
    };
    if mode == "unsupported" {
        return r;
    }
    let end = Instant::now() + Duration::from_millis(1500);
    let result = (|| -> Result<Vec<u8>, String> {
        let mut s =
            ports::connect(p.tcp.local, p.tcp.target, Some(&p.tcp.interface), port).map_err(io)?;
        let host = if mode == "https" && p.hostname.is_some() {
            p.hostname.clone().unwrap()
        } else if p.tcp.target.is_ipv6() {
            format!("[{}]", p.tcp.target)
        } else {
            p.tcp.target.to_string()
        };
        let request=format!("HEAD / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nUser-Agent: FlowGuard/0.1\r\n\r\n");
        if mode == "https" {
            r.tls_details = Some(crate::tls_details::Details {
                server_name: p
                    .hostname
                    .clone()
                    .unwrap_or_else(|| p.tcp.target.to_string()),
                verification: "not_completed".into(),
                reason: "not_completed".into(),
                certificate: None,
            });
            let config = config.ok_or_else(|| {
                r.tls_details.as_mut().unwrap().reason = "roots_unavailable".into();
                "tls_roots_unavailable"
            })?;
            let mut c = rustls::ClientConnection::new(
                config,
                match &p.hostname {
                    Some(name) => rustls::pki_types::ServerName::try_from(name.clone())
                        .map_err(|_| "tls_name")?,
                    None => rustls::pki_types::ServerName::IpAddress(p.tcp.target.into()),
                },
            )
            .map_err(|_| "tls_error")?;
            s.set_nonblocking(true).map_err(io)?;
            while c.is_handshaking() {
                retry(end, || c.complete_io(&mut s)).map_err(|e| {
                    let details = r.tls_details.as_mut().unwrap();
                    details.reason = crate::tls_details::reason(&e).into();
                    details.verification =
                        if matches!(details.reason.as_str(), "timeout" | "handshake_failed") {
                            "not_completed"
                        } else {
                            "failed"
                        }
                        .into();
                    details.certificate = c
                        .peer_certificates()
                        .and_then(|certs| certs.first())
                        .and_then(|cert| crate::tls_details::certificate(cert.as_ref()));
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        io(e)
                    } else {
                        "tls_error".into()
                    }
                })?;
            }
            let details = r.tls_details.as_mut().unwrap();
            details.verification = "verified".into();
            details.reason = "verified".into();
            details.certificate = c
                .peer_certificates()
                .and_then(|certs| certs.first())
                .and_then(|cert| crate::tls_details::certificate(cert.as_ref()));
            r.tls = c.protocol_version().map(|version| {
                match version {
                    rustls::ProtocolVersion::TLSv1_2 => "TLS 1.2",
                    rustls::ProtocolVersion::TLSv1_3 => "TLS 1.3",
                    _ => "TLS",
                }
                .to_string()
            });
            let mut secure = rustls::StreamOwned::new(c, s);
            deadline(&secure.sock, end).map_err(io)?;
            let mut remaining = request.as_bytes();
            while !remaining.is_empty() {
                let written = retry(end, || secure.write(remaining)).map_err(io)?;
                if written == 0 {
                    return Err("io:WriteZero".into());
                }
                remaining = &remaining[written..];
            }
            read_bounded(
                |b| {
                    deadline(&secure.sock, end)?;
                    retry(end, || secure.read(b))
                },
                true,
            )
            .map_err(io)
        } else {
            if mode == "http" {
                deadline(&s, end).map_err(io)?;
                s.write_all(request.as_bytes()).map_err(io)?;
            }
            read_bounded(
                |b| {
                    deadline(&s, end)?;
                    s.read(b)
                },
                mode == "http",
            )
            .map_err(io)
        }
    })();
    match result {
        Ok(b) => parse(&mut r, &b, mode),
        Err(e) => {
            r.status = if e.contains("TimedOut") || e.contains("WouldBlock") {
                "timeout"
            } else if e.starts_with("tls_") {
                "tls_error"
            } else if e.contains("PermissionDenied") {
                "denied"
            } else {
                "error"
            }
            .into()
        }
    };
    r.observed_at = now();
    r
}
pub fn run(p: Plan, mut publish: impl FnMut(Snapshot), cancelled: impl Fn() -> bool) {
    let config = if p.mode == "https" || p.mode == "auto" {
        tls_config()
    } else {
        None
    };
    let mut result = p.snapshot();
    for chunk in p.tcp.ports.chunks(2) {
        if cancelled() {
            result.stage = "cancelled".into();
            publish(result);
            return;
        }
        if !p.tcp.matches(&crate::network::observe()) {
            result.stage = "failed".into();
            result.error = Some("network_changed".into());
            publish(result);
            return;
        }
        let rows = std::thread::scope(|scope| {
            chunk
                .iter()
                .map(|port| {
                    let config = config.clone();
                    let p = &p;
                    scope.spawn(move || probe(p, *port, config))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().expect("service worker"))
                .collect::<Vec<_>>()
        });
        if cancelled() {
            result.stage = "cancelled".into();
            publish(result);
            return;
        }
        result.rows.extend(rows);
        result.completed = result.rows.len();
        publish(result.clone());
    }
    if p.tcp.matches(&crate::network::observe()) {
        result.stage = "completed".into();
    } else {
        result.stage = "failed".into();
        result.error = Some("network_changed".into());
    }
    publish(result);
}
pub fn csv(s: &Snapshot, generation: u64, ports: u64) -> String {
    let mut out=String::from("\u{feff}operation,port_operation,generation,stale,stage,error,mode,target,source,interface,family,port,candidate,service,status,method,product_reported,response,tls,observed_at_unix,requested_hostname,tls_server_name,tls_verification,tls_reason,certificate_issuer,certificate_not_before_unix,certificate_not_after_unix,requested_ports,source_netmask,comparison_status,baseline_operation,baseline_port_operation,change,changed_fields,previous_status,previous_method,previous_service,previous_product_reported,previous_response,previous_tls,previous_tls_details_json,previous_observed_at_unix,user_display_name,user_comment,user_note_updated_at_unix\r\n");
    for r in &s.rows {
        let mut cells = vec![
            s.operation.to_string(),
            s.port_operation.to_string(),
            s.generation.to_string(),
            (s.generation != generation || s.port_operation != ports).to_string(),
            s.stage.clone(),
            s.error.clone().unwrap_or_default(),
            s.mode.clone(),
            s.target.clone(),
            s.source_address.clone(),
            s.interface.clone(),
            s.family.clone(),
            r.port.to_string(),
            r.candidate.clone(),
            r.service.clone().unwrap_or_default(),
            r.status.clone(),
            r.method.clone(),
            r.product.clone().unwrap_or_default(),
            r.response.clone().unwrap_or_default(),
            r.tls.clone().unwrap_or_default(),
            r.observed_at.to_string(),
            s.hostname.clone().unwrap_or_default(),
            r.tls_details
                .as_ref()
                .map(|d| d.server_name.clone())
                .unwrap_or_default(),
            r.tls_details
                .as_ref()
                .map(|d| d.verification.clone())
                .unwrap_or_default(),
            r.tls_details
                .as_ref()
                .map(|d| d.reason.clone())
                .unwrap_or_default(),
            r.tls_details
                .as_ref()
                .and_then(|d| d.certificate.as_ref())
                .and_then(|c| c.issuer.clone())
                .unwrap_or_default(),
            r.tls_details
                .as_ref()
                .and_then(|d| d.certificate.as_ref())
                .and_then(|c| c.not_before)
                .map(|t| t.to_string())
                .unwrap_or_default(),
            r.tls_details
                .as_ref()
                .and_then(|d| d.certificate.as_ref())
                .and_then(|c| c.not_after)
                .map(|t| t.to_string())
                .unwrap_or_default(),
        ];
        let change = s.comparison.rows.iter().find(|c| c.port == r.port);
        let previous = change.and_then(|c| c.previous.as_ref());
        cells.extend([
            s.requested_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            s.source_netmask.clone(),
            s.comparison.status.clone(),
            s.comparison
                .baseline_operation
                .map(|n| n.to_string())
                .unwrap_or_default(),
            s.comparison
                .baseline_port_operation
                .map(|n| n.to_string())
                .unwrap_or_default(),
            change
                .map(|c| c.kind.clone())
                .unwrap_or_else(|| "uncompared".into()),
            change.map(|c| c.fields.join(" ")).unwrap_or_default(),
            previous.map(|r| r.status.clone()).unwrap_or_default(),
            previous.map(|r| r.method.clone()).unwrap_or_default(),
            previous.and_then(|r| r.service.clone()).unwrap_or_default(),
            previous.and_then(|r| r.product.clone()).unwrap_or_default(),
            previous
                .and_then(|r| r.response.clone())
                .unwrap_or_default(),
            previous.and_then(|r| r.tls.clone()).unwrap_or_default(),
            previous
                .and_then(|r| r.tls_details.as_ref())
                .map(|d| serde_json::to_string(d).unwrap_or_default())
                .unwrap_or_default(),
            previous
                .map(|r| r.observed_at.to_string())
                .unwrap_or_default(),
        ]);
        let note = s.notes.get(&r.port);
        cells.extend([
            note.map(|n| n.label.clone()).unwrap_or_default(),
            note.map(|n| n.comment.clone()).unwrap_or_default(),
            note.map(|n| n.updated_at.to_string()).unwrap_or_default(),
        ]);
        out.push_str(
            &cells
                .iter()
                .map(|v| crate::inventory::cell(v))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_notes_are_exported_separately_with_csv_escaping() {
        let mut snapshot = Snapshot {
            rows: vec![row()],
            ..Default::default()
        };
        crate::service_notes::edit(
            &mut snapshot.notes,
            crate::service_notes::Edit {
                port: 80,
                expected_revision: 0,
                label: "=SUM(1)".into(),
                comment: "manual,\"note\"".into(),
            },
            123,
        )
        .unwrap();
        let exported = csv(&snapshot, 0, 0);
        assert_eq!(exported.lines().next().unwrap().split(',').count(), 45);
        assert!(exported.contains("user_display_name,user_comment,user_note_updated_at_unix"));
        assert!(exported.contains("'=SUM(1)"));
        assert!(exported.contains("\"manual,\"\"note\"\"\""));
        assert!(exported.ends_with("\"123\"\r\n"));
    }
    fn row() -> Row {
        Row {
            port: 80,
            candidate: "HTTP".into(),
            service: None,
            status: "unsupported".into(),
            method: "http".into(),
            product: None,
            response: None,
            tls: None,
            tls_details: None,
            observed_at: 0,
        }
    }
    #[test]
    fn evidence_not_numbers() {
        let mut r = row();
        parse(
            &mut r,
            b"HTTP/1.1 200 OK\r\nServer: Example/1\r\n\r\n",
            "http",
        );
        assert_eq!(r.service.as_deref(), Some("HTTP"));
        assert_eq!(r.product.as_deref(), Some("Example/1"));
        let mut r = row();
        parse(&mut r, b"220 FTP ready\r\n", "smtp");
        assert_eq!(r.status, "unrecognized");
        parse(&mut r, b"220 mail ESMTP ready\r\n", "smtp");
        assert_eq!(r.service.as_deref(), Some("SMTP"));
        let mut r = row();
        parse(&mut r, b"SSH-2.0-TestSSH_1\r\n", "ssh");
        assert_eq!(r.service.as_deref(), Some("SSH"));
    }
    #[test]
    fn bounded_responses() {
        assert!(read_bounded(
            |b| {
                b.fill(b'a');
                Ok(b.len())
            },
            true
        )
        .is_err());
        let mut r = row();
        parse(&mut r, b"HTTP/1.1 999 bad\r\n\r\n", "http");
        assert_eq!(r.status, "unrecognized");
        assert_eq!(clean("A\u{1b}B\n"), "AB");
        let s = Snapshot {
            rows: vec![Row {
                product: Some("=SUM(1)".into()),
                ..row()
            }],
            ..Default::default()
        };
        assert!(csv(&s, 1, 2).contains("'=SUM(1)"));
        assert_eq!(mode_for(1234, "auto"), "unsupported");
    }
    #[test]
    #[ignore = "host loopback sockets only"]
    fn actual_http_and_greetings() {
        for (mode, response, expected) in [
            (
                "http",
                "HTTP/1.1 204 No Content\r\nServer: Test/1\r\n\r\n",
                "HTTP",
            ),
            ("ssh", "SSH-2.0-LocalTest\r\n", "SSH"),
            ("smtp", "220 localhost ESMTP ready\r\n", "SMTP"),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let m = mode.to_string();
            let server = std::thread::spawn(move || {
                let (mut s, _) = listener.accept().unwrap();
                s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                if m == "http" {
                    let mut b = [0; 512];
                    let n = s.read(&mut b).unwrap();
                    assert!(b[..n].starts_with(b"HEAD / HTTP/1.1\r\n"));
                }
                s.write_all(response.as_bytes()).unwrap();
            });
            let p = Plan {
                tcp: ports::Plan {
                    settings: Default::default(),
                    local: "127.0.0.1".parse().unwrap(),
                    target: "127.0.0.1".parse().unwrap(),
                    interface: "lo0".into(),
                    netmask: "255.0.0.0".into(),
                    generation: 1,
                    ports: vec![port],
                    selection: "test".into(),
                },
                port_operation: 1,
                mode: mode.into(),
                hostname: None,
            };
            assert_eq!(probe(&p, port, None).service.as_deref(), Some(expected));
            server.join().unwrap();
        }
    }
    #[test]
    #[ignore = "host IPv4/IPv6 loopback TLS sockets only"]
    fn verified_https_and_untrusted_certificate() {
        // Ephemeral loopback credentials exist only in this test's temporary directory.
        let fixture = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            let output = std::process::Command::new("openssl")
                .args(args)
                .current_dir(fixture.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "test certificate generation failed"
            );
        };
        for (name, san) in [
            ("local", "IP:127.0.0.1,IP:::1"),
            ("wrong", "IP:127.0.0.2"),
            ("dns", "DNS:service.example.test"),
        ] {
            if name == "local" {
                run(&[
                    "genpkey",
                    "-algorithm",
                    "RSA",
                    "-pkeyopt",
                    "rsa_keygen_bits:2048",
                    "-out",
                    "key.pem",
                ]);
            }
            run(&[
                "req",
                "-x509",
                "-new",
                "-key",
                "key.pem",
                "-out",
                &format!("{name}.pem"),
                "-days",
                "2",
                "-subj",
                "/CN=FlowGuard loopback test",
                "-addext",
                &format!("subjectAltName={san}"),
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-addext",
                "extendedKeyUsage=serverAuth",
            ]);
            run(&[
                "x509",
                "-in",
                &format!("{name}.pem"),
                "-outform",
                "DER",
                "-out",
                &format!("{name}.der"),
            ]);
        }
        run(&[
            "pkcs8", "-topk8", "-nocrypt", "-in", "key.pem", "-outform", "DER", "-out", "key.der",
        ]);
        let cert = rustls::pki_types::CertificateDer::from(
            std::fs::read(fixture.path().join("local.der")).unwrap(),
        );
        let system_roots = tls_config().expect("macOS trust roots available");
        for (address, variant, hostname, mismatch) in [
            ("127.0.0.1", "local", None, false),
            ("::1", "local", None, false),
            ("127.0.0.1", "wrong", None, true),
            ("127.0.0.1", "dns", Some("service.example.test"), false),
            ("::1", "dns", Some("service.example.test"), false),
            ("127.0.0.1", "dns", Some("other.example.test"), true),
            ("127.0.0.1", "dns", None, true),
        ] {
            let cert = if variant == "local" {
                cert.clone()
            } else {
                rustls::pki_types::CertificateDer::from(
                    std::fs::read(fixture.path().join(format!("{variant}.der"))).unwrap(),
                )
            };
            for (trusted, time_shift) in
                [(true, 0i64), (false, 0), (true, 3 * 86400), (true, -86400)]
            {
                let listener = std::net::TcpListener::bind((address, 0)).unwrap();
                let port = listener.local_addr().unwrap().port();
                let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert.clone()],
                    rustls::pki_types::PrivateKeyDer::Pkcs8(
                        rustls::pki_types::PrivatePkcs8KeyDer::from(
                            std::fs::read(fixture.path().join("key.der")).unwrap(),
                        ),
                    ),
                )
                .unwrap();
                let server = std::thread::spawn(move || {
                    let (s, _) = listener.accept().unwrap();
                    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    s.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                    let mut tls = rustls::StreamOwned::new(
                        rustls::ServerConnection::new(Arc::new(config)).unwrap(),
                        s,
                    );
                    let mut b = [0; 512];
                    if let Ok(n) = tls.read(&mut b) {
                        assert!(b[..n].starts_with(b"HEAD / HTTP/1.1\r\n"));
                        if let Some(hostname) = hostname {
                            assert_eq!(tls.conn.server_name(), Some(hostname));
                            assert!(std::str::from_utf8(&b[..n])
                                .unwrap()
                                .contains(&format!("Host: {hostname}:{port}\r\n")));
                        } else {
                            assert!(tls.conn.server_name().is_none());
                        }
                        let _ = tls.write_all(b"HTTP/1.1 200 OK\r\nServer: LocalTLS/1\r\n\r\n");
                    }
                });
                let mut roots = rustls::RootCertStore::empty();
                if trusted {
                    roots.add(cert.clone()).unwrap();
                }
                let mut client = rustls::ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_root_certificates(roots)
                .with_no_client_auth();
                #[derive(Debug)]
                struct TestTime(u64);
                impl rustls::time_provider::TimeProvider for TestTime {
                    fn current_time(&self) -> Option<rustls::pki_types::UnixTime> {
                        Some(rustls::pki_types::UnixTime::since_unix_epoch(
                            Duration::from_secs(self.0),
                        ))
                    }
                }
                if time_shift != 0 {
                    client.time_provider = Arc::new(TestTime((now() as i64 + time_shift) as u64));
                }
                let source = crate::network::observe()
                    .addresses
                    .into_iter()
                    .find(|a| {
                        a.interface == "lo0"
                            && a.address
                                == if address == "127.0.0.2" {
                                    "127.0.0.1"
                                } else {
                                    address
                                }
                    })
                    .unwrap();
                let plan = Plan {
                    tcp: ports::Plan {
                        settings: Default::default(),
                        target: address.parse().unwrap(),
                        local: if address == "127.0.0.2" {
                            "127.0.0.1"
                        } else {
                            address
                        }
                        .parse()
                        .unwrap(),
                        interface: "lo0".into(),
                        netmask: source.netmask,
                        generation: 1,
                        ports: vec![port],
                        selection: "test".into(),
                    },
                    port_operation: 1,
                    mode: "https".into(),
                    hostname: hostname.map(str::to_string),
                };
                let client = if trusted {
                    Arc::new(client)
                } else {
                    system_roots.clone()
                };
                let r = probe(&plan, port, Some(client));
                if trusted && !mismatch && time_shift == 0 {
                    assert_eq!(r.service.as_deref(), Some("HTTPS"));
                    assert!(r.tls.is_some());
                    assert_eq!(r.product.as_deref(), Some("LocalTLS/1"));
                    let details = r.tls_details.as_ref().unwrap();
                    assert_eq!(details.verification, "verified");
                    let metadata = details.certificate.as_ref().unwrap();
                    assert!(metadata
                        .issuer
                        .as_ref()
                        .unwrap()
                        .contains("FlowGuard loopback test"));
                    assert!(metadata.not_before.unwrap() <= now() as i64);
                    assert!(metadata.not_after.unwrap() > now() as i64);
                    let exported = csv(
                        &Snapshot {
                            hostname: plan.hostname.clone(),
                            rows: vec![r.clone()],
                            ..Default::default()
                        },
                        1,
                        1,
                    );
                    assert!(exported.contains("certificate_not_after_unix"));
                    assert!(exported.contains("FlowGuard loopback test"));
                } else {
                    assert_eq!(r.status, "tls_error");
                    assert!(r.service.is_none());
                    assert_eq!(
                        r.tls_details.as_ref().unwrap().reason,
                        if !trusted {
                            "unknown_issuer"
                        } else if time_shift > 0 {
                            "expired"
                        } else if time_shift < 0 {
                            "not_yet_valid"
                        } else {
                            "name_mismatch"
                        }
                    );
                }
                server.join().unwrap();
            }
        }
    }
    #[test]
    #[ignore = "host IPv6 loopback worker and timeout only"]
    fn worker_csv_and_total_deadline() {
        let listener = std::net::TcpListener::bind(("::1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut b = [0; 512];
            s.read(&mut b).unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nServer: WorkerFixture/1\r\n\r\n")
                .unwrap();
        });
        let mask = crate::network::observe()
            .addresses
            .into_iter()
            .find(|a| a.interface == "lo0" && a.address == "::1")
            .unwrap()
            .netmask;
        let p = Plan {
            tcp: ports::Plan {
                settings: Default::default(),
                target: "::1".parse().unwrap(),
                local: "::1".parse().unwrap(),
                interface: "lo0".into(),
                netmask: mask,
                generation: 1,
                ports: vec![port],
                selection: "test".into(),
            },
            port_operation: 7,
            mode: "http".into(),
            hostname: None,
        };
        let mut snapshots = vec![];
        run(p.clone(), |s| snapshots.push(s), || false);
        let mut result = snapshots.pop().unwrap();
        assert_eq!(result.stage, "completed");
        assert_eq!(result.rows[0].service.as_deref(), Some("HTTP"));
        result.operation = 3;
        if let Ok(path) = std::env::var("FLOWGUARD_SERVICES_CSV_TEST_OUTPUT") {
            std::fs::write(path, csv(&result, 1, 7)).unwrap();
        }
        server.join().unwrap();
        let listener = std::net::TcpListener::bind(("::1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (_s, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(1800));
        });
        let start = Instant::now();
        let row = probe(&p, port, None);
        assert_eq!(row.status, "timeout");
        assert!(start.elapsed() < Duration::from_secs(3));
        server.join().unwrap();
    }
}
