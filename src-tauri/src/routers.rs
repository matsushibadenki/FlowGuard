//! Router collection boundary. Wire data never establishes transport authentication or write authority.
pub use crate::security::{Capability, ObservationSource, SecurityFlow, SecuritySnapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const MAX_SOURCES: usize = 8;
const MAX_ROWS: usize = 128;
const MAX_BYTES: usize = 1024 * 1024;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
const FRESH: u64 = 300;
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    InvalidConfig,
    Busy,
    Timeout,
    Cancelled,
    Authentication,
    SourceMismatch,
    InvalidPayload,
    Limit,
    Replay,
    Stale,
    Unavailable,
    Internal,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: Code,
    pub retryable: bool,
}
impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Self {
            code,
            retryable: matches!(code, Code::Busy | Code::Timeout | Code::Unavailable),
        }
    }
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    LanBeforeNat,
    WanAfterNat,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Authentication {
    Anonymous,
    PinnedSha256(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub endpoint: SocketAddr,
    pub location: Location,
    pub authentication: Authentication,
}
impl Source {
    fn valid(&self) -> bool {
        identifier(&self.id)
            && self.endpoint.port() != 0
            && !self.endpoint.ip().is_unspecified()
            && !self.endpoint.ip().is_multicast()
            && match &self.authentication {
                Authentication::Anonymous => true,
                Authentication::PinnedSha256(p) => {
                    p.len() == 64
                        && p.bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                }
            }
    }
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
// Not Deserialize: only a native transport adapter may attest the peer after verifying its handshake.
#[derive(Clone)]
pub struct Peer {
    endpoint: SocketAddr,
    verified_pin: Option<String>,
}
impl Peer {
    pub fn anonymous(endpoint: SocketAddr) -> Self {
        Self {
            endpoint,
            verified_pin: None,
        }
    }
    /// Caller MUST have authenticated this exact transport peer; not a claim from a payload.
    pub fn authenticated(endpoint: SocketAddr, verified_pin: String) -> Self {
        Self {
            endpoint,
            verified_pin: Some(verified_pin),
        }
    }
}
pub struct ReadOutput {
    pub payload: Vec<u8>,
    pub peer: Peer,
}
#[derive(Clone)]
pub struct Budget {
    end: Instant,
    stop: Arc<AtomicBool>,
}
impl Budget {
    pub fn check(&self) -> Result<()> {
        if self.stop.load(Ordering::SeqCst) {
            Err(Code::Cancelled.into())
        } else if Instant::now() >= self.end {
            Err(Code::Timeout.into())
        } else {
            Ok(())
        }
    }
    pub fn remaining(&self) -> Result<Duration> {
        self.check()?;
        Ok(self.end.saturating_duration_since(Instant::now()))
    }
}
/// Read-only implementors cannot receive a control command or a control authorization.
pub trait RouterAdapter: Send + 'static {
    fn collect(&mut self, source: &Source, budget: &Budget) -> Result<ReadOutput>;
}
// Separate surface; no collector or Tauri command mints this authority. Future review service only.
pub struct ControlAuthorization {
    source: Source,
    action: ControlAction,
    expires: Instant,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    SetFirewall(bool),
    Quarantine(std::net::IpAddr),
}
impl ControlAuthorization {
    pub fn check(
        &self,
        source: &Source,
        action: ControlAction,
        peer: &Peer,
        budget: &Budget,
    ) -> Result<()> {
        budget.check()?;
        if Instant::now() >= self.expires {
            return Err(Code::Authentication.into());
        }
        if &self.source != source || self.action != action {
            return Err(Code::SourceMismatch.into());
        }
        if !authenticate(source, peer)? {
            return Err(Code::Authentication.into());
        }
        Ok(())
    }
}
/// Implementors must check the opaque, exact-action authorization and authenticated peer before writing.
pub trait RouterControlAdapter: Send {
    fn execute(
        &mut self,
        source: &Source,
        action: ControlAction,
        authorization: &ControlAuthorization,
        budget: &Budget,
    ) -> Result<()>;
}
static WORKERS: AtomicUsize = AtomicUsize::new(0);
struct Worker;
impl Drop for Worker {
    fn drop(&mut self) {
        WORKERS.fetch_sub(1, Ordering::SeqCst);
    }
}
pub fn read(
    mut adapter: Box<dyn RouterAdapter>,
    source: Source,
    stop: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<ReadOutput> {
    if !source.valid() || timeout.is_zero() || timeout > Duration::from_secs(30) {
        return Err(Code::InvalidConfig.into());
    }
    let budget = Budget {
        end: Instant::now() + timeout,
        stop,
    };
    budget.check()?;
    if WORKERS
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            (n < 2).then_some(n + 1)
        })
        .is_err()
    {
        return Err(Code::Busy.into());
    }
    let permit = Worker;
    let worker_budget = budget.clone();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("flowguard-router-read".into())
        .spawn(move || {
            let _permit = permit;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                adapter.collect(&source, &worker_budget)
            }))
            .unwrap_or_else(|_| Err(Code::Internal.into()))
            .and_then(|output| {
                if output.payload.len() > MAX_BYTES {
                    Err(Code::Limit.into())
                } else {
                    Ok(output)
                }
            });
            let _ = tx.send(result);
        })
        .map_err(|_| Error::from(Code::Unavailable))?;
    loop {
        if let Err(error) = budget.check() {
            return Err(error);
        }
        match rx.recv_timeout(budget.remaining()?.min(Duration::from_millis(10))) {
            Ok(result) => {
                budget.check()?;
                return result;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Err(Code::Internal.into()),
        }
    }
}
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    id: String,
    local_endpoint: SocketAddr,
    remote_endpoint: SocketAddr,
    protocol: String,
    first_observed_at: u64,
    bytes_sent: Option<u64>,
    bytes_received: Option<u64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Batch {
    schema: u32,
    source_id: String,
    sequence: u64,
    captured_at: u64,
    records: Vec<Record>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: String,
    pub endpoint: SocketAddr,
    pub location: Location,
    pub stage: String,
    pub authentication: String,
    pub sampled_at: Option<u64>,
    pub error: Option<Error>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Correlation {
    pub mac_flow_id: String,
    pub router_flow_id: String,
    pub basis: String,
}
struct Entry {
    source: Source,
    operation: u64,
    stop: Arc<AtomicBool>,
    status: SourceStatus,
    sequence: Option<u64>,
    flows: Vec<SecurityFlow>,
}
#[derive(Clone)]
pub struct Ticket {
    source: Source,
    operation: u64,
    stop: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Store {
    entries: BTreeMap<String, Entry>,
    operation: u64,
}
impl Store {
    pub fn busy(&self) -> bool {
        self.entries
            .values()
            .any(|entry| entry.status.stage == "collecting")
    }
    pub fn register(&mut self, source: Source) -> Result<()> {
        if !source.valid() {
            return Err(Code::InvalidConfig.into());
        }
        if !self.entries.contains_key(&source.id) && self.entries.len() >= MAX_SOURCES {
            return Err(Code::Limit.into());
        }
        if let Some(old) = self.entries.remove(&source.id) {
            old.stop.store(true, Ordering::SeqCst);
        }
        self.operation += 1;
        let status = SourceStatus {
            id: source.id.clone(),
            endpoint: source.endpoint,
            location: source.location,
            stage: "idle".into(),
            authentication: "unknown".into(),
            sampled_at: None,
            error: None,
        };
        self.entries.insert(
            source.id.clone(),
            Entry {
                source,
                operation: self.operation,
                stop: Arc::new(AtomicBool::new(false)),
                status,
                sequence: None,
                flows: vec![],
            },
        );
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let entry = self.entries.remove(id).ok_or(Code::InvalidConfig)?;
        entry.stop.store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn begin(&mut self, id: &str) -> Result<Ticket> {
        let entry = self.entries.get_mut(id).ok_or(Code::InvalidConfig)?;
        if entry.status.stage == "collecting" {
            return Err(Code::Busy.into());
        }
        self.operation += 1;
        entry.operation = self.operation;
        entry.stop = Arc::new(AtomicBool::new(false));
        entry.status.stage = "collecting".into();
        entry.status.error = None;
        Ok(Ticket {
            source: entry.source.clone(),
            operation: entry.operation,
            stop: entry.stop.clone(),
        })
    }
    pub fn stop(&mut self, id: &str) -> Result<()> {
        let entry = self.entries.get_mut(id).ok_or(Code::InvalidConfig)?;
        entry.stop.store(true, Ordering::SeqCst);
        entry.status.stage = "cancelled".into();
        entry.status.error = Some(Code::Cancelled.into());
        Ok(())
    }
    pub fn finish(&mut self, ticket: &Ticket, result: Result<ReadOutput>, now: u64) -> Result<()> {
        let entry = self.entries.get_mut(&ticket.source.id).ok_or(Code::Stale)?;
        if entry.operation != ticket.operation
            || entry.stop.load(Ordering::SeqCst)
            || entry.status.stage != "collecting"
        {
            return Err(Code::Stale.into());
        }
        let validated = result.and_then(|output| validate(entry, output, now));
        match validated {
            Ok((batch, flows, authenticated)) => {
                entry.sequence = Some(batch.sequence);
                entry.status.sampled_at = Some(batch.captured_at);
                entry.status.authentication = if authenticated {
                    "verified"
                } else {
                    "unverified"
                }
                .into();
                entry.status.stage = "ready".into();
                entry.status.error = None;
                entry.flows = flows;
                Ok(())
            }
            Err(error) => {
                entry.status.stage = "failed".into();
                entry.status.error = Some(error.clone());
                entry.status.authentication = "unknown".into();
                Err(error)
            }
        }
    }
    pub fn augment(&self, mut snapshot: SecuritySnapshot, now: u64) -> SecuritySnapshot {
        for entry in self.entries.values() {
            let mut status = entry.status.clone();
            if status.stage == "ready"
                && status
                    .sampled_at
                    .is_none_or(|at| now.saturating_sub(at) > FRESH)
            {
                status.stage = "stale".into();
            }
            if status.stage == "ready" {
                snapshot.router_capabilities.flow_export = Capability::Available;
                if status.authentication == "verified" && status.location == Location::LanBeforeNat
                {
                    correlate(&snapshot.flows, &entry.flows, &mut snapshot.correlations);
                }
                snapshot.flows.extend(entry.flows.clone());
            }
            snapshot.router_sources.push(status);
        }
        snapshot
    }
}
fn authenticate(source: &Source, peer: &Peer) -> Result<bool> {
    if peer.endpoint != source.endpoint {
        return Err(Code::SourceMismatch.into());
    }
    match &source.authentication {
        Authentication::Anonymous => Ok(false),
        Authentication::PinnedSha256(pin) => {
            if peer.verified_pin.as_ref() == Some(pin) {
                Ok(true)
            } else {
                Err(Code::Authentication.into())
            }
        }
    }
}
fn validate(
    entry: &Entry,
    output: ReadOutput,
    now: u64,
) -> Result<(Batch, Vec<SecurityFlow>, bool)> {
    let authenticated = authenticate(&entry.source, &output.peer)?;
    if output.payload.len() > MAX_BYTES {
        return Err(Code::Limit.into());
    }
    let batch: Batch =
        serde_json::from_slice(&output.payload).map_err(|_| Error::from(Code::InvalidPayload))?;
    if batch.schema != 1 {
        return Err(Code::InvalidPayload.into());
    }
    if batch.source_id != entry.source.id {
        return Err(Code::SourceMismatch.into());
    }
    if batch.captured_at > MAX_SAFE
        || batch.sequence > MAX_SAFE
        || batch.captured_at > now.saturating_add(5)
        || now.saturating_sub(batch.captured_at) > FRESH
        || batch.captured_at == 0
    {
        return Err(Code::InvalidPayload.into());
    }
    if entry
        .sequence
        .is_some_and(|sequence| batch.sequence <= sequence)
        || entry
            .status
            .sampled_at
            .is_some_and(|at| batch.captured_at < at)
    {
        return Err(Code::Replay.into());
    }
    if batch.records.len() > MAX_ROWS {
        return Err(Code::Limit.into());
    }
    let mut seen: HashMap<&str, &Record> = HashMap::new();
    let mut flows = vec![];
    for record in &batch.records {
        if !identifier(&record.id)
            || !["TCP", "UDP"].contains(&record.protocol.as_str())
            || record.first_observed_at == 0
            || record.first_observed_at > batch.captured_at
            || record.first_observed_at > MAX_SAFE
            || record.bytes_sent.is_some_and(|v| v > MAX_SAFE)
            || record.bytes_received.is_some_and(|v| v > MAX_SAFE)
            || [record.local_endpoint, record.remote_endpoint]
                .iter()
                .any(|address| {
                    address.port() == 0
                        || address.ip().is_unspecified()
                        || address.ip().is_multicast()
                })
            || record.local_endpoint.is_ipv4() != record.remote_endpoint.is_ipv4()
        {
            return Err(Code::InvalidPayload.into());
        }
        if let Some(previous) = seen.insert(&record.id, record) {
            if previous != record {
                return Err(Code::InvalidPayload.into());
            }
            continue;
        }
        flows.push(SecurityFlow {
            id: format!("router:{}:{}", entry.source.id, record.id),
            source: ObservationSource::Router,
            source_id: entry.source.id.clone(),
            local_endpoint: record.local_endpoint.to_string(),
            remote_address: match record.remote_endpoint {
                SocketAddr::V6(v) if v.scope_id() != 0 => format!("{}%{}", v.ip(), v.scope_id()),
                address => address.ip().to_string(),
            },
            remote_port: record.remote_endpoint.port(),
            protocol: record.protocol.clone(),
            direction: "unknown".into(),
            app_key: None,
            pid: None,
            first_observed_at: record.first_observed_at,
            sampled_at: Some(batch.captured_at),
            bytes_sent: record.bytes_sent,
            bytes_received: record.bytes_received,
            counter_basis: if record.bytes_sent.is_some() || record.bytes_received.is_some() {
                "router_reported_endpoint_counters"
            } else {
                "unavailable"
            }
            .into(),
        });
    }
    Ok((batch, flows, authenticated))
}
fn tuple(flow: &SecurityFlow) -> Option<(SocketAddr, SocketAddr, &str)> {
    let local: SocketAddr = flow.local_endpoint.parse().ok()?;
    let remote = SocketAddr::new(flow.remote_address.parse().ok()?, flow.remote_port);
    if [local, remote]
        .iter()
        .any(|endpoint| endpoint.ip().to_canonical().is_loopback())
    {
        return None;
    }
    Some((local, remote, &flow.protocol))
}
fn correlate(mac: &[SecurityFlow], router: &[SecurityFlow], out: &mut Vec<Correlation>) {
    if out.len() >= 512 {
        return;
    }
    // Parse each endpoint once; never perform a full cross-product of all source rows.
    let mut index: HashMap<_, Vec<&SecurityFlow>> = HashMap::new();
    for flow in mac
        .iter()
        .filter(|flow| flow.source == ObservationSource::MacSocket)
    {
        if let Some(key) = tuple(flow) {
            index.entry(key).or_default().push(flow);
        }
    }
    for right in router {
        let Some(key) = tuple(right) else {
            continue;
        };
        let Some(candidates) = index.get(&key) else {
            continue;
        };
        for left in candidates {
            if out.len() >= 512 {
                return;
            }
            if let Some(at) = left.sampled_at {
                if right.sampled_at.is_some_and(|time| at.abs_diff(time) <= 2)
                    && right.first_observed_at <= at
                    && left.first_observed_at <= right.sampled_at.unwrap_or(0)
                {
                    out.push(Correlation {
                        mac_flow_id: left.id.clone(),
                        router_flow_id: right.id.clone(),
                        basis: "same_tuple_near_sample_candidate_only".into(),
                    });
                }
            }
        }
    }
}
impl Ticket {
    pub fn collect(
        &self,
        adapter: Box<dyn RouterAdapter>,
        timeout: Duration,
    ) -> Result<ReadOutput> {
        read(adapter, self.source.clone(), self.stop.clone(), timeout)
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub static READ_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    pub fn source() -> Source {
        Source {
            id: "home".into(),
            endpoint: "192.168.1.1:443".parse().unwrap(),
            location: Location::LanBeforeNat,
            authentication: Authentication::PinnedSha256("a".repeat(64)),
        }
    }
    pub fn output(sequence: u64, at: u64) -> ReadOutput {
        ReadOutput{peer:Peer::authenticated(source().endpoint,"a".repeat(64)),payload:serde_json::to_vec(&serde_json::json!({"schema":1,"sourceId":"home","sequence":sequence,"capturedAt":at,"records":[{"id":"flow1","localEndpoint":"192.168.1.20:50000","remoteEndpoint":"1.1.1.1:443","protocol":"TCP","firstObservedAt":at-1,"bytesSent":null,"bytesReceived":0}]})).unwrap()}
    }
    fn mutate(out: &mut ReadOutput, change: impl FnOnce(&mut serde_json::Value)) {
        let mut value = serde_json::from_slice(&out.payload).unwrap();
        change(&mut value);
        out.payload = serde_json::to_vec(&value).unwrap();
    }
    fn store() -> Store {
        let mut store = Store::default();
        store.register(source()).unwrap();
        store
    }
    fn mac(at: u64) -> SecuritySnapshot {
        let c = crate::model::Connection {
            identity: None,
            id: "mac1".into(),
            app: "local".into(),
            app_key: "path".into(),
            pid: 1,
            local: "192.168.1.20:50000".into(),
            destination: "1.1.1.1".into(),
            port: 443,
            protocol: "TCP".into(),
            direction: "unknown".into(),
            observed_at: at - 1,
        };
        crate::security::normalize(&[c], false, Some(at))
    }
    #[test]
    fn payload_identity_authentication_and_source_cannot_self_attest() {
        let changes: Vec<Box<dyn Fn(&mut ReadOutput)>> = vec![
            Box::new(|out| out.peer = Peer::anonymous(source().endpoint)),
            Box::new(|out| out.peer = Peer::authenticated(source().endpoint, "b".repeat(64))),
            Box::new(|out| {
                out.peer = Peer::authenticated("192.168.1.2:443".parse().unwrap(), "a".repeat(64))
            }),
            Box::new(|out| mutate(out, |v| v["sourceId"] = "other".into())),
            Box::new(|out| mutate(out, |v| v["authenticated"] = true.into())),
            Box::new(|out| mutate(out, |v| v["records"][0]["appKey"] = "trusted".into())),
        ];
        for change in changes {
            let mut store = store();
            let ticket = store.begin("home").unwrap();
            let mut out = output(1, 100);
            change(&mut out);
            assert!(store.finish(&ticket, Ok(out), 100).is_err());
            let result = store.augment(mac(100), 100);
            assert_eq!(result.flows.len(), 1);
            assert!(result.correlations.is_empty());
            assert_eq!(result.router_sources[0].stage, "failed");
            assert_eq!(
                result.router_capabilities.firewall_write,
                Capability::Unknown
            );
        }
    }
    #[test]
    fn batches_are_atomic_bounded_and_strictly_typed() {
        let mutations: Vec<Box<dyn Fn(&mut serde_json::Value)>> = vec![
            Box::new(|v| v["schema"] = 2.into()),
            Box::new(|v| v["records"][0]["protocol"] = "ICMP".into()),
            Box::new(|v| v["records"][0]["localEndpoint"] = "name.example:42".into()),
            Box::new(|v| v["records"][0]["remoteEndpoint"] = "1.1.1.1:0".into()),
            Box::new(|v| v["records"][0]["remoteEndpoint"] = "[::1]:443".into()),
            Box::new(|v| v["records"][0]["remoteEndpoint"] = "224.0.0.251:5353".into()),
            Box::new(|v| v["records"][0]["id"] = "../id\n".into()),
            Box::new(|v| v["records"][0]["firstObservedAt"] = 101.into()),
            Box::new(|v| v["records"][0]["bytesSent"] = (MAX_SAFE + 1).into()),
            Box::new(|v| v["capturedAt"] = 106.into()),
            Box::new(|v| v["capturedAt"] = 0.into()),
            Box::new(|v| v["sequence"] = (MAX_SAFE + 1).into()),
            Box::new(|v| v["records"] = vec![v["records"][0].clone(); 129].into()),
            Box::new(|v| {
                let mut second = v["records"][0].clone();
                second["bytesReceived"] = 7.into();
                v["records"].as_array_mut().unwrap().push(second);
            }),
        ];
        for mutation in mutations {
            let mut store = store();
            let ticket = store.begin("home").unwrap();
            let mut out = output(1, 100);
            mutate(&mut out, mutation);
            assert!(store.finish(&ticket, Ok(out), 100).is_err());
            assert!(store
                .augment(SecuritySnapshot::default(), 100)
                .flows
                .is_empty());
        }
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        let mut out = output(1, 100);
        out.payload = vec![0; MAX_BYTES + 1];
        assert_eq!(
            store.finish(&ticket, Ok(out), 100).unwrap_err().code,
            Code::Limit
        );
    }
    #[test]
    fn duplicates_and_correlations_keep_sources_counters_and_uncertainty() {
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        let mut out = output(1, 100);
        mutate(&mut out, |v| {
            let row = v["records"][0].clone();
            v["records"].as_array_mut().unwrap().push(row);
        });
        store.finish(&ticket, Ok(out), 100).unwrap();
        let result = store.augment(mac(100), 100);
        assert_eq!(result.flows.len(), 2);
        assert_eq!(result.correlations.len(), 1);
        assert_eq!(
            result.correlations[0].basis,
            "same_tuple_near_sample_candidate_only"
        );
        let flow = &result.flows[1];
        assert_eq!(flow.id, "router:home:flow1");
        assert_eq!(flow.bytes_sent, None);
        assert_eq!(flow.bytes_received, Some(0));
        assert_eq!(flow.app_key, None);
        assert_eq!(flow.pid, None);
        assert_eq!(flow.direction, "unknown");
        assert_eq!(result.flows[0].bytes_received, None);
        assert_eq!(
            result.router_capabilities.flow_export,
            Capability::Available
        );
        assert_eq!(
            result.router_capabilities.firewall_write,
            Capability::Unknown
        );
        assert!(store.augment(mac(103), 103).correlations.is_empty());
        let result = store.augment(mac(401), 401);
        assert_eq!(result.router_sources[0].stage, "stale");
        assert_eq!(result.flows.len(), 1);
        assert_eq!(result.router_capabilities.flow_export, Capability::Unknown);
    }
    #[test]
    fn identical_loopback_endpoints_on_different_hosts_are_not_correlated() {
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        let mut out = output(1, 100);
        mutate(&mut out, |v| {
            v["records"][0]["localEndpoint"] = "127.0.0.1:50000".into();
            v["records"][0]["remoteEndpoint"] = "127.0.0.1:443".into();
        });
        store.finish(&ticket, Ok(out), 100).unwrap();
        let mut local = mac(100);
        local.flows[0].local_endpoint = "127.0.0.1:50000".into();
        local.flows[0].remote_address = "127.0.0.1".into();
        let result = store.augment(local, 100);
        assert_eq!(result.flows.len(), 2);
        assert!(result.correlations.is_empty());
    }
    #[test]
    fn anonymous_wan_and_unknown_locations_never_create_correlations() {
        for (location, anonymous) in [
            (Location::LanBeforeNat, true),
            (Location::WanAfterNat, false),
            (Location::Unknown, false),
        ] {
            let mut config = source();
            config.location = location;
            if anonymous {
                config.authentication = Authentication::Anonymous;
            }
            let mut store = Store::default();
            store.register(config).unwrap();
            let ticket = store.begin("home").unwrap();
            let mut out = output(1, 100);
            if anonymous {
                out.peer = Peer::anonymous(source().endpoint);
            }
            store.finish(&ticket, Ok(out), 100).unwrap();
            let result = store.augment(mac(100), 100);
            assert_eq!(result.flows.len(), 2);
            assert!(result.correlations.is_empty());
            if anonymous {
                assert_eq!(result.router_sources[0].authentication, "unverified");
            }
        }
    }
    #[test]
    fn replay_cancel_reregister_and_remove_fence_late_results() {
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        store.finish(&ticket, Ok(output(4, 100)), 100).unwrap();
        let ticket = store.begin("home").unwrap();
        assert_eq!(
            store
                .finish(&ticket, Ok(output(4, 100)), 100)
                .unwrap_err()
                .code,
            Code::Replay
        );
        let ticket = store.begin("home").unwrap();
        store.stop("home").unwrap();
        assert_eq!(
            store
                .finish(&ticket, Ok(output(5, 101)), 101)
                .unwrap_err()
                .code,
            Code::Stale
        );
        let ticket = store.begin("home").unwrap();
        store.register(source()).unwrap();
        assert_eq!(
            store
                .finish(&ticket, Ok(output(6, 102)), 102)
                .unwrap_err()
                .code,
            Code::Stale
        );
        let ticket = store.begin("home").unwrap();
        store.remove("home").unwrap();
        assert_eq!(
            store
                .finish(&ticket, Ok(output(7, 103)), 103)
                .unwrap_err()
                .code,
            Code::Stale
        );
    }
    #[test]
    fn limits_and_error_serialization_do_not_include_peer_secrets() {
        let mut store = Store::default();
        for i in 0..8 {
            let mut config = source();
            config.id = format!("source{i}");
            store.register(config).unwrap();
        }
        assert_eq!(store.register(source()).unwrap_err().code, Code::Limit);
        let json = serde_json::to_string(&store.augment(SecuritySnapshot::default(), 100)).unwrap();
        assert!(!json.contains(&"a".repeat(64)));
        assert!(json.contains("unknown"));
        let mut invalid = source();
        invalid.id = "../source".into();
        assert!(Store::default().register(invalid).is_err());
    }
    pub struct Fixture {
        pub at: u64,
    }
    impl RouterAdapter for Fixture {
        fn collect(&mut self, _: &Source, budget: &Budget) -> Result<ReadOutput> {
            budget.check()?;
            Ok(output(1, self.at))
        }
    }
    #[test]
    fn read_deadline_and_cancellation_return_without_waiting_for_uncooperative_adapter() {
        let _guard = READ_LOCK.lock().unwrap();
        struct Slow;
        impl RouterAdapter for Slow {
            fn collect(&mut self, _: &Source, _: &Budget) -> Result<ReadOutput> {
                std::thread::sleep(Duration::from_millis(120));
                Ok(output(1, 100))
            }
        }
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        let begin = Instant::now();
        let result = ticket.collect(Box::new(Slow), Duration::from_millis(15));
        let error = store.finish(&ticket, result, 100).unwrap_err();
        assert_eq!(
            store
                .augment(SecuritySnapshot::default(), 100)
                .router_sources[0]
                .stage,
            "failed"
        );
        assert_eq!(error.code, Code::Timeout);
        assert!(begin.elapsed() < Duration::from_millis(100));
        let stopped = Arc::new(AtomicBool::new(true));
        assert_eq!(
            read(
                Box::new(Fixture { at: 100 }),
                source(),
                stopped,
                Duration::from_secs(1)
            )
            .err()
            .unwrap()
            .code,
            Code::Cancelled
        );
        let ticket = store.begin("home").unwrap();
        let result = ticket.collect(Box::new(Fixture { at: 100 }), Duration::from_secs(1));
        store.finish(&ticket, result, 100).unwrap();
        assert_eq!(
            store.augment(SecuritySnapshot::default(), 100).flows.len(),
            1
        );
        // Wait only for this intentionally noncooperative fixture, so later tests see no worker slot.
        std::thread::sleep(Duration::from_millis(130));
        struct Oversized;
        impl RouterAdapter for Oversized {
            fn collect(&mut self, _: &Source, _: &Budget) -> Result<ReadOutput> {
                let mut output = output(1, 100);
                output.payload = vec![0; MAX_BYTES + 1];
                Ok(output)
            }
        }
        assert_eq!(
            read(
                Box::new(Oversized),
                source(),
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(1)
            )
            .err()
            .unwrap()
            .code,
            Code::Limit
        );
    }
    #[test]
    fn ipv6_is_canonical_and_scope_is_not_discarded() {
        let mut store = store();
        let ticket = store.begin("home").unwrap();
        let mut out = output(1, 100);
        mutate(&mut out, |v| {
            v["records"][0]["localEndpoint"] = "[fe80::20%4]:50000".into();
            v["records"][0]["remoteEndpoint"] = "[fe80::1%4]:443".into();
        });
        store.finish(&ticket, Ok(out), 100).unwrap();
        let result = store.augment(SecuritySnapshot::default(), 100);
        assert_eq!(result.flows[0].remote_address, "fe80::1%4");
        assert!(result.correlations.is_empty());
    }
}
#[cfg(test)]
mod authority_tests {
    use super::*;
    #[test]
    fn control_authority_is_exact_expiring_and_not_derived_from_read_capability() {
        let source = super::tests::source();
        let action = ControlAction::SetFirewall(false);
        let permit = ControlAuthorization {
            source: source.clone(),
            action,
            expires: Instant::now() + Duration::from_secs(1),
        };
        let budget = Budget {
            end: Instant::now() + Duration::from_secs(1),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let peer = Peer::authenticated(source.endpoint, "a".repeat(64));
        permit.check(&source, action, &peer, &budget).unwrap();
        assert_eq!(
            permit
                .check(&source, ControlAction::SetFirewall(true), &peer, &budget)
                .unwrap_err()
                .code,
            Code::SourceMismatch
        );
        assert_eq!(
            permit
                .check(
                    &source,
                    ControlAction::Quarantine("192.168.1.20".parse().unwrap()),
                    &peer,
                    &budget
                )
                .unwrap_err()
                .code,
            Code::SourceMismatch
        );
        assert_eq!(
            permit
                .check(&source, action, &Peer::anonymous(source.endpoint), &budget)
                .unwrap_err()
                .code,
            Code::Authentication
        );
        let expired = ControlAuthorization {
            source: source.clone(),
            action,
            expires: Instant::now(),
        };
        assert!(expired.check(&source, action, &peer, &budget).is_err());
        let mut other = source.clone();
        other.endpoint = "192.168.1.2:443".parse().unwrap();
        assert!(permit.check(&other, action, &peer, &budget).is_err());
    }
    #[test]
    fn worker_capacity_remains_bounded_for_noncooperative_adapters() {
        let _guard = super::tests::READ_LOCK.lock().unwrap();
        struct Hold {
            started: std::sync::mpsc::Sender<()>,
            release: std::sync::mpsc::Receiver<()>,
        }
        impl RouterAdapter for Hold {
            fn collect(&mut self, _: &Source, _: &Budget) -> Result<ReadOutput> {
                self.started.send(()).unwrap();
                let _ = self.release.recv_timeout(Duration::from_secs(2));
                Ok(super::tests::output(1, 100))
            }
        }
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let mut releases = vec![];
        let mut readers = vec![];
        for _ in 0..2 {
            let (tx, rx) = std::sync::mpsc::channel();
            releases.push(tx);
            let started = started_tx.clone();
            readers.push(std::thread::spawn(move || {
                read(
                    Box::new(Hold {
                        started,
                        release: rx,
                    }),
                    super::tests::source(),
                    Arc::new(AtomicBool::new(false)),
                    Duration::from_secs(1),
                )
            }));
        }
        for _ in 0..2 {
            started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        }
        assert_eq!(
            read(
                Box::new(super::tests::Fixture { at: 100 }),
                super::tests::source(),
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(1)
            )
            .err()
            .unwrap()
            .code,
            Code::Busy
        );
        for release in releases {
            release.send(()).unwrap();
        }
        for reader in readers {
            reader.join().unwrap().unwrap();
        }
    }
}
