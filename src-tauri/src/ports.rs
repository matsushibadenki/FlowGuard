use serde::Serialize;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6},
    num::NonZeroU32,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub selected_port: Option<u16>,
    pub comparison: Comparison,
    pub operation: u64,
    pub source_address: String,
    pub family: String,
    pub source_netmask: String,
    pub requested_ports: Vec<u16>,
    pub selection: String,
    pub stage: String,
    pub target: String,
    pub interface: String,
    pub generation: u64,
    pub completed: usize,
    pub total: usize,
    pub rows: Vec<Row>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub port: u16,
    pub status: String,
    pub elapsed_ms: u64,
    pub observed_at: u64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub status: String,
    pub baseline_operation: Option<u64>,
    pub rows: Vec<Change>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub port: u16,
    pub kind: String,
    pub previous: Option<Row>,
}
#[derive(Clone, PartialEq, Eq)]
struct Scope {
    target: IpAddr,
    local: IpAddr,
    interface: String,
    mask: String,
    generation: u64,
    ports: Vec<u16>,
}
impl Scope {
    fn new(p: &Plan) -> Self {
        Self {
            target: p.target,
            local: p.local,
            interface: p.interface.clone(),
            mask: p.netmask.clone(),
            generation: p.generation,
            ports: p.ports.clone(),
        }
    }
}
#[derive(Default)]
pub struct Comparator {
    baseline: Option<(Scope, Snapshot)>,
    scope_changed: bool,
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
        Comparison {
            status: "running".into(),
            baseline_operation: self.baseline.as_ref().map(|(_, s)| s.operation),
            rows: vec![],
        }
    }
    pub fn invalidate(&mut self) {
        self.baseline = None;
    }
    pub fn update(&mut self, p: &Plan, s: &mut Snapshot) {
        if s.error.as_deref() == Some("network_changed") {
            self.invalidate();
            s.comparison = Comparison {
                status: "invalidated".into(),
                ..Default::default()
            };
            return;
        }
        let full = s.stage == "completed"
            && s.error.is_none()
            && s.completed == p.ports.len()
            && s.rows.len() == p.ports.len()
            && p.ports
                .iter()
                .all(|port| s.rows.iter().filter(|r| r.port == *port).count() == 1);
        let previous = self
            .baseline
            .as_ref()
            .filter(|(scope, _)| *scope == Scope::new(p))
            .map(|(_, s)| s);
        let known = |status: &str| matches!(status, "open" | "closed");
        let certain = full
            && s.rows.iter().all(|r| {
                known(&r.status)
                    && previous
                        .and_then(|old| old.rows.iter().find(|old| old.port == r.port))
                        .is_none_or(|old| known(&old.status))
            });
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
        } else if !certain {
            "uncertain"
        } else {
            "ready"
        };
        s.comparison = Comparison {
            status: status.into(),
            baseline_operation: previous.map(|s| s.operation),
            rows: s
                .rows
                .iter()
                .map(|r| {
                    let old = previous.and_then(|s| s.rows.iter().find(|old| old.port == r.port));
                    let kind = if !full {
                        "uncompared"
                    } else if let Some(old) = old {
                        if !known(&old.status) || !known(&r.status) {
                            "uncertain"
                        } else if old.status == r.status {
                            "unchanged"
                        } else if r.status == "open" {
                            "opened"
                        } else {
                            "closed"
                        }
                    } else {
                        "uncompared"
                    };
                    Change {
                        port: r.port,
                        kind: kind.into(),
                        previous: old.cloned(),
                    }
                })
                .collect(),
        };
        if full {
            let mut baseline = s.clone();
            baseline.comparison = Default::default();
            self.baseline = Some((Scope::new(p), baseline));
        }
    }
}
#[derive(Clone)]
pub struct Plan {
    pub target: IpAddr,
    pub local: IpAddr,
    pub interface: String,
    pub netmask: String,
    pub generation: u64,
    pub ports: Vec<u16>,
    pub selection: String,
}
impl Plan {
    pub fn new(
        s: &crate::model::Snapshot,
        n: &crate::network::NetworkSnapshot,
        id: &str,
        generation: u64,
        first: u16,
        last: u16,
    ) -> Result<Self, String> {
        if first == 0 || last < first || u32::from(last) - u32::from(first) >= 256 {
            return Err("port_range".into());
        }
        if generation != s.inventory.generation
            || !["completed", "partial"].contains(&s.discovery.stage.as_str())
        {
            return Err("stale_target".into());
        }
        let d = s
            .discovery
            .devices
            .iter()
            .find(|d| d.id == id)
            .ok_or("stale_target")?;
        let anchor =
            crate::discovery::Plan::new(n, &d.interface, &s.inventory.local_address, false)?;
        if anchor.local.netmask != s.inventory.netmask {
            return Err("network_changed".into());
        }
        let (target, local, netmask) = if d.family == "IPv4" {
            let base =
                crate::discovery::Plan::new(n, &d.interface, &s.inventory.local_address, true)?;
            let target = d
                .address
                .parse::<Ipv4Addr>()
                .map_err(|_| "invalid_target")?;
            let local = base
                .local
                .address
                .parse::<Ipv4Addr>()
                .map_err(|_| "invalid_source")?;
            if target != local && !base.targets.contains(&target) {
                return Err("outside_lan".into());
            }
            (IpAddr::V4(target), IpAddr::V4(local), base.local.netmask)
        } else if d.family == "IPv6" {
            let target = parse_v6(&d.address, &d.interface).ok_or("invalid_ipv6_scope")?;
            if !lan_v6(target) {
                return Err("outside_lan".into());
            }
            let mut candidates = n
                .addresses
                .iter()
                .filter(|a| a.up && a.interface == d.interface && a.family == "IPv6")
                .filter_map(|a| {
                    let ip = parse_v6(&a.address, &d.interface)?;
                    let prefix = a.netmask.parse::<u8>().ok()?;
                    (lan_v6(ip) && same_prefix(ip, target, prefix)).then_some((
                        ip,
                        prefix,
                        a.address == s.inventory.local_address,
                    ))
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(ip, _, anchor)| (!*anchor, *ip));
            let (local, prefix, _) = candidates.first().ok_or("ipv6_source_unavailable")?;
            (IpAddr::V6(target), IpAddr::V6(*local), prefix.to_string())
        } else {
            return Err("invalid_family".into());
        };
        Ok(Self {
            target,
            local,
            interface: d.interface.clone(),
            netmask,
            generation,
            ports: (first..=last).collect(),
            selection: "range".into(),
        })
    }
    pub fn with_preset(mut self, preset: &str) -> Result<Self, String> {
        if preset == "range" {
            return Ok(self);
        }
        let catalog: std::collections::BTreeMap<String, Vec<u16>> =
            serde_json::from_str(include_str!("../../src/core/port-presets.json"))
                .map_err(|_| "invalid_preset_catalog")?;
        self.ports = catalog.get(preset).cloned().ok_or("invalid_preset")?;
        if self.ports.is_empty() || self.ports.len() > 256 || self.ports.contains(&0) {
            return Err("invalid_preset_catalog".into());
        }
        self.ports.sort_unstable();
        self.ports.dedup();
        self.selection = preset.into();
        Ok(self)
    }
    pub fn matches(&self, n: &crate::network::NetworkSnapshot) -> bool {
        n.error.is_none()
            && n.addresses.iter().any(|a| {
                a.up && a.interface == self.interface
                    && parse_address(&a.address, &self.interface) == Some(self.local)
                    && a.netmask == self.netmask
            })
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            selected_port: None,
            source_address: display_address(self.local, &self.interface),
            family: if self.target.is_ipv4() {
                "IPv4"
            } else {
                "IPv6"
            }
            .into(),
            source_netmask: self.netmask.clone(),
            requested_ports: self.ports.clone(),
            selection: self.selection.clone(),
            stage: "running".into(),
            target: display_address(self.target, &self.interface),
            interface: self.interface.clone(),
            generation: self.generation,
            total: self.ports.len(),
            ..Default::default()
        }
    }
}
fn parse_v6(address: &str, interface: &str) -> Option<Ipv6Addr> {
    let (literal, zone) = address
        .split_once('%')
        .map_or((address, None), |(ip, zone)| (ip, Some(zone)));
    if zone.is_some_and(|zone| zone != interface) {
        return None;
    }
    literal.parse().ok()
}
fn parse_address(address: &str, interface: &str) -> Option<IpAddr> {
    address
        .parse::<Ipv4Addr>()
        .map(IpAddr::V4)
        .ok()
        .or_else(|| parse_v6(address, interface).map(IpAddr::V6))
}
fn lan_v6(ip: Ipv6Addr) -> bool {
    ip.is_unicast_link_local() || ip.is_unique_local() || (ip.segments()[0] & 0xe000) == 0x2000
}
fn same_prefix(a: Ipv6Addr, b: Ipv6Addr, prefix: u8) -> bool {
    if !(64..=128).contains(&prefix) {
        return false;
    }
    let mask = u128::MAX.checked_shl(u32::from(128 - prefix)).unwrap_or(0);
    (u128::from(a) & mask) == (u128::from(b) & mask)
}
fn display_address(ip: IpAddr, interface: &str) -> String {
    match ip {
        IpAddr::V6(ip) if ip.is_unicast_link_local() => format!("{ip}%{interface}"),
        _ => ip.to_string(),
    }
}
pub fn eligible(device: &crate::discovery::Device) -> bool {
    match device.family.as_str() {
        "IPv4" => device
            .address
            .parse::<Ipv4Addr>()
            .is_ok_and(|ip| ip.is_private()),
        "IPv6" => parse_v6(&device.address, &device.interface).is_some_and(lan_v6),
        _ => false,
    }
}
fn classify(e: &std::io::Error) -> &'static str {
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused => "closed",
        std::io::ErrorKind::PermissionDenied => "denied",
        std::io::ErrorKind::TimedOut => "timeout",
        std::io::ErrorKind::NetworkUnreachable | std::io::ErrorKind::HostUnreachable => {
            "unreachable"
        }
        _ => "error",
    }
}
pub(crate) fn connect(
    local: IpAddr,
    target: IpAddr,
    interface: Option<&str>,
    port: u16,
) -> std::io::Result<std::net::TcpStream> {
    let socket = socket2::Socket::new(
        if target.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        },
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    if local.is_ipv4() != target.is_ipv4() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "address family mismatch",
        ));
    }
    if target.is_ipv6() {
        socket.set_only_v6(true)?;
    }
    let mut scope = 0;
    if let Some(name) = interface {
        let name = std::ffi::CString::new(name)?;
        let index = NonZeroU32::new(unsafe { libc::if_nametoindex(name.as_ptr()) })
            .ok_or_else(std::io::Error::last_os_error)?;
        #[cfg(target_os = "macos")]
        {
            if target.is_ipv4() {
                socket.bind_device_by_index_v4(Some(index))?;
            } else {
                socket.bind_device_by_index_v6(Some(index))?;
            }
        }
        scope = index.get();
        #[cfg(not(target_os = "macos"))]
        {
            let _ = index;
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "macOS required",
            ));
        }
    }
    let address = |ip: IpAddr, port: u16| match ip {
        IpAddr::V4(ip) => SocketAddr::from((ip, port)),
        IpAddr::V6(ip) => SocketAddr::V6(SocketAddrV6::new(
            ip,
            port,
            0,
            if ip.is_unicast_link_local() { scope } else { 0 },
        )),
    };
    socket.bind(&address(local, 0).into())?;
    socket.connect_timeout(&address(target, port).into(), Duration::from_millis(600))?;
    Ok(socket.into())
}
fn probe(
    local: impl Into<IpAddr>,
    target: impl Into<IpAddr>,
    interface: Option<&str>,
    port: u16,
) -> Row {
    let (local, target) = (local.into(), target.into());
    let started = Instant::now();
    let result = connect(local, target, interface, port);
    Row {
        port,
        status: result
            .as_ref()
            .map(|_| "open")
            .unwrap_or_else(|e| classify(e))
            .into(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        observed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}
pub fn run(plan: Plan, mut publish: impl FnMut(Snapshot), cancelled: impl Fn() -> bool) {
    let mut result = plan.snapshot();
    for chunk in plan.ports.chunks(4) {
        if cancelled() {
            result.stage = "cancelled".into();
            publish(result);
            return;
        }
        if !plan.matches(&crate::network::observe()) {
            result.stage = "failed".into();
            result.error = Some("network_changed".into());
            publish(result);
            return;
        }
        let rows = std::thread::scope(|scope| {
            chunk
                .iter()
                .map(|port| {
                    scope.spawn(|| probe(plan.local, plan.target, Some(&plan.interface), *port))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().expect("TCP worker"))
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
    result.stage = if plan.matches(&crate::network::observe()) {
        "completed"
    } else {
        "failed"
    }
    .into();
    if result.stage == "failed" {
        result.error = Some("network_changed".into())
    }
    publish(result);
}
pub fn csv(job: &Snapshot, current_generation: u64) -> String {
    let headers = [
        "operation",
        "inventory_generation",
        "current_inventory_generation",
        "stale_collection",
        "stage",
        "error",
        "target",
        "source_address",
        "interface",
        "protocol",
        "selection",
        "requested_ports",
        "completed",
        "total",
        "port",
        "status",
        "elapsed_ms",
        "observed_at_unix",
        "address_family",
        "source_netmask",
        "comparison_status",
        "baseline_operation",
        "change",
        "previous_status",
        "previous_elapsed_ms",
        "previous_observed_at_unix",
    ];
    let mut text = String::from("\u{feff}");
    text.push_str(&headers.map(crate::inventory::cell).join(","));
    text.push_str("\r\n");
    for row in &job.rows {
        let change = job.comparison.rows.iter().find(|c| c.port == row.port);
        let previous = change.and_then(|c| c.previous.as_ref());
        let values = [
            job.operation.to_string(),
            job.generation.to_string(),
            current_generation.to_string(),
            (job.generation != current_generation).to_string(),
            job.stage.clone(),
            job.error.clone().unwrap_or_default(),
            job.target.clone(),
            job.source_address.clone(),
            job.interface.clone(),
            "TCP".into(),
            job.selection.clone(),
            job.requested_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(";"),
            job.completed.to_string(),
            job.total.to_string(),
            row.port.to_string(),
            row.status.clone(),
            row.elapsed_ms.to_string(),
            row.observed_at.to_string(),
            job.family.clone(),
            job.source_netmask.clone(),
            job.comparison.status.clone(),
            job.comparison
                .baseline_operation
                .map(|v| v.to_string())
                .unwrap_or_default(),
            change
                .map(|c| c.kind.clone())
                .unwrap_or_else(|| "uncompared".into()),
            previous.map(|r| r.status.clone()).unwrap_or_default(),
            previous
                .map(|r| r.elapsed_ms.to_string())
                .unwrap_or_default(),
            previous
                .map(|r| r.observed_at.to_string())
                .unwrap_or_default(),
        ];
        text.push_str(
            &values
                .iter()
                .map(|s| crate::inventory::cell(s))
                .collect::<Vec<_>>()
                .join(","),
        );
        text.push_str("\r\n");
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_and_csv_keep_the_exact_request_and_partial_outcomes() {
        let base = Plan {
            target: "192.168.1.30".parse().unwrap(),
            local: "192.168.1.20".parse().unwrap(),
            interface: "en0".into(),
            netmask: "255.255.255.0".into(),
            generation: 2,
            ports: vec![1, 2],
            selection: "range".into(),
        };
        let common = base.clone().with_preset("common").unwrap();
        assert!(common.ports.contains(&443));
        assert!(common.ports.contains(&3389));
        assert!(common.ports.len() < 256);
        assert_eq!(
            base.clone().with_preset("admin").unwrap().ports,
            vec![22, 3389, 5900]
        );
        assert!(base.clone().with_preset("unknown").is_err());
        assert_eq!(base.with_preset("range").unwrap().ports, vec![1, 2]);
        let mut job = common.snapshot();
        job.operation = 7;
        job.stage = "cancelled".into();
        job.completed = 3;
        job.rows = vec![
            Row {
                port: 22,
                status: "open".into(),
                elapsed_ms: 2,
                observed_at: 1,
            },
            Row {
                port: 53,
                status: "timeout".into(),
                elapsed_ms: 600,
                observed_at: 1,
            },
            Row {
                port: 80,
                status: "denied".into(),
                elapsed_ms: 0,
                observed_at: 1,
            },
        ];
        let output = csv(&job, 3);
        assert!(output.starts_with("\u{feff}\"operation\""));
        assert_eq!(output.lines().count(), 4);
        assert!(output.contains("\"cancelled\""));
        assert!(output.contains("\"timeout\""));
        assert!(output.contains("\"denied\""));
        assert!(output.contains("\"true\""));
        assert!(output.contains("22;53;80;443"));
        assert!(output.contains("\"192.168.1.20\""));
        if let Ok(path) = std::env::var("FLOWGUARD_TCP_CSV_TEST_OUTPUT") {
            std::fs::write(path, &output).unwrap()
        }
        job.interface = "=unexpected,\"value\"".into();
        let escaped = csv(&job, 2);
        assert!(escaped.contains("\"'=unexpected,\"\"value\"\"\""));
        assert_eq!(job.interface, "=unexpected,\"value\"");
    }
    #[test]
    fn comparison_uses_completed_baselines_and_never_turns_timeouts_into_closed() {
        let mut comparator = Comparator::default();
        let plan = Plan {
            target: "192.168.1.30".parse().unwrap(),
            local: "192.168.1.20".parse().unwrap(),
            interface: "en0".into(),
            netmask: "255.255.255.0".into(),
            generation: 1,
            ports: vec![22, 80, 443, 8080],
            selection: "test".into(),
        };
        let result = |op, statuses: &[&str]| {
            let mut s = plan.snapshot();
            s.operation = op;
            s.stage = "completed".into();
            s.completed = statuses.len();
            s.rows = plan
                .ports
                .iter()
                .zip(statuses)
                .map(|(port, status)| Row {
                    port: *port,
                    status: (*status).into(),
                    elapsed_ms: op,
                    observed_at: op,
                })
                .collect();
            s
        };
        comparator.begin(&plan);
        let mut first = result(1, &["closed", "open", "open", "closed"]);
        comparator.update(&plan, &mut first);
        assert_eq!(first.comparison.status, "first");
        comparator.begin(&plan);
        let mut uncertain = result(2, &["open", "closed", "timeout", "closed"]);
        comparator.update(&plan, &mut uncertain);
        assert_eq!(uncertain.comparison.status, "uncertain");
        assert_eq!(uncertain.comparison.baseline_operation, Some(1));
        assert_eq!(
            uncertain
                .comparison
                .rows
                .iter()
                .map(|r| r.kind.as_str())
                .collect::<Vec<_>>(),
            vec!["opened", "closed", "uncertain", "unchanged"]
        );
        comparator.begin(&plan);
        let mut interrupted = result(3, &["open"]);
        interrupted.stage = "cancelled".into();
        comparator.update(&plan, &mut interrupted);
        assert_eq!(interrupted.comparison.status, "incomplete");
        assert_eq!(interrupted.comparison.rows[0].kind, "uncompared");
        comparator.begin(&plan);
        let mut complete = result(4, &["open", "closed", "open", "closed"]);
        comparator.update(&plan, &mut complete);
        assert_eq!(complete.comparison.baseline_operation, Some(2));
        assert_eq!(complete.comparison.rows[0].kind, "unchanged");
        assert_eq!(complete.comparison.rows[2].kind, "uncertain");
        let output = csv(&complete, 1);
        assert!(output.contains("previous_status"));
        assert!(output.contains("\"uncertain\""));
        if let Ok(path) = std::env::var("FLOWGUARD_TCP_DIFF_CSV_TEST_OUTPUT") {
            std::fs::write(path, &output).unwrap()
        }
        let mut scope = plan.clone();
        scope.ports = vec![22];
        comparator.begin(&scope);
        let mut changed = scope.snapshot();
        changed.stage = "completed".into();
        changed.operation = 5;
        changed.completed = 1;
        changed.rows = vec![complete.rows[0].clone()];
        comparator.update(&scope, &mut changed);
        assert_eq!(changed.comparison.status, "scope_changed");
        assert!(changed.comparison.baseline_operation.is_none());
        scope.generation = 2;
        comparator.begin(&scope);
        changed.generation = 2;
        comparator.update(&scope, &mut changed);
        assert_eq!(changed.comparison.status, "scope_changed");
        changed.error = Some("network_changed".into());
        comparator.update(&scope, &mut changed);
        assert_eq!(changed.comparison.status, "invalidated");
        assert!(comparator.begin(&scope).baseline_operation.is_none());
    }
    #[test]
    fn errors_are_not_closed() {
        assert_eq!(classify(&std::io::ErrorKind::TimedOut.into()), "timeout");
        assert_eq!(
            classify(&std::io::ErrorKind::PermissionDenied.into()),
            "denied"
        );
        assert_eq!(
            classify(&std::io::ErrorKind::ConnectionRefused.into()),
            "closed"
        );
    }
    #[test]
    #[ignore = "host IPv6 loopback sockets only"]
    fn ipv6_loopback_worker_and_comparison_use_only_v6() {
        let listener = std::net::TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let n = crate::network::observe();
        let source = n
            .addresses
            .iter()
            .find(|a| a.interface == "lo0" && a.address == "::1")
            .unwrap();
        let plan = Plan {
            local: IpAddr::V6(Ipv6Addr::LOCALHOST),
            target: IpAddr::V6(Ipv6Addr::LOCALHOST),
            interface: "lo0".into(),
            netmask: source.netmask.clone(),
            generation: 1,
            ports: vec![port],
            selection: "ipv6_test".into(),
        };
        let mut snapshots = vec![];
        run(plan.clone(), |s| snapshots.push(s), || false);
        let mut first = snapshots.pop().unwrap();
        assert_eq!(first.stage, "completed");
        assert_eq!(first.rows[0].status, "open");
        assert_eq!(first.family, "IPv6");
        first.operation = 1;
        let mut comparator = Comparator::default();
        comparator.begin(&plan);
        comparator.update(&plan, &mut first);
        drop(listener);
        snapshots.clear();
        run(plan.clone(), |s| snapshots.push(s), || false);
        let mut second = snapshots.pop().unwrap();
        assert_eq!(second.rows[0].status, "closed");
        second.operation = 2;
        comparator.begin(&plan);
        comparator.update(&plan, &mut second);
        assert_eq!(second.comparison.rows[0].kind, "closed");
        let csv = csv(&second, 1);
        assert!(csv.contains("\"IPv6\""));
        assert!(csv.contains("\"::1\""));
        if let Ok(path) = std::env::var("FLOWGUARD_IPV6_CSV_TEST_OUTPUT") {
            std::fs::write(path, &csv).unwrap()
        }
    }
    #[test]
    #[ignore = "host loopback sockets"]
    fn loopback_open_and_refused() {
        let server = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = server.local_addr().unwrap().port();
        assert_eq!(
            probe(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, Some("lo0"), port).status,
            "open"
        );
        drop(server);
        assert_eq!(
            probe(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, Some("lo0"), port).status,
            "closed"
        );
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let open = listener.local_addr().unwrap().port();
        let closed_listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let closed = closed_listener.local_addr().unwrap().port();
        drop(closed_listener);
        let network = crate::network::observe();
        let local = network
            .addresses
            .iter()
            .find(|a| a.interface == "lo0" && a.address == "127.0.0.1")
            .unwrap();
        let plan = Plan {
            local: IpAddr::V4(Ipv4Addr::LOCALHOST),
            target: IpAddr::V4(Ipv4Addr::LOCALHOST),
            interface: "lo0".into(),
            netmask: local.netmask.clone(),
            generation: 9,
            ports: vec![open, closed],
            selection: "test_sparse".into(),
        };
        let mut results = Vec::new();
        run(plan, |snapshot| results.push(snapshot), || false);
        let final_result = results.last().unwrap();
        assert_eq!(final_result.stage, "completed");
        assert_eq!(final_result.requested_ports, vec![open, closed]);
        assert_eq!(final_result.completed, 2);
        assert_eq!(final_result.rows[0].status, "open");
        assert_eq!(final_result.rows[1].status, "closed");
        let mut comparator = Comparator::default();
        let base = Plan {
            local: IpAddr::V4(Ipv4Addr::LOCALHOST),
            target: IpAddr::V4(Ipv4Addr::LOCALHOST),
            interface: "lo0".into(),
            netmask: local.netmask.clone(),
            generation: 9,
            ports: vec![open, closed],
            selection: "test_sparse".into(),
        };
        comparator.begin(&base);
        let mut first = final_result.clone();
        first.operation = 1;
        comparator.update(&base, &mut first);
        drop(listener);
        let reopened = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, closed)).unwrap();
        comparator.begin(&base);
        let mut next = Vec::new();
        run(base.clone(), |s| next.push(s), || false);
        let mut second = next.pop().unwrap();
        second.operation = 2;
        comparator.update(&base, &mut second);
        assert_eq!(second.comparison.rows[0].kind, "closed");
        assert_eq!(second.comparison.rows[1].kind, "opened");
        drop(reopened);
    }
}
