use crate::{enforcement::Enforcement, model::*, renderer};
use std::{collections::HashSet, fs, path::PathBuf, sync::Mutex};
pub struct SharedState {
    pub inner: Mutex<Session>,
    pub path: PathBuf,
    _document_lock: fs::File,
}
pub struct Session {
    pub ports: crate::ports::Snapshot,
    ports_focus: u64,
    ports_comparator: crate::ports::Comparator,
    ports_operation: u64,
    ports_cancel: bool,
    ports_plan: Option<crate::ports::Plan>,
    pub network: crate::network::NetworkSnapshot,
    pub discovery: crate::discovery::DiscoverySnapshot,
    pub inventory: crate::inventory::Inventory,
    discovery_operation: u64,
    discovery_cancel: bool,
    discovery_plan: Option<crate::discovery::Plan>,
    pub extension: Enforcement,
    pub document: Document,
    pub demo: Vec<Connection>,
    pub connections: Vec<Connection>,
    pub history: Vec<Connection>,
    pub selected_id: Option<String>,
    pub firewall: FirewallStatus,
    pub last_observed: Option<u64>,
    pub revision: u64,
}
impl SharedState {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let parent = path.parent().ok_or("missing_data_directory")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        // Keep a separate, stable inode locked across atomic document replacement.
        let document_lock = options
            .open(path.with_extension("lock"))
            .map_err(|e| e.to_string())?;
        document_lock.try_lock().map_err(|_| "FlowGuard is already running for this document. Close the other instance and retry.".to_string())?;
        let document = if path.exists() {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            serde_json::from_slice::<Document>(&bytes)
                .map_err(|e| format!("Cannot read saved document: {e}"))?
        } else {
            Document::default()
        };
        if document.rules.len() > 2000 || !document.rules.iter().all(validate_rule) {
            return Err("Invalid saved rules".into());
        }
        let p = &document.preferences;
        if !["ja", "en", "zh-CN"].contains(&p.locale.as_str())
            || ![
                "map",
                "apps",
                "rules",
                "history",
                "settings",
                "network",
                "servers",
                "diagnostics",
            ]
            .contains(&p.view.as_str())
            || !["demo", "live"].contains(&p.mode.as_str())
            || !["left", "right", "floating"].contains(&p.panel.as_str())
        {
            return Err("Invalid saved preferences".into());
        }
        let mut document = document;
        document.preferences.panel = "right".into(); // no floating window survives restart
        Ok(Self {
            path,
            _document_lock: document_lock,
            inner: Mutex::new(Session {
                ports: Default::default(),
                ports_focus: 0,
                ports_comparator: Default::default(),
                ports_operation: 0,
                ports_cancel: false,
                ports_plan: None,
                network: Default::default(),
                discovery: Default::default(),
                inventory: Default::default(),
                discovery_operation: 0,
                discovery_cancel: false,
                discovery_plan: None,
                extension: Enforcement::default(),
                document,
                demo: demo_connections(),
                connections: vec![],
                history: vec![],
                selected_id: None,
                firewall: FirewallStatus {
                    enabled: None,
                    enforcement: "unavailable".into(),
                    observer: "starting".into(),
                    error: None,
                },
                last_observed: None,
                revision: 0,
            }),
        })
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| "state_unavailable")?
            .snapshot())
    }
    fn persist(&self, doc: &Document) -> Result<(), String> {
        let parent = self.path.parent().ok_or("missing_data_directory")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("tmp");
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp).map_err(|e| e.to_string())?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }
    pub fn dispatch(&self, command: Command) -> Result<Snapshot, String> {
        self.dispatch_checked(command, None)
    }
    pub fn dispatch_checked(
        &self,
        command: Command,
        expected_rules: Option<&[Rule]>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if matches!(
            &command,
            Command::SaveRule { .. } | Command::DeleteRule { .. } | Command::RestoreRules { .. }
        ) {
            if let Some(expected) = expected_rules {
                if expected != s.document.rules.as_slice() {
                    return Err("edit_conflict".into());
                }
            }
        }
        let previous = s.document.clone();
        let previous_selected = s.selected_id.clone();
        match command {
            Command::Select { id } => {
                if !s.visible().iter().any(|c| c.id == id) {
                    return Err("connection_expired".into());
                }
                s.selected_id = Some(id);
            }
            Command::SetView { view } => {
                if ![
                    "map",
                    "apps",
                    "rules",
                    "history",
                    "settings",
                    "network",
                    "servers",
                    "diagnostics",
                ]
                .contains(&view.as_str())
                {
                    return Err("invalid_view".into());
                }
                s.document.preferences.view = view;
            }
            Command::SetLocale { locale } => {
                if !["ja", "en", "zh-CN"].contains(&locale.as_str()) {
                    return Err("invalid_locale".into());
                }
                s.document.preferences.locale = locale;
            }
            Command::SetMode { mode } => {
                if !["demo", "live"].contains(&mode.as_str()) {
                    return Err("invalid_mode".into());
                }
                s.document.preferences.mode = mode;
                s.selected_id = s.visible().first().map(|c| c.id.clone());
            }
            Command::SetPanel { panel } => {
                if !["left", "right", "floating"].contains(&panel.as_str()) {
                    return Err("invalid_panel".into());
                }
                s.document.preferences.panel = panel;
            }
            Command::SaveRule {
                connection_id,
                action,
                direction,
            } => {
                if !["allow", "block", "ask"].contains(&action.as_str())
                    || !["outbound", "inbound", "both"].contains(&direction.as_str())
                {
                    return Err("invalid_rule".into());
                }
                let c = s
                    .visible()
                    .into_iter()
                    .find(|c| c.id == connection_id)
                    .ok_or("connection_expired")?;
                if c.app_key.is_empty() {
                    return Err("application_identity_unavailable".into());
                }
                let source = s.document.preferences.mode.clone();
                let id = format!(
                    "{}:{}:{}:{}:{}",
                    source, c.app_key, c.destination, c.port, direction
                );
                let rule = Rule {
                    id: id.clone(),
                    app_key: c.app_key,
                    app: c.app,
                    destination: c.destination,
                    port: c.port,
                    direction,
                    action,
                    updated_at: now(),
                    source,
                };
                if let Some(index) = s.document.rules.iter().position(|r| r.id == id) {
                    s.document.rules[index] = rule;
                } else {
                    if s.document.rules.len() >= 2000 {
                        return Err("rule_limit".into());
                    }
                    s.document.rules.push(rule);
                }
            }
            Command::DeleteRule { id } => s.document.rules.retain(|r| r.id != id),
            Command::RestoreRules { rules } => {
                if rules.len() > 2000 || !rules.iter().all(validate_rule) {
                    return Err("invalid_backup".into());
                }
                let ids: HashSet<_> = rules.iter().map(|r| &r.id).collect();
                if ids.len() != rules.len() {
                    return Err("duplicate_rules".into());
                }
                s.document.rules = rules;
            }
        }
        if let Err(e) = self.persist(&s.document) {
            s.document = previous;
            s.selected_id = previous_selected;
            return Err(e);
        }
        let rules = s.document.rules.clone();
        s.extension.reconcile(&rules);
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn extension_change(
        &self,
        change: impl FnOnce(&mut Enforcement, &[Rule]),
    ) -> Result<Snapshot, String> {
        let mut session = self.inner.lock().map_err(|_| "state_unavailable")?;
        let rules = session.document.rules.clone();
        change(&mut session.extension, &rules);
        session.revision += 1;
        Ok(session.snapshot())
    }
    pub fn begin_extension(&self, stage: &str) -> Result<(u64, Vec<Rule>, Snapshot), String> {
        let mut session = self.inner.lock().map_err(|_| "state_unavailable")?;
        if stage == "activating" {
            crate::extension::policy_payload(&session.document.rules)?;
        }
        let operation = session.extension.begin(stage)?;
        session.revision += 1;
        Ok((
            operation,
            session
                .document
                .rules
                .iter()
                .filter(|r| r.source == "live")
                .cloned()
                .collect(),
            session.snapshot(),
        ))
    }
    pub fn lan_selection(&self, id: &str) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        s.inventory.select(id)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn lan_panel(&self, panel: &str) -> Result<Snapshot, String> {
        if !["docked", "floating"].contains(&panel) {
            return Err("invalid_panel".into());
        }
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        s.inventory.snapshot.panel = panel.into();
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn lan_csv(&self, generation: u64) -> Result<String, String> {
        let s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.inventory.snapshot.generation != generation
            || ["running", "cancelling", "idle"].contains(&s.discovery.stage.as_str())
        {
            return Err("export_stale_or_busy".into());
        }
        Ok(crate::inventory::csv(&s.inventory.snapshot, &s.discovery))
    }
    pub fn open_ports(&self, id: &str, generation: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.inventory.snapshot.generation != generation
            || !["completed", "partial"].contains(&s.discovery.stage.as_str())
            || !s
                .inventory
                .snapshot
                .rows
                .iter()
                .any(|r| r.device.id == id && r.kind != "not_seen")
            || !s.discovery.devices.iter().any(|d| {
                d.id == id
                    && d.family == "IPv4"
                    && d.address
                        .parse::<std::net::Ipv4Addr>()
                        .is_ok_and(|ip| ip.is_private())
            })
        {
            return Err("stale_target".into());
        }
        let mut document = s.document.clone();
        document.preferences.view = "network".into();
        self.persist(&document)?;
        s.inventory.select(id)?;
        s.document = document;
        s.ports_focus += 1;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn ports_csv(&self, operation: u64) -> Result<String, String> {
        let s = self.inner.lock().map_err(|_| "state_lock")?;
        if operation == 0
            || s.ports.operation != operation
            || ["running", "cancelling", ""].contains(&s.ports.stage.as_str())
            || s.ports.rows.is_empty()
        {
            return Err("stale_or_unfinished_ports".into());
        }
        Ok(crate::ports::csv(&s.ports, s.inventory.snapshot.generation))
    }
    pub fn begin_ports(&self, plan: &crate::ports::Plan) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if ["running", "cancelling"].contains(&s.ports.stage.as_str())
            || ["running", "cancelling"].contains(&s.discovery.stage.as_str())
            || s.inventory.snapshot.generation != plan.generation
        {
            return Err("ports_busy_or_stale".into());
        }
        s.ports_operation += 1;
        s.ports_cancel = false;
        s.ports = plan.snapshot();
        s.ports.comparison = s.ports_comparator.begin(plan);
        s.ports.operation = s.ports_operation;
        s.ports_plan = Some(plan.clone());
        s.revision += 1;
        Ok((s.ports_operation, s.snapshot()))
    }
    pub fn ports_cancelled(&self, op: u64) -> bool {
        self.inner
            .lock()
            .map(|s| {
                s.ports_operation != op
                    || s.ports_cancel
                    || s.inventory.snapshot.generation != s.ports.generation
            })
            .unwrap_or(true)
    }
    pub fn stop_ports(&self) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.ports.stage == "running" {
            s.ports_cancel = true;
            s.ports.stage = "cancelling".into();
            s.revision += 1;
        }
        Ok(s.snapshot())
    }
    pub fn update_ports(
        &self,
        op: u64,
        mut result: crate::ports::Snapshot,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.ports_operation != op || !["running", "cancelling"].contains(&s.ports.stage.as_str()) {
            return Err("stale_ports".into());
        }
        if s.ports_cancel || s.inventory.snapshot.generation != s.ports.generation {
            if result.stage == "running" {
                return Err("cancelled".into());
            }
            result = s.ports.clone();
            result.stage = "cancelled".into();
        }
        result.operation = op;
        if let Some(plan) = s.ports_plan.clone() {
            s.ports_comparator.update(&plan, &mut result);
        }
        s.ports = result;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_discovery(
        &self,
        plan: &crate::discovery::Plan,
    ) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if ["running", "cancelling"].contains(&s.discovery.stage.as_str()) {
            return Err("discovery_busy".into());
        }
        if ["running", "cancelling"].contains(&s.ports.stage.as_str()) {
            return Err("ports_busy".into());
        }
        s.ports_comparator.invalidate();
        s.discovery_operation += 1;
        s.discovery_cancel = false;
        let generation = s.discovery_operation;
        s.inventory.begin(plan, generation);
        s.discovery_plan = Some(plan.clone());
        s.discovery = crate::discovery::DiscoverySnapshot {
            name_mode: plan.name_mode.clone(),
            stage: "running".into(),
            interface: plan.local.interface.clone(),
            range: plan.range.clone(),
            active: plan.active,
            total: plan.targets.len(),
            ..Default::default()
        };
        s.revision += 1;
        Ok((s.discovery_operation, s.snapshot()))
    }
    pub fn discovery_cancelled(&self, operation: u64) -> bool {
        self.inner
            .lock()
            .map(|s| s.discovery_operation != operation || s.discovery_cancel)
            .unwrap_or(true)
    }
    pub fn stop_discovery(&self) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.discovery.stage == "running" {
            s.discovery_cancel = true;
            s.discovery.stage = "cancelling".into();
            s.revision += 1;
        }
        Ok(s.snapshot())
    }
    pub fn update_discovery(
        &self,
        operation: u64,
        mut result: crate::discovery::DiscoverySnapshot,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.discovery_operation != operation
            || !["running", "cancelling"].contains(&s.discovery.stage.as_str())
        {
            return Err("stale_discovery".into());
        }
        if s.discovery_cancel {
            // Freeze published rows at stop; do not publish any late batch results.
            if result.stage == "running" {
                return Err("discovery_cancelled".into());
            }
            result = s.discovery.clone();
            result.stage = "cancelled".into();
            result.observed_at = Some(now());
        }
        if result.devices.len() > 1024 {
            result.devices.truncate(1024);
            if !result.errors.iter().any(|e| e == "results_truncated") {
                result.errors.push("results_truncated".into());
            }
            if result.stage == "completed" {
                result.stage = "partial".into();
            }
        }
        s.inventory.update(&result);
        s.discovery = result;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn observed(
        &self,
        mut connections: Vec<Connection>,
        firewall: FirewallStatus,
        network: crate::network::NetworkSnapshot,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if firewall.error.is_none() {
            let existing: HashSet<_> = s.connections.iter().map(|c| c.id.clone()).collect();
            for c in &mut connections {
                if let Some(previous) = s.connections.iter().find(|previous| previous.id == c.id) {
                    c.observed_at = previous.observed_at;
                }
                if !existing.contains(&c.id) {
                    s.history.insert(0, c.clone());
                }
            }
            s.history.truncate(500);
            s.connections = connections;
            s.last_observed = Some(now());
        } else {
            s.connections.clear();
        }
        s.firewall = firewall;
        s.network = network;
        if s.ports_plan
            .as_ref()
            .is_some_and(|plan| !plan.matches(&s.network))
            && ["completed", "cancelled"].contains(&s.ports.stage.as_str())
        {
            s.ports.stage = "failed".into();
            s.ports.error = Some("network_changed".into());
            s.ports_comparator.invalidate();
            s.ports.comparison = crate::ports::Comparison {
                status: "invalidated".into(),
                ..Default::default()
            };
        }
        if s.discovery_plan
            .as_ref()
            .is_some_and(|plan| !plan.matches(&s.network))
            && ["completed", "partial", "cancelled"].contains(&s.discovery.stage.as_str())
        {
            s.discovery.stage = "failed".into();
            if !s.discovery.errors.iter().any(|e| e == "network_changed") {
                s.discovery.errors.push("network_changed".into());
            }
        }
        if s.discovery.errors.iter().any(|e| e == "network_changed") {
            let result = s.discovery.clone();
            s.inventory.invalidate(&result);
        }
        if !s
            .visible()
            .iter()
            .any(|c| Some(&c.id) == s.selected_id.as_ref())
        {
            s.selected_id = s.visible().first().map(|c| c.id.clone());
        }
        s.revision += 1;
        Ok(s.snapshot())
    }
}
impl Session {
    pub fn visible(&self) -> Vec<Connection> {
        if self.document.preferences.mode == "demo" {
            self.demo.clone()
        } else {
            self.connections.clone()
        }
    }
    fn snapshot(&self) -> Snapshot {
        let connections = self.visible();
        let layout = renderer::layout(&connections);
        Snapshot {
            ports_focus: self.ports_focus,
            ports: self.ports.clone(),
            network: self.network.clone(),
            discovery: self.discovery.clone(),
            inventory: self.inventory.snapshot.clone(),
            revision: self.revision,
            extension: self.extension.status.clone(),
            document: self.document.clone(),
            applications: group_applications(&connections),
            connections,
            history: if self.document.preferences.mode == "demo" {
                self.demo.clone()
            } else {
                self.history.clone()
            },
            selected_id: self.selected_id.clone(),
            firewall: self.firewall.clone(),
            last_observed: self.last_observed,
            layout,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> SharedState {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "flowguard-{}-{}-{}.json",
            std::process::id(),
            now(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        SharedState::load(path).unwrap()
    }
    #[test]
    fn tcp_stop_and_scope_fencing() {
        let state = state();
        let network = crate::network::NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.20".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let discovery =
            crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false).unwrap();
        let (op, started) = state.begin_discovery(&discovery).unwrap();
        let mut result = started.discovery;
        result.stage = "completed".into();
        result.devices = vec![crate::discovery::Device {
            id: "en0|192.168.1.1".into(),
            interface: "en0".into(),
            address: "192.168.1.1".into(),
            family: "IPv4".into(),
            mac: None,
            source: "arp".into(),
            response: "unverified".into(),
            observed_at: now(),
            name: Default::default(),
        }];
        state.update_discovery(op, result).unwrap();
        let snapshot = state.snapshot().unwrap();
        let opened = state.open_ports("en0|192.168.1.1", op).unwrap();
        assert_eq!(opened.ports_focus, 1);
        assert_eq!(opened.document.preferences.view, "network");
        assert_eq!(
            opened.inventory.selected_id.as_deref(),
            Some("en0|192.168.1.1")
        );
        assert_eq!(opened.ports.operation, 0);
        assert!(state.open_ports("missing", op).is_err());
        assert!(state.open_ports("en0|192.168.1.1", op + 1).is_err());
        let make = |a, b| crate::ports::Plan::new(&snapshot, &network, "en0|192.168.1.1", op, a, b);
        assert!(make(0, 1).is_err());
        assert!(make(1, 257).is_err());
        assert!(make(20, 10).is_err());
        assert!(make(1, 256).is_ok());
        assert!(crate::ports::Plan::new(&snapshot, &network, "missing", op, 1, 1).is_err());
        assert!(
            crate::ports::Plan::new(&snapshot, &network, "en0|192.168.1.1", op + 1, 1, 1).is_err()
        );
        let mut outside = snapshot.clone();
        outside.discovery.devices[0].address = "8.8.8.8".into();
        assert!(
            crate::ports::Plan::new(&outside, &network, "en0|192.168.1.1", op, 22, 22).is_err()
        );
        outside.discovery.devices[0].address = "192.168.2.1".into();
        assert!(
            crate::ports::Plan::new(&outside, &network, "en0|192.168.1.1", op, 22, 22).is_err()
        );
        outside.discovery.devices[0].address = "fe80::1".into();
        assert!(
            crate::ports::Plan::new(&outside, &network, "en0|192.168.1.1", op, 22, 22).is_err()
        );
        let plan = make(22, 22).unwrap();
        assert!(!plan.matches(&Default::default()));
        let (job, started) = state.begin_ports(&plan).unwrap();
        assert_eq!(started.ports.operation, job);
        assert!(state.ports_csv(job).is_err());
        assert!(state.begin_discovery(&discovery).is_err());
        assert!(state.begin_ports(&plan).is_err());
        state.stop_ports().unwrap();
        assert!(state.ports_cancelled(job));
        assert!(state.update_ports(job, started.ports.clone()).is_err());
        let mut late = started.ports;
        late.stage = "completed".into();
        late.rows.push(crate::ports::Row {
            port: 22,
            status: "open".into(),
            elapsed_ms: 1,
            observed_at: now(),
        });
        let frozen = state.update_ports(job, late).unwrap();
        assert_eq!(frozen.ports.stage, "cancelled");
        assert!(frozen.ports.rows.is_empty());
        let (next, _) = state.begin_ports(&plan).unwrap();
        assert_ne!(job, next);
        assert!(state.update_ports(job, Default::default()).is_err());
        let mut finished = state.snapshot().unwrap().ports;
        finished.stage = "completed".into();
        finished.completed = 1;
        finished.rows = vec![crate::ports::Row {
            port: 22,
            status: "open".into(),
            elapsed_ms: 2,
            observed_at: now(),
        }];
        state.update_ports(next, finished).unwrap();
        assert!(state.ports_csv(job).is_err());
        assert!(state.ports_csv(next).unwrap().contains("requested_ports"));
        state.begin_discovery(&discovery).unwrap();
        assert!(state.ports_csv(next).unwrap().contains("\"true\""));
    }
    #[test]
    fn discovery_snapshot_caps_results_even_during_a_job() {
        let state = state();
        let network = crate::network::NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.20".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let plan = crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false).unwrap();
        let (operation, snapshot) = state.begin_discovery(&plan).unwrap();
        let mut result = snapshot.discovery;
        result.devices = (0..1025)
            .map(|n| crate::discovery::Device {
                name: Default::default(),
                id: n.to_string(),
                interface: "en0".into(),
                address: format!("fd00::{n:x}"),
                family: "IPv6".into(),
                mac: None,
                source: "ndp".into(),
                response: "unverified".into(),
                observed_at: now(),
            })
            .collect();
        let updated = state.update_discovery(operation, result.clone()).unwrap();
        assert_eq!(updated.discovery.devices.len(), 1024);
        assert!(updated
            .discovery
            .errors
            .iter()
            .any(|e| e == "results_truncated"));
        result.stage = "completed".into();
        assert_eq!(
            state
                .update_discovery(operation, result)
                .unwrap()
                .discovery
                .stage,
            "partial"
        );
    }
    #[test]
    fn stop_freezes_name_progress_and_rejects_late_names() {
        let state = state();
        let network = crate::network::NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.20".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let plan = crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false)
            .unwrap()
            .with_names("mdns")
            .unwrap();
        assert!(plan.clone().with_names("arbitrary").is_err());
        let (operation, snapshot) = state.begin_discovery(&plan).unwrap();
        let mut result = snapshot.discovery;
        assert_eq!(result.name_mode, "mdns");
        result.names_total = 1;
        result.devices = vec![crate::discovery::Device {
            id: "en0|192.168.1.1".into(),
            interface: "en0".into(),
            address: "192.168.1.1".into(),
            family: "IPv4".into(),
            mac: None,
            source: "arp".into(),
            response: "unverified".into(),
            observed_at: now(),
            name: crate::names::NameObservation {
                status: "pending".into(),
                ..Default::default()
            },
        }];
        state.update_discovery(operation, result.clone()).unwrap();
        state.stop_discovery().unwrap();
        result.stage = "completed".into();
        result.names_completed = 1;
        result.devices[0].name.value = Some("late.local".into());
        result.devices[0].name.status = "resolved".into();
        let stopped = state.update_discovery(operation, result).unwrap();
        assert_eq!(stopped.discovery.stage, "cancelled");
        assert_eq!(stopped.discovery.names_completed, 0);
        assert!(stopped.discovery.devices[0].name.value.is_none());
    }
    #[test]
    fn inventory_selection_panel_and_csv_use_shared_generation() {
        let state = state();
        let network = crate::network::NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.20".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let plan = crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false).unwrap();
        let (op, started) = state.begin_discovery(&plan).unwrap();
        assert!(state.lan_csv(op).is_err());
        let mut result = started.discovery;
        result.stage = "completed".into();
        result.devices = vec![crate::discovery::Device {
            id: "en0|192.168.1.1".into(),
            interface: "en0".into(),
            address: "192.168.1.1".into(),
            family: "IPv4".into(),
            mac: None,
            source: "arp".into(),
            response: "unverified".into(),
            observed_at: now(),
            name: Default::default(),
        }];
        state.update_discovery(op, result).unwrap();
        assert!(state.lan_selection("missing").is_err());
        let selected = state.lan_selection("en0|192.168.1.1").unwrap();
        assert_eq!(
            selected.inventory.selected_id.as_deref(),
            Some("en0|192.168.1.1")
        );
        assert_eq!(
            state.lan_panel("floating").unwrap().inventory.panel,
            "floating"
        );
        assert_eq!(state.lan_panel("docked").unwrap().inventory.panel, "docked");
        assert!(state.lan_csv(op).unwrap().contains("192.168.1.1"));
        state.begin_discovery(&plan).unwrap();
        assert!(state.lan_csv(op).is_err());
        assert!(state.snapshot().unwrap().inventory.selected_id.is_none());
    }
    #[test]
    fn discovery_cancellation_fences_late_results_and_new_jobs() {
        let state = state();
        let network = crate::network::NetworkSnapshot {
            addresses: vec![crate::network::InterfaceAddress {
                interface: "en0".into(),
                address: "192.168.1.20".into(),
                family: "IPv4".into(),
                netmask: "255.255.255.0".into(),
                up: true,
            }],
            ..Default::default()
        };
        let plan = crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false).unwrap();
        let (op, started) = state.begin_discovery(&plan).unwrap();
        assert!(state.begin_discovery(&plan).is_err());
        state.stop_discovery().unwrap();
        assert!(state.discovery_cancelled(op));
        assert!(state
            .update_discovery(op, started.discovery.clone())
            .is_err());
        let mut completed = started.discovery;
        completed.stage = "completed".into();
        assert_eq!(
            state
                .update_discovery(op, completed.clone())
                .unwrap()
                .discovery
                .stage,
            "cancelled"
        );
        let (next, _) = state.begin_discovery(&plan).unwrap();
        assert_ne!(op, next);
        assert!(state.update_discovery(op, completed).is_err());
        assert!(!state.discovery_cancelled(next));
        let current = state.snapshot().unwrap();
        let observed = state.observed(vec![], current.firewall, network).unwrap();
        assert_eq!(observed.discovery.stage, "running");
        assert_eq!(observed.discovery.interface, "en0");
        let mut finished = observed.discovery;
        finished.stage = "completed".into();
        state.update_discovery(next, finished).unwrap();
        let current = state.snapshot().unwrap();
        let changed = state
            .observed(vec![], current.firewall, Default::default())
            .unwrap();
        assert_eq!(changed.discovery.stage, "failed");
        assert!(changed
            .discovery
            .errors
            .iter()
            .any(|e| e == "network_changed"));
    }

    #[test]
    fn activation_validation_precedes_state_change_and_captures_only_live_rules() {
        let s = state();
        s.extension_change(|extension, _| extension.configure(true, true, true))
            .unwrap();
        let rule = Rule {
            id: "r".into(),
            app_key: "/test".into(),
            app: "Test".into(),
            destination: "1.2.3.4".into(),
            port: 443,
            direction: "both".into(),
            action: "ask".into(),
            updated_at: 1,
            source: "live".into(),
        };
        s.inner.lock().unwrap().document.rules = vec![rule.clone()];
        let previous = s.snapshot().unwrap().revision;
        assert_eq!(
            s.begin_extension("activating").err().unwrap(),
            "unsupported_or_invalid_rules"
        );
        assert_eq!(s.snapshot().unwrap().revision, previous);
        assert!(!s.snapshot().unwrap().extension.busy);
        let mut valid = rule.clone();
        valid.action = "block".into();
        let mut demo = rule;
        demo.source = "demo".into();
        demo.id = "demo".into();
        s.inner.lock().unwrap().document.rules = vec![valid.clone(), demo];
        let (_, sent, snapshot) = s.begin_extension("activating").unwrap();
        assert_eq!(sent, vec![valid]);
        assert!(snapshot.extension.busy);
    }
    #[test]
    fn saved_rules_remain_unapplied_and_roundtrip() {
        let s = state();
        s.dispatch(Command::SetMode {
            mode: "demo".into(),
        })
        .unwrap();
        let snap = s
            .dispatch(Command::SaveRule {
                connection_id: "demo-0".into(),
                action: "block".into(),
                direction: "both".into(),
            })
            .unwrap();
        assert_eq!(snap.firewall.enforcement, "unavailable");
        assert_eq!(snap.document.rules.len(), 1);
        let path = s.path.clone();
        drop(s);
        assert_eq!(
            SharedState::load(path.clone())
                .unwrap()
                .snapshot()
                .unwrap()
                .document
                .rules[0]
                .action,
            "block"
        );
        let _ = fs::remove_file(&path);
    }
    #[test]
    fn duplicate_instance_is_rejected_and_lock_releases_on_exit() {
        let s = state();
        let path = s.path.clone();
        assert!(SharedState::load(path.clone()).is_err());
        drop(s);
        assert!(SharedState::load(path.clone()).is_ok());
        let _ = fs::remove_file(path.with_extension("lock"));
    }
    #[test]
    fn stale_rule_edits_cannot_replace_newer_rules() {
        let s = state();
        s.dispatch(Command::SetMode {
            mode: "demo".into(),
        })
        .unwrap();
        let previous = s.snapshot().unwrap().document.rules;
        s.dispatch_checked(
            Command::SaveRule {
                connection_id: "demo-0".into(),
                action: "block".into(),
                direction: "both".into(),
            },
            Some(&previous),
        )
        .unwrap();
        assert_eq!(
            s.dispatch_checked(Command::RestoreRules { rules: vec![] }, Some(&previous))
                .err()
                .unwrap(),
            "edit_conflict"
        );
        let current = s.snapshot().unwrap().document.rules;
        assert_eq!(current.len(), 1);
        s.dispatch_checked(
            Command::DeleteRule {
                id: current[0].id.clone(),
            },
            Some(&current),
        )
        .unwrap();
        assert!(s.snapshot().unwrap().document.rules.is_empty());
    }
    #[test]
    fn failed_persistence_rolls_back() {
        let mut s = state();
        s.path = PathBuf::from("/dev/null/document.json");
        assert!(s
            .dispatch(Command::SetLocale {
                locale: "en".into()
            })
            .is_err());
        assert_eq!(s.snapshot().unwrap().document.preferences.locale, "ja");
    }
}
