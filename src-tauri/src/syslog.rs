//! Explicit, session-only, unauthenticated syslog reception. Content is never executable evidence.
use crate::network::NetworkSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    io::{ErrorKind, Read},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const LINE: usize = 4096;
const RETAIN: usize = 256;
const BYTES: u64 = 8 * 1024 * 1024;
const FRAMES: u64 = 20_000;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub bind_address: Ipv4Addr,
    pub port: u16,
    pub source_address: Ipv4Addr,
    pub transport: String,
    pub duration_seconds: u64,
    pub acknowledge_unverified: bool,
}
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub bytes_read: u64,
    pub frames: u64,
    pub accepted: u64,
    pub rejected_source: u64,
    pub invalid: u64,
    pub oversized: u64,
    pub rate_dropped: u64,
    pub evicted: u64,
    pub incomplete: u64,
    pub connections: u64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: u64,
    pub received_at: u64,
    pub peer: String,
    pub priority: u8,
    pub text: String,
    pub sanitized: bool,
    pub trust: String,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub operation: u64,
    pub config: Option<Config>,
    pub stage: String,
    pub bound_endpoint: Option<String>,
    pub counts: Counts,
    pub rows: Vec<Row>,
    pub selected_id: Option<u64>,
    pub error: Option<String>,
}
#[derive(Clone)]
pub struct Ticket {
    pub operation: u64,
    config: Config,
    interface: String,
    netmask: String,
    stop: Arc<AtomicBool>,
}
fn local(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_loopback() || address.is_link_local()
}
impl Ticket {
    fn new(config: Config, network: &NetworkSnapshot) -> Result<Self, String> {
        if !config.acknowledge_unverified
            || !local(config.bind_address)
            || !local(config.source_address)
            || config.port < 1024
            || !["udp", "tcp_newline"].contains(&config.transport.as_str())
            || !(30..=900).contains(&config.duration_seconds)
            || config.bind_address.is_loopback() != config.source_address.is_loopback()
        {
            return Err("invalid_config".into());
        }
        if network.error.is_some() {
            return Err("address_unavailable".into());
        }
        let interface = network
            .addresses
            .iter()
            .find(|a| a.up && a.family == "IPv4" && a.address == config.bind_address.to_string())
            .ok_or("address_unavailable")?;
        Ok(Self {
            operation: 0,
            config,
            interface: interface.interface.clone(),
            netmask: interface.netmask.clone(),
            stop: Arc::new(AtomicBool::new(false)),
        })
    }
    fn matches(&self, network: &NetworkSnapshot) -> bool {
        network.error.is_none()
            && network.addresses.iter().any(|a| {
                a.up && a.family == "IPv4"
                    && a.address == self.config.bind_address.to_string()
                    && a.interface == self.interface
                    && a.netmask == self.netmask
            })
    }
}
#[derive(Default)]
pub struct Machine {
    pub snapshot: Snapshot,
    ticket: Option<Ticket>,
}
impl Machine {
    pub fn busy(&self) -> bool {
        ["starting", "listening", "stopping"].contains(&self.snapshot.stage.as_str())
    }
    pub fn begin(&mut self, config: Config, network: &NetworkSnapshot) -> Result<Ticket, String> {
        if self.busy() {
            return Err("busy".into());
        }
        let mut ticket = Ticket::new(config, network)?;
        ticket.operation = self.snapshot.operation + 1;
        self.snapshot = Snapshot {
            operation: ticket.operation,
            config: Some(ticket.config.clone()),
            stage: "starting".into(),
            ..Default::default()
        };
        self.ticket = Some(ticket.clone());
        Ok(ticket)
    }
    pub fn stop(&mut self, operation: u64) -> Result<(), String> {
        if self.snapshot.operation != operation || !self.busy() {
            return Err("stale".into());
        }
        if let Some(ticket) = &self.ticket {
            ticket.stop.store(true, Ordering::SeqCst);
        }
        self.snapshot.stage = "stopping".into();
        Ok(())
    }
    pub fn apply(&mut self, mut result: Snapshot) -> Result<(), String> {
        if result.operation != self.snapshot.operation || !self.busy() {
            return Err("stale".into());
        }
        if self
            .ticket
            .as_ref()
            .is_some_and(|t| t.stop.load(Ordering::SeqCst))
        {
            if ["starting", "listening"].contains(&result.stage.as_str()) {
                return Err("stale".into());
            }
            // Freeze published evidence at stop; do not incorporate late messages/counters.
            result = self.snapshot.clone();
            result.stage = "stopped".into();
            result.error = None;
        }
        result.selected_id = self
            .snapshot
            .selected_id
            .filter(|id| result.rows.iter().any(|r| r.id == *id));
        self.snapshot = result;
        if !self.busy() {
            self.ticket = None;
        }
        Ok(())
    }
    pub fn clear(&mut self, operation: u64) -> Result<(), String> {
        if self.busy() || self.snapshot.operation != operation {
            return Err("stale".into());
        }
        self.snapshot = Snapshot {
            operation: operation + 1,
            ..Default::default()
        };
        Ok(())
    }
    pub fn select(&mut self, operation: u64, id: u64) -> Result<(), String> {
        if self.snapshot.operation != operation || !self.snapshot.rows.iter().any(|r| r.id == id) {
            return Err("stale".into());
        }
        self.snapshot.selected_id = Some(id);
        Ok(())
    }
}
struct Collector {
    snapshot: Snapshot,
    window: Instant,
    window_frames: u64,
    window_bytes: u64,
}
impl Collector {
    fn new(ticket: &Ticket) -> Self {
        Self {
            snapshot: Snapshot {
                operation: ticket.operation,
                config: Some(ticket.config.clone()),
                stage: "listening".into(),
                ..Default::default()
            },
            window: Instant::now(),
            window_frames: 0,
            window_bytes: 0,
        }
    }
    fn bytes(&mut self, n: usize) {
        self.snapshot.counts.bytes_read = self.snapshot.counts.bytes_read.saturating_add(n as u64);
        if self.snapshot.counts.bytes_read >= BYTES {
            self.snapshot.stage = "limit".into();
        }
    }
    fn frame(&mut self, peer: SocketAddr, bytes: &[u8]) {
        if self.snapshot.stage != "listening" {
            return;
        }
        let counts = &mut self.snapshot.counts;
        counts.frames += 1;
        if counts.frames > FRAMES {
            self.snapshot.stage = "limit".into();
            return;
        }
        if peer.ip() != self.snapshot.config.as_ref().unwrap().source_address {
            counts.rejected_source += 1;
            return;
        }
        if bytes.len() > LINE {
            counts.oversized += 1;
            return;
        }
        if self.window.elapsed() >= Duration::from_secs(1) {
            self.window = Instant::now();
            self.window_frames = 0;
            self.window_bytes = 0;
        }
        self.window_frames += 1;
        self.window_bytes += bytes.len() as u64;
        if self.window_frames > 20 || self.window_bytes > 64 * 1024 {
            counts.rate_dropped += 1;
            return;
        }
        let Some((priority, text, sanitized)) = parse(bytes) else {
            counts.invalid += 1;
            return;
        };
        counts.accepted += 1;
        self.snapshot.rows.push(Row {
            id: counts.accepted,
            received_at: crate::model::now(),
            peer: peer.to_string(),
            priority,
            text,
            sanitized,
            trust: "unverified_source_and_content".into(),
        });
        if self.snapshot.rows.len() > RETAIN {
            self.snapshot.rows.remove(0);
            counts.evicted += 1;
        }
    }
}
fn parse(bytes: &[u8]) -> Option<(u8, String, bool)> {
    if bytes.is_empty() || bytes.len() > LINE || bytes.first() != Some(&b'<') {
        return None;
    }
    let end = bytes.iter().take(5).position(|b| *b == b'>')?;
    let digits = &bytes[1..end];
    if digits.is_empty()
        || digits.len() > 3
        || !digits.iter().all(u8::is_ascii_digit)
        || (digits.len() > 1 && digits[0] == b'0')
    {
        return None;
    }
    let priority = std::str::from_utf8(digits).ok()?.parse::<u8>().ok()?;
    if priority > 191 {
        return None;
    }
    let text = std::str::from_utf8(&bytes[end + 1..]).ok()?;
    if text.is_empty() {
        return None;
    }
    let safe: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .filter(|c| !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'))
        .collect();
    let sanitized = safe != text;
    Some((priority, safe, sanitized))
}
#[derive(Default)]
struct Framer {
    bytes: Vec<u8>,
}
impl Framer {
    fn push(&mut self, byte: u8) -> Result<Option<Vec<u8>>, ()> {
        if byte == b'\n' {
            if self.bytes.last() == Some(&b'\r') {
                self.bytes.pop();
            }
            return Ok(Some(std::mem::take(&mut self.bytes)));
        }
        if self.bytes.len() >= LINE && !(self.bytes.len() == LINE && byte == b'\r') {
            return Err(());
        }
        self.bytes.push(byte);
        Ok(None)
    }
}
struct Client {
    socket: TcpStream,
    peer: SocketAddr,
    framer: Framer,
    active: Instant,
}
enum Receiver {
    Udp(UdpSocket),
    Tcp(TcpListener, Vec<Client>),
}
impl Receiver {
    fn bind(ticket: &Ticket) -> std::io::Result<Self> {
        let address = SocketAddrV4::new(ticket.config.bind_address, ticket.config.port);
        if ticket.config.transport == "udp" {
            let s = UdpSocket::bind(address)?;
            s.set_nonblocking(true)?;
            Ok(Self::Udp(s))
        } else {
            let socket = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::STREAM,
                Some(socket2::Protocol::TCP),
            )?;
            socket.set_reuse_address(true)?;
            socket.bind(&address.into())?;
            socket.listen(8)?;
            socket.set_nonblocking(true)?;
            Ok(Self::Tcp(socket.into(), vec![]))
        }
    }
    fn address(&self) -> std::io::Result<SocketAddr> {
        match self {
            Self::Udp(s) => s.local_addr(),
            Self::Tcp(s, _) => s.local_addr(),
        }
    }
    fn poll(&mut self, collector: &mut Collector) -> std::io::Result<()> {
        match self {
            Self::Udp(socket) => {
                let mut bytes = [0; LINE + 1];
                for _ in 0..64 {
                    match socket.recv_from(&mut bytes) {
                        Ok((n, peer)) => {
                            collector.bytes(n);
                            collector.frame(peer, &bytes[..n]);
                            if collector.snapshot.stage != "listening" {
                                break;
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                }
            }
            Self::Tcp(listener, clients) => {
                for _ in 0..4 {
                    match listener.accept() {
                        Ok((socket, peer)) => {
                            collector.snapshot.counts.connections += 1;
                            if collector.snapshot.counts.connections > 1000 {
                                collector.snapshot.stage = "limit".into();
                                break;
                            }
                            if peer.ip()
                                != collector.snapshot.config.as_ref().unwrap().source_address
                            {
                                collector.snapshot.counts.rejected_source += 1;
                                continue;
                            }
                            if clients.len() >= 2 {
                                collector.snapshot.counts.rate_dropped += 1;
                                continue;
                            }
                            socket.set_nonblocking(true)?;
                            clients.push(Client {
                                socket,
                                peer,
                                framer: Framer::default(),
                                active: Instant::now(),
                            });
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                }
                let mut index = 0;
                while index < clients.len() {
                    let client = &mut clients[index];
                    let mut bytes = [0; LINE];
                    let mut remove = client.active.elapsed() >= Duration::from_secs(2);
                    if !remove {
                        match client.socket.read(&mut bytes) {
                            Ok(0) => remove = true,
                            Ok(n) => {
                                client.active = Instant::now();
                                collector.bytes(n);
                                for &byte in &bytes[..n] {
                                    match client.framer.push(byte) {
                                        Ok(Some(line)) => collector.frame(client.peer, &line),
                                        Ok(None) => {}
                                        Err(()) => {
                                            collector.snapshot.counts.oversized += 1;
                                            remove = true;
                                            break;
                                        }
                                    }
                                    if collector.snapshot.stage != "listening" {
                                        break;
                                    }
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                            Err(_) => remove = true,
                        }
                    }
                    if remove {
                        if !client.framer.bytes.is_empty() {
                            collector.snapshot.counts.incomplete += 1;
                        }
                        clients.remove(index);
                    } else {
                        index += 1;
                    }
                }
            }
        }
        Ok(())
    }
}
pub fn run(ticket: Ticket, mut publish: impl FnMut(Snapshot)) {
    let mut result = Collector::new(&ticket);
    if ticket.stop.load(Ordering::SeqCst) {
        result.snapshot.stage = "stopped".into();
        publish(result.snapshot);
        return;
    }
    let receiver = Receiver::bind(&ticket);
    let mut receiver = match receiver {
        Ok(s) => s,
        Err(_) => {
            result.snapshot.stage = "failed".into();
            result.snapshot.error = Some("bind_failed".into());
            publish(result.snapshot);
            return;
        }
    };
    result.snapshot.bound_endpoint = receiver.address().ok().map(|a| a.to_string());
    let started = Instant::now();
    let mut published = Instant::now();
    let mut published_counts = Counts::default();
    let mut checked = Instant::now();
    if !ticket.stop.load(Ordering::SeqCst) {
        publish(result.snapshot.clone());
    }
    loop {
        if ticket.stop.load(Ordering::SeqCst) {
            result.snapshot.stage = "stopped".into();
            break;
        }
        if started.elapsed() >= Duration::from_secs(ticket.config.duration_seconds) {
            result.snapshot.stage = "expired".into();
            break;
        }
        if checked.elapsed() >= Duration::from_secs(2) {
            checked = Instant::now();
            if !ticket.matches(&crate::network::observe()) {
                result.snapshot.stage = "network_changed".into();
                break;
            }
        }
        if receiver.poll(&mut result).is_err() {
            result.snapshot.stage = "failed".into();
            result.snapshot.error = Some("receive_failed".into());
            break;
        }
        if result.snapshot.stage != "listening" {
            break;
        }
        if published.elapsed() >= Duration::from_secs(1)
            && result.snapshot.counts != published_counts
        {
            publish(result.snapshot.clone());
            published_counts = result.snapshot.counts.clone();
            published = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Close the listener and every client before announcing terminal state or accepting a new start.
    if let Receiver::Tcp(_, clients) = &receiver {
        result.snapshot.counts.incomplete += clients
            .iter()
            .filter(|c| !c.framer.bytes.is_empty())
            .count() as u64;
    }
    drop(receiver);
    publish(result.snapshot);
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn config() -> Config {
        Config {
            bind_address: Ipv4Addr::LOCALHOST,
            port: 5514,
            source_address: Ipv4Addr::LOCALHOST,
            transport: "udp".into(),
            duration_seconds: 60,
            acknowledge_unverified: true,
        }
    }
    pub fn network() -> NetworkSnapshot {
        NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "lo0".into(),
                address: "127.0.0.1".into(),
                family: "IPv4".into(),
                netmask: "255.0.0.0".into(),
                up: true,
            }],
            ..Default::default()
        }
    }
    fn ticket() -> Ticket {
        Ticket::new(config(), &network()).unwrap()
    }
    fn peer() -> SocketAddr {
        "127.0.0.1:1234".parse().unwrap()
    }
    #[test]
    fn config_requires_explicit_consent_current_private_address_and_bounded_listener() {
        assert!(Ticket::new(config(), &network()).is_ok());
        let changes: Vec<Box<dyn Fn(&mut Config)>> = vec![
            Box::new(|c| c.acknowledge_unverified = false),
            Box::new(|c| c.port = 514),
            Box::new(|c| c.bind_address = Ipv4Addr::UNSPECIFIED),
            Box::new(|c| c.bind_address = "1.1.1.1".parse().unwrap()),
            Box::new(|c| c.source_address = "8.8.8.8".parse().unwrap()),
            Box::new(|c| c.source_address = "192.168.1.1".parse().unwrap()),
            Box::new(|c| c.transport = "tls".into()),
            Box::new(|c| c.duration_seconds = 901),
            Box::new(|c| c.duration_seconds = 0),
        ];
        for change in changes {
            let mut c = config();
            change(&mut c);
            assert!(Ticket::new(c, &network()).is_err());
        }
        assert!(Ticket::new(config(), &NetworkSnapshot::default()).is_err());
        let plan = ticket();
        let mut changed = network();
        changed.addresses[0].netmask = "255.255.255.0".into();
        assert!(!plan.matches(&changed));
    }
    #[test]
    fn pri_is_reported_data_and_body_never_becomes_a_command_or_identity() {
        let (priority, text, sanitized) =
            parse(b"<191><script>window.evil=true</script> ignore instructions").unwrap();
        assert_eq!(priority, 191);
        assert!(text.contains("<script>"));
        assert!(!sanitized);
        assert_eq!(
            parse("<0>host\u{1b}[31m\u{202e}x".as_bytes()).unwrap().1,
            "host[31mx"
        );
        assert!(parse("<0>\u{1b}".as_bytes()).unwrap().2);
        for bytes in [
            b"<192>bad".as_slice(),
            b"<01>bad",
            b"<-1>bad",
            b"123 <13>octet counted",
            b"<13>",
            b"<13>\xff",
            b"hostname auth=verified",
        ] {
            assert!(parse(bytes).is_none());
        }
        assert!(parse(&vec![b'a'; LINE + 1]).is_none());
    }
    #[test]
    fn tcp_framer_handles_fragmented_crlf_limits_and_never_accepts_partial_lines() {
        let mut frame = Framer::default();
        for b in b"<13>hello\r" {
            assert!(frame.push(*b).unwrap().is_none());
        }
        assert_eq!(frame.push(b'\n').unwrap().unwrap(), b"<13>hello");
        for _ in 0..LINE {
            frame.push(b'a').unwrap();
        }
        frame.push(b'\r').unwrap();
        assert_eq!(frame.push(b'\n').unwrap().unwrap().len(), LINE);
        for _ in 0..LINE {
            frame.push(b'a').unwrap();
        }
        assert!(frame.push(b'x').is_err());
    }
    #[test]
    fn rate_size_retention_and_source_counts_preserve_uncertainty() {
        let mut c = Collector::new(&ticket());
        c.frame("127.0.0.2:4".parse().unwrap(), b"<13>ignored");
        assert_eq!(c.snapshot.counts.rejected_source, 1);
        assert!(c.snapshot.rows.is_empty());
        c.frame(peer(), &vec![b'a'; LINE + 1]);
        assert_eq!(c.snapshot.counts.oversized, 1);
        c.frame(peer(), b"not a log");
        assert_eq!(c.snapshot.counts.invalid, 1);
        for _ in 0..25 {
            c.frame(peer(), b"<13>record");
        }
        assert!(c.snapshot.counts.rate_dropped > 0);
        assert!(c.snapshot.counts.accepted <= 20);
        let mut c = Collector::new(&ticket());
        for _ in 0..260 {
            c.window -= Duration::from_secs(2);
            c.frame(peer(), b"<13>record");
        }
        assert_eq!(c.snapshot.rows.len(), RETAIN);
        assert_eq!(c.snapshot.counts.evicted, 4);
        assert_eq!(c.snapshot.rows[0].id, 5);
        assert_eq!(c.snapshot.rows[0].trust, "unverified_source_and_content");
        c.bytes(BYTES as usize);
        assert_eq!(c.snapshot.stage, "limit");
        let count = c.snapshot.counts.accepted;
        c.frame(peer(), b"<13>late");
        assert_eq!(c.snapshot.counts.accepted, count);
        let mut c = Collector::new(&ticket());
        c.snapshot.counts.frames = FRAMES;
        c.frame(peer(), b"<13>limit");
        assert_eq!(c.snapshot.stage, "limit");
    }
    #[test]
    fn stop_freezes_evidence_clear_and_restart_fence_old_operations() {
        let mut m = Machine::default();
        let plan = m.begin(config(), &network()).unwrap();
        assert!(m.begin(config(), &network()).is_err());
        let mut c = Collector::new(&plan);
        c.frame(peer(), b"<13>first");
        m.apply(c.snapshot.clone()).unwrap();
        m.select(plan.operation, 1).unwrap();
        m.stop(plan.operation).unwrap();
        let mut terminal = None;
        run(plan.clone(), |snapshot| terminal = Some(snapshot));
        assert_eq!(terminal.unwrap().stage, "stopped");
        c.frame(peer(), b"<13>late");
        assert!(m.apply(c.snapshot.clone()).is_err());
        c.snapshot.stage = "stopped".into();
        m.apply(c.snapshot.clone()).unwrap();
        assert_eq!(m.snapshot.rows.len(), 1);
        assert_eq!(m.snapshot.selected_id, Some(1));
        m.clear(plan.operation).unwrap();
        let next = m.begin(config(), &network()).unwrap();
        assert!(next.operation > plan.operation);
        assert!(m.apply(c.snapshot).is_err());
        assert!(m.clear(plan.operation).is_err());
        assert!(m.snapshot.rows.is_empty());
    }
    #[test]
    #[ignore = "host loopback UDP only; no LAN packets"]
    fn actual_udp_reception_source_filter_oversize_and_port_release() {
        let mut plan = ticket();
        plan.config.port = 0;
        plan.config.duration_seconds = 1;
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            run(plan, |s| {
                tx.send(s).unwrap();
            })
        });
        let initial = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(initial.stage, "listening");
        let address: SocketAddr = initial.bound_endpoint.as_ref().unwrap().parse().unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender
            .send_to(b"<13>1 - unverified-host app - - - loopback", address)
            .unwrap();
        sender.send_to(&vec![b'a'; 5000], address).unwrap();
        sender.send_to(b"bad", address).unwrap();
        let mut final_snapshot = initial;
        while let Ok(snapshot) = rx.recv_timeout(Duration::from_secs(3)) {
            let terminal = snapshot.stage != "listening";
            final_snapshot = snapshot;
            if terminal {
                break;
            }
        }
        worker.join().unwrap();
        assert_eq!(final_snapshot.stage, "expired");
        assert_eq!(final_snapshot.rows.len(), 1);
        assert_eq!(final_snapshot.counts.oversized, 1);
        assert_eq!(final_snapshot.counts.invalid, 1);
        assert!(final_snapshot.rows[0].peer.starts_with("127.0.0.1:"));
        assert!(UdpSocket::bind(address).is_ok());
        let mut plan = ticket();
        plan.config.port = 0;
        let mut receiver = Receiver::bind(&plan).unwrap();
        let address = receiver.address().unwrap();
        let mut c = Collector::new(&plan);
        c.snapshot.config.as_mut().unwrap().source_address = "127.0.0.2".parse().unwrap();
        sender.send_to(b"<13>wrong sender", address).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        receiver.poll(&mut c).unwrap();
        assert_eq!(c.snapshot.counts.rejected_source, 1);
        assert!(c.snapshot.rows.is_empty());
    }
    #[test]
    #[ignore = "host loopback TCP only; no LAN packets"]
    fn actual_tcp_fragmentation_incomplete_stop_and_exclusive_restart() {
        use std::io::Write;
        let mut plan = ticket();
        plan.config.transport = "tcp_newline".into();
        plan.config.port = 0;
        let stop = plan.stop.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            run(plan, |s| {
                let _ = tx.send(s);
            })
        });
        let initial = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let address: SocketAddr = initial.bound_endpoint.clone().unwrap().parse().unwrap();
        let mut same = ticket();
        same.config.transport = "tcp_newline".into();
        same.config.port = address.port();
        assert!(Receiver::bind(&same).is_err());
        let mut sender = TcpStream::connect(address).unwrap();
        sender.write_all(b"<13>fragment").unwrap();
        sender
            .write_all(b"ed\r\n<4>second\n<13>unfinished")
            .unwrap();
        sender.shutdown(std::net::Shutdown::Write).unwrap();
        let progress = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(progress.rows.len(), 2);
        assert_eq!(progress.rows[0].text, "fragmented");
        assert_eq!(progress.counts.incomplete, 1);
        stop.store(true, Ordering::SeqCst);
        let ended = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        assert_eq!(ended.stage, "stopped");
        assert!(Receiver::bind(&same).is_ok());
    }
}
