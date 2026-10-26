pub const ALREADY_RUNNING: &str =
    "FlowGuard is already running for this document. Close the other instance and retry.";
use crate::{enforcement::Enforcement, model::*, renderer};
use std::{collections::HashSet, fs, path::PathBuf, sync::Mutex};
pub struct SharedState {
    pub inner: Mutex<Session>,
    pub path: PathBuf,
    _document_lock: fs::File,
}
pub struct Session {
    pub gpu_owner: String,
    pub gpu_float_operation: u64,
    pub gpu_dock_operation: u64,
    pub temporary: crate::temporary_protection::Machine,
    pub flow_names: crate::flow_names::Machine,
    pub gpu_filter: crate::gpu_filter::Filter,
    diagnostics: crate::diagnostics::Snapshot,
    destination_names: crate::destination_names::Machine,
    firewall_control: crate::firewall_control::Machine,
    syslog: crate::syslog::Machine,
    routers: crate::routers::Store,
    pub asks: crate::asks::Machine,
    pub updates: crate::updates::Machine,
    pub services: crate::services::Snapshot,
    services_comparator: crate::service_comparison::Comparator,
    services_operation: u64,
    services_cancel: bool,
    services_plan: Option<crate::services::Plan>,
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
    demo_history: Vec<Connection>,
    pub connections: Vec<Connection>,
    pub history: Vec<Connection>,
    pub selected_id: Option<String>,
    pub firewall: FirewallStatus,
    pub last_observed: Option<u64>,
    pub revision: u64,
}
impl SharedState {
    pub fn icon_subject(&self, subject: &str) -> Option<std::path::PathBuf> {
        let s = self.inner.lock().ok()?;
        if s.document.preferences.mode != "live" {
            return None;
        }
        s.connections
            .iter()
            .chain(s.history.iter())
            .find(|c| {
                !c.app_key.is_empty()
                    && (c.app_key == subject
                        || c.identity
                            .as_ref()
                            .is_some_and(|id| id.valid() && id.key() == subject))
            })
            .map(|c| std::path::PathBuf::from(&c.app_key))
    }

    pub fn configure_destination_names(
        &self,
        enabled: bool,
        expected: u64,
        fresh: Option<crate::network::NetworkSnapshot>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if enabled {
            if s.updates.snapshot.stage == "installing"
                || s.document.preferences.mode != "live"
                || s.firewall.error.is_some()
            {
                return Err("name_lookup_unavailable".into());
            }
            if expected != s.destination_names.snapshot().operation {
                return Err("name_lookup_stale".into());
            }
            if fresh
                .as_ref()
                .and_then(crate::destination_names::network_key)
                != crate::destination_names::network_key(&s.network)
            {
                return Err("network_changed".into());
            }
        }
        let network = s.network.clone();
        s.destination_names.configure(enabled, &network)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn clear_destination_names(&self, expected: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.destination_names.clear(expected)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_destination_name(
        &self,
        id: &str,
        expected: &str,
        operation: u64,
        fresh: &crate::network::NetworkSnapshot,
    ) -> Result<(crate::destination_names::Ticket, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing"
            || s.document.preferences.mode != "live"
            || s.firewall.error.is_some()
        {
            return Err("name_lookup_unavailable".into());
        }
        if operation != s.destination_names.snapshot().operation {
            return Err("name_lookup_stale".into());
        }
        if crate::destination_names::network_key(fresh)
            != crate::destination_names::network_key(&s.network)
        {
            return Err("network_changed".into());
        }
        if !s
            .connections
            .iter()
            .any(|c| c.id == id && c.destination == expected)
        {
            return Err("connection_expired".into());
        }
        let network = s.network.clone();
        let ticket = s.destination_names.begin(expected, &network)?;
        s.revision += 1;
        Ok((ticket, s.snapshot()))
    }
    pub fn finish_destination_name(
        &self,
        ticket: &crate::destination_names::Ticket,
        value: Option<String>,
        status: &str,
        fresh: &crate::network::NetworkSnapshot,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.destination_names.finish(ticket, value, status, fresh)?;
        s.revision += 1;
        Ok(s.snapshot())
    }

    pub fn begin_firewall_control(
        &self,
        action: crate::firewall_control::Action,
        expected: Option<bool>,
        acknowledged: bool,
    ) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        let op = s.firewall_control.begin(action, expected, acknowledged)?;
        s.revision += 1;
        Ok((op, s.snapshot()))
    }
    pub fn finish_firewall_control(
        &self,
        op: u64,
        result: Result<crate::firewall_control::Reply, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.firewall_control.finish(op, result)?;
        s.revision += 1;
        Ok(s.snapshot())
    }

    pub fn begin_syslog(
        &self,
        config: crate::syslog::Config,
        network: &crate::network::NetworkSnapshot,
    ) -> Result<(crate::syslog::Ticket, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        let ticket = s.syslog.begin(config, network)?;
        s.revision += 1;
        Ok((ticket, s.snapshot()))
    }
    pub fn update_syslog(&self, result: crate::syslog::Snapshot) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.syslog.apply(result)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn stop_syslog(&self, operation: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.syslog.stop(operation)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn clear_syslog(&self, operation: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.syslog.clear(operation)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn select_syslog(&self, operation: u64, id: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.syslog.select(operation, id)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    // Integration-test facade; production router commands are not exposed yet.
    #[cfg(test)]
    pub fn register_router(
        &self,
        source: crate::routers::Source,
    ) -> Result<Snapshot, crate::routers::Error> {
        let mut s = self
            .inner
            .lock()
            .map_err(|_| crate::routers::Code::Internal)?;
        if s.updates.snapshot.stage == "installing" {
            return Err(crate::routers::Code::Busy.into());
        }
        s.routers.register(source)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    // Integration-test facade; production router commands are not exposed yet.
    #[cfg(test)]
    pub fn remove_router(&self, id: &str) -> Result<Snapshot, crate::routers::Error> {
        let mut s = self
            .inner
            .lock()
            .map_err(|_| crate::routers::Code::Internal)?;
        s.routers.remove(id)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    // Integration-test facade; production router commands are not exposed yet.
    #[cfg(test)]
    pub fn stop_router(&self, id: &str) -> Result<Snapshot, crate::routers::Error> {
        let mut s = self
            .inner
            .lock()
            .map_err(|_| crate::routers::Code::Internal)?;
        s.routers.stop(id)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    // Integration-test facade; production router commands are not exposed yet.
    #[cfg(test)]
    pub fn collect_router(
        &self,
        id: &str,
        adapter: Box<dyn crate::routers::RouterAdapter>,
        timeout: std::time::Duration,
    ) -> Result<Snapshot, crate::routers::Error> {
        let ticket = {
            let mut s = self
                .inner
                .lock()
                .map_err(|_| crate::routers::Code::Internal)?;
            if s.updates.snapshot.stage == "installing" {
                return Err(crate::routers::Code::Busy.into());
            }
            let ticket = s.routers.begin(id)?;
            s.revision += 1;
            ticket
        };
        let result = ticket.collect(adapter, timeout);
        let mut s = self
            .inner
            .lock()
            .map_err(|_| crate::routers::Code::Internal)?;
        let result = s.routers.finish(&ticket, result, now());
        if result
            .as_ref()
            .err()
            .is_none_or(|error| error.code != crate::routers::Code::Stale)
        {
            s.revision += 1;
        }
        result?;
        Ok(s.snapshot())
    }
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
        document_lock
            .try_lock()
            .map_err(|_| ALREADY_RUNNING.to_string())?;
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
        if !["stable", "beta"].contains(&p.update_channel.as_str())
            || !["allow", "block"].contains(&p.ask_timeout_action.as_str())
            || !["ja", "en", "zh-CN"].contains(&p.locale.as_str())
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
        let updates = crate::updates::Machine::with_history(
            &path.with_extension("updates.json"),
            &document.preferences.update_channel,
        );
        let inventory = crate::inventory::Inventory::load_archives(&path);
        Ok(Self {
            path,
            _document_lock: document_lock,
            inner: Mutex::new(Session {
                gpu_owner: "main".into(),
                gpu_float_operation: 0,
                gpu_dock_operation: 0,
                temporary: Default::default(),
                flow_names: Default::default(),
                gpu_filter: Default::default(),
                diagnostics: Default::default(),
                destination_names: Default::default(),
                firewall_control: Default::default(),
                syslog: Default::default(),
                routers: Default::default(),
                asks: Default::default(),
                updates,
                services: Default::default(),
                services_comparator: Default::default(),
                services_operation: 0,
                services_cancel: false,
                services_plan: None,
                ports: Default::default(),
                ports_focus: 0,
                ports_comparator: Default::default(),
                ports_operation: 0,
                ports_cancel: false,
                ports_plan: None,
                network: Default::default(),
                discovery: Default::default(),
                inventory,
                discovery_operation: 0,
                discovery_cancel: false,
                discovery_plan: None,
                extension: Enforcement::default(),
                document,
                demo: demo_connections(),
                demo_history: demo_connections(),
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
            Command::SetSystemException { .. }
                | Command::SetAskTimeoutAction { .. }
                | Command::SaveRule { .. }
                | Command::MigrateRule { .. }
                | Command::DeleteRule { .. }
                | Command::RestoreRules { .. }
        ) {
            if let Some(expected) = expected_rules {
                if expected != s.document.rules.as_slice() {
                    return Err("edit_conflict".into());
                }
            }
        }
        let previous = s.document.clone();
        let previous_selected = s.selected_id.clone();
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        match command {
            Command::SetUpdateChannel { channel } => {
                if !["stable", "beta"].contains(&channel.as_str())
                    || ["checking", "downloading", "ready", "installing"]
                        .contains(&s.updates.snapshot.stage.as_str())
                {
                    return Err("updater_busy".into());
                }
                s.document.preferences.update_channel = channel;
            }
            Command::SetHistoryRetention { retention } => {
                s.document.preferences.history_retention = retention
            }
            Command::AcknowledgeProtectionGuide => {
                s.document.preferences.protection_guide_seen = true
            }
            Command::SetUnknownNotifications { enabled } => {
                s.document.preferences.unknown_notifications = enabled
            }
            Command::SetAskNotifications { enabled } => {
                s.document.preferences.ask_notifications = enabled
            }
            Command::SetSystemException { id, enabled } => {
                if expected_rules.is_none() {
                    return Err("edit_conflict".into());
                }
                let rule = s
                    .document
                    .rules
                    .iter_mut()
                    .find(|r| r.id == id)
                    .ok_or("invalid_rule")?;
                if rule.source != "live"
                    || crate::system_protection::preset(
                        rule.identity.as_ref(),
                        rule.port,
                        &rule.direction,
                        "TCP", // v4 rules encode only TCP intentions.
                    )
                    .is_none()
                {
                    return Err("invalid_rule".into());
                }
                rule.system_exception = enabled;
                rule.updated_at = now();
            }

            Command::SetAskTimeoutAction { action } => {
                if !["allow", "block"].contains(&action.as_str()) {
                    return Err("invalid_rule".into());
                }
                s.document.preferences.ask_timeout_action = action.clone();
                for rule in &mut s.document.rules {
                    if rule.action == "ask" {
                        rule.timeout_action = action.clone();
                        rule.updated_at = now();
                    }
                }
            }
            Command::SetAutoUpdates { enabled } => s.document.preferences.auto_updates = enabled,
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
                if c.protocol != "TCP" {
                    return Err("unsupported_protocol".into());
                }
                if c.app_key.is_empty() {
                    return Err("application_identity_unavailable".into());
                }
                let source = s.document.preferences.mode.clone();
                let identity = if source == "live" {
                    let fresh = crate::identity::for_pid(c.pid)
                        .ok_or("application_identity_unavailable")?;
                    if c.identity.as_ref() != Some(&fresh) {
                        return Err("application_identity_changed".into());
                    }
                    Some(fresh)
                } else {
                    None
                };
                let subject = identity
                    .as_ref()
                    .map(|i| i.key())
                    .unwrap_or_else(|| c.app_key.clone());
                let id = format!(
                    "{}:{}:{}:{}:{}",
                    source, subject, c.destination, c.port, direction
                );
                let rule = Rule {
                    transport: "TCP".into(),
                    system_exception: false,
                    timeout_action: s.document.preferences.ask_timeout_action.clone(),
                    identity,
                    id: id.clone(),
                    app_key: subject,
                    app: c.app,
                    destination: c.destination,
                    port: c.port,
                    direction,
                    action,
                    updated_at: now(),
                    source,
                };
                if let Some(index) = s.document.rules.iter().position(|r| {
                    r.source == rule.source
                        && r.destination == rule.destination
                        && r.port == rule.port
                        && r.direction == rule.direction
                        && if rule.source == "live" {
                            r.identity.is_some() && r.identity == rule.identity
                        } else {
                            r.app_key == rule.app_key
                        }
                }) {
                    let mut rule = rule;
                    rule.id = s.document.rules[index].id.clone();
                    s.document.rules[index] = rule;
                } else {
                    if s.document.rules.len() >= 2000 {
                        return Err("rule_limit".into());
                    }
                    s.document.rules.push(rule);
                }
            }
            Command::MigrateRule { id, connection_id } => {
                let index = s
                    .document
                    .rules
                    .iter()
                    .position(|r| r.id == id)
                    .ok_or("connection_expired")?;
                let legacy = s.document.rules[index].clone();
                let c = s
                    .connections
                    .iter()
                    .find(|c| c.id == connection_id)
                    .ok_or("connection_expired")?;
                if c.protocol != "TCP"
                    || legacy.source != "live"
                    || legacy.identity.is_some()
                    || legacy.app_key != c.app_key
                    || legacy.destination != c.destination
                    || legacy.port != c.port
                {
                    return Err("migration_conflict".into());
                }
                let identity =
                    crate::identity::for_pid(c.pid).ok_or("application_identity_unavailable")?;
                if c.identity.as_ref() != Some(&identity) {
                    return Err("application_identity_changed".into());
                }
                if s.document.rules.iter().any(|r| {
                    r.source == "live"
                        && r.identity.as_ref() == Some(&identity)
                        && r.destination == legacy.destination
                        && r.port == legacy.port
                        && r.direction == legacy.direction
                }) {
                    return Err("migration_conflict".into());
                }
                s.document.rules[index].system_exception = false;
                s.document.rules[index].app_key = identity.key();
                s.document.rules[index].identity = Some(identity);
                s.document.rules[index].updated_at = now();
            }
            Command::DeleteRule { id } => s.document.rules.retain(|r| r.id != id),
            Command::RestoreRules { mut rules } => {
                for rule in &mut rules {
                    rule.system_exception = false;
                }
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
        if previous.preferences.update_channel != s.document.preferences.update_channel {
            s.updates.snapshot.channel = s.document.preferences.update_channel.clone();
            s.updates.snapshot.stage = "idle".into();
            s.updates.update = None;
            s.updates.bytes = None;
        }
        // Drop history only after the preference was saved successfully.
        let retention = s.document.preferences.history_retention;
        retention.prune(&mut s.history, now());
        retention.prune(&mut s.demo_history, now());
        if previous.preferences.mode != s.document.preferences.mode {
            s.destination_names.invalidate();
            if s.document.preferences.mode != "live" {
                s.temporary.clear();
            }
            {
                let mode = s.document.preferences.mode.clone();
                let network = s.network.clone();
                let instance = s.extension.status.instance_id.clone();
                let connections = s.connections.clone();
                s.flow_names
                    .observe(&mode, &network, instance.as_deref(), &connections);
            }
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
        let before = session.extension.status.instance_id.clone();
        change(&mut session.extension, &rules);
        if before != session.extension.status.instance_id
            || !session.extension.status.signed
            || !["ready", "applied", "stale"].contains(&session.extension.status.stage.as_str())
        {
            session.asks.invalidate();
        }
        session.revision += 1;
        Ok(session.snapshot())
    }
    pub fn begin_recovery(&self, now: u64) -> Result<(u64, Vec<Rule>, Snapshot), String> {
        let mut session = self.inner.lock().map_err(|_| "state_unavailable")?;
        if session.updates.snapshot.stage == "installing"
            || !session.extension.recovery_due(&session.document.rules, now)
        {
            return Err("recovery_not_due".into());
        }
        let operation = session.extension.begin("applying")?;
        session.extension.recovering(now);
        let rules = session
            .document
            .rules
            .iter()
            .filter(|r| crate::model::enforceable(r))
            .cloned()
            .collect();
        session.revision += 1;
        Ok((operation, rules, session.snapshot()))
    }
    pub fn begin_extension(&self, stage: &str) -> Result<(u64, Vec<Rule>, Snapshot), String> {
        let mut session = self.inner.lock().map_err(|_| "state_unavailable")?;
        if session.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if stage == "activating" {
            crate::extension::policy_payload(&session.document.rules)?;
        }
        let operation = session.extension.begin(stage)?;
        if ["disabling", "removing", "activating"].contains(&stage) {
            session.temporary.clear();
        }
        session.asks.invalidate();
        if ["activating", "applying"].contains(&stage) {
            let rules = session.document.rules.clone();
            session.extension.requested(&rules);
        }
        session.revision += 1;
        Ok((
            operation,
            session
                .document
                .rules
                .iter()
                .filter(|r| crate::model::enforceable(r))
                .cloned()
                .collect(),
            session.snapshot(),
        ))
    }
    pub fn begin_ask_poll(&self) -> Result<(u64, String), String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.updates.snapshot.stage == "installing"
            || s.extension.status.busy
            || !s.extension.status.signed
            || !["ready", "applied", "stale"].contains(&s.extension.status.stage.as_str())
        {
            return Err("ask_unavailable".into());
        }
        let instance = s
            .extension
            .status
            .instance_id
            .clone()
            .ok_or("ask_unavailable")?;
        let op = s.asks.begin_poll().ok_or("ask_unavailable")?;
        Ok((op, instance))
    }
    pub fn finish_ask_poll(
        &self,
        op: u64,
        instance: &str,
        result: Result<crate::asks::Envelope, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.extension.status.instance_id.as_deref() != Some(instance)
            || !s.asks.polled(op, instance, result)
        {
            return Err("ask_stale".into());
        }
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn claim_ask(
        &self,
        id: &str,
        instance: &str,
        policy: &str,
        action: &str,
    ) -> Result<(u64, crate::asks::Answer, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.updates.snapshot.stage == "installing"
            || s.extension.status.busy
            || !s.extension.status.signed
            || !["ready", "applied", "stale"].contains(&s.extension.status.stage.as_str())
            || s.extension.status.instance_id.as_deref() != Some(instance)
        {
            return Err("ask_unavailable".into());
        }
        let (op, answer) = s.asks.claim(id, instance, policy, action)?;
        s.revision += 1;
        Ok((op, answer, s.snapshot()))
    }
    pub fn finish_ask_answer(
        &self,
        op: u64,
        answer: &crate::asks::Answer,
        result: Result<crate::asks::Reply, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if !s.asks.answered(op, answer, result) {
            return Err("ask_stale".into());
        }
        s.revision += 1;
        Ok(s.snapshot())
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
    pub fn lan_edit(
        &self,
        generation: u64,
        command: crate::inventory::Edit,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if matches!(&command, crate::inventory::Edit::Archive {}) {
            if generation != s.inventory.snapshot.generation
                || s.inventory.snapshot.status == "running"
            {
                return Err("lan_stale".into());
            }
            let job = s.discovery.clone();
            s.inventory.archive(&job, &self.path)?;
        } else {
            s.inventory.edit(generation, command)?;
        }
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn lan_copy(&self, generation: u64) -> Result<String, String> {
        let s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.inventory.snapshot.generation != generation
            || ["idle", "running", "cancelling"].contains(&s.discovery.stage.as_str())
        {
            return Err("export_stale_or_busy".into());
        }
        s.inventory.selected_csv(&s.discovery)
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
    pub fn update_change<T>(
        &self,
        change: impl FnOnce(&mut crate::updates::Machine) -> Result<T, String>,
    ) -> Result<(T, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        let old_stage = s.updates.snapshot.stage.clone();
        let value = change(&mut s.updates)?;
        if old_stage != s.updates.snapshot.stage {
            s.updates
                .record_history(&self.path.with_extension("updates.json"));
        }
        s.revision += 1;
        Ok((value, s.snapshot()))
    }
    pub fn begin_update_install(
        &self,
    ) -> Result<
        (
            u64,
            tauri_plugin_updater::Update,
            std::sync::Arc<Vec<u8>>,
            Snapshot,
        ),
        String,
    > {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.syslog.busy() || s.routers.busy() || !crate::updates::can_install(&s.snapshot()) {
            return Err("update_install_unavailable".into());
        }
        let (op, update, bytes) = s.updates.install()?;
        s.updates
            .record_history(&self.path.with_extension("updates.json"));
        s.revision += 1;
        Ok((op, update, bytes, s.snapshot()))
    }
    pub fn begin_services(&self, plan: &crate::services::Plan) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if ["running", "cancelling"].contains(&s.services.stage.as_str())
            || ["running", "cancelling"].contains(&s.ports.stage.as_str())
            || ["running", "cancelling"].contains(&s.discovery.stage.as_str())
            || s.ports.operation != plan.port_operation
            || s.ports.stage != "completed"
            || !["completed", "partial"].contains(&s.discovery.stage.as_str())
            || s.inventory.snapshot.generation != plan.tcp.generation
        {
            return Err("services_busy_or_stale".into());
        }
        s.services_operation += 1;
        s.services_cancel = false;
        s.services = plan.snapshot();
        s.services.comparison = s.services_comparator.begin(plan);
        s.services.operation = s.services_operation;
        s.services_plan = Some(plan.clone());
        s.document.preferences.view = "servers".into();
        s.revision += 1;
        Ok((s.services_operation, s.snapshot()))
    }
    pub fn services_cancelled(&self, op: u64) -> bool {
        self.inner
            .lock()
            .map(|s| {
                s.services_operation != op
                    || s.services_cancel
                    || s.inventory.snapshot.generation != s.services.generation
                    || s.ports.operation != s.services.port_operation
                    || s.discovery.stage == "failed"
            })
            .unwrap_or(true)
    }
    pub fn stop_services(&self) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.services.stage == "running" {
            s.services_cancel = true;
            s.services.stage = "cancelling".into();
            s.revision += 1;
        }
        Ok(s.snapshot())
    }
    pub fn update_services(
        &self,
        op: u64,
        mut result: crate::services::Snapshot,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.services_operation != op
            || !["running", "cancelling"].contains(&s.services.stage.as_str())
        {
            return Err("stale_services".into());
        }
        if s.services_cancel
            || s.inventory.snapshot.generation != s.services.generation
            || s.ports.operation != s.services.port_operation
            || s.discovery.stage == "failed"
        {
            if result.stage == "running" {
                return Err("cancelled".into());
            }
            result = s.services.clone();
            result.stage = "cancelled".into();
            if s.discovery.stage == "failed" {
                result.error = Some("network_changed".into());
            }
        }
        result.notes = s.services.notes.clone();
        result.operation = op;
        result.selected_port = s.services.selected_port;
        if let Some(plan) = s.services_plan.clone() {
            s.services_comparator.update(&plan, &mut result);
        }
        s.services = result;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn temporary_action(
        &self,
        action: &str,
        connection: Option<&str>,
        id: Option<&str>,
        direction: Option<&str>,
        peer: Option<&str>,
        seconds: u16,
        confirmed: bool,
        fresh: Option<crate::network::NetworkSnapshot>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        match action {
            "add" => {
                if s.updates.snapshot.stage == "installing"
                    || s.extension.status.busy
                    || !s.extension.status.signed
                    || s.document.preferences.mode != "live"
                {
                    return Err("temporary_unavailable".into());
                }
                let c = s
                    .connections
                    .iter()
                    .find(|c| Some(c.id.as_str()) == connection)
                    .cloned()
                    .ok_or("connection_expired")?;
                let verified =
                    crate::identity::for_pid(c.pid).ok_or("application_identity_unavailable")?;
                if c.identity.as_ref() != Some(&verified) {
                    return Err("application_identity_changed".into());
                }
                let network = fresh.ok_or("network_unavailable")?;
                if crate::destination_names::network_key(&network)
                    != crate::destination_names::network_key(&s.network)
                {
                    return Err("network_changed".into());
                }
                let instance = s
                    .extension
                    .status
                    .instance_id
                    .clone()
                    .ok_or("temporary_unavailable")?;
                s.temporary.add(
                    c,
                    &network,
                    &instance,
                    direction.ok_or("invalid_direction")?,
                    peer.ok_or("peer_evidence_required")?,
                    seconds,
                    confirmed,
                )?;
            }
            "remove" => s.temporary.remove(id.ok_or("grant_required")?)?,
            "clear" => s.temporary.clear(),
            _ => return Err("invalid_temporary_action".into()),
        };
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_temporary_sync(
        &self,
    ) -> Result<Option<crate::temporary_protection::Ticket>, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        let network = s.network.clone();
        let instance = if s.extension.status.signed {
            s.extension.status.instance_id.clone()
        } else {
            None
        };
        let connections = s.connections.clone();
        let live = s.document.preferences.mode == "live"
            && ![
                "disabled",
                "removed",
                "disabling",
                "removing",
                "activating",
                "restart_required",
                "approval_required",
            ]
            .contains(&s.extension.status.stage.as_str());
        s.temporary
            .observe(&network, instance.as_deref(), &connections, live);
        if s.updates.snapshot.stage == "installing" {
            return Ok(None);
        }
        Ok(s.temporary.begin())
    }
    pub fn finish_temporary_sync(
        &self,
        t: &crate::temporary_protection::Ticket,
        result: Result<crate::temporary_protection::Reply, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.temporary.finish(t, result);
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn configure_flow_names(
        &self,
        enabled: bool,
        search: bool,
        operation: u64,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if enabled && s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        let mode = s.document.preferences.mode.clone();
        let network = s.network.clone();
        let instance = if s.extension.status.signed {
            s.extension.status.instance_id.clone()
        } else {
            None
        };
        s.flow_names.configure(
            enabled,
            search,
            operation,
            &mode,
            &network,
            instance.as_deref(),
        )?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_flow_names(
        &self,
        operation: u64,
    ) -> Result<(crate::flow_names::Ticket, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        let mode = s.document.preferences.mode.clone();
        let network = s.network.clone();
        let instance = s.extension.status.instance_id.clone();
        let ticket = s
            .flow_names
            .begin(operation, &mode, &network, instance.as_deref())?;
        s.revision += 1;
        Ok((ticket, s.snapshot()))
    }
    pub fn finish_flow_names(
        &self,
        ticket: &crate::flow_names::Ticket,
        result: Result<crate::flow_names::Reply, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        let mode = s.document.preferences.mode.clone();
        let network = s.network.clone();
        let instance = s.extension.status.instance_id.clone();
        let connections = s.connections.clone();
        s.flow_names.finish(
            ticket,
            result,
            &mode,
            &network,
            instance.as_deref(),
            &connections,
        );
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn diagnostic_export(&self, report: u64, format: &str) -> Result<String, String> {
        let s = self.inner.lock().map_err(|_| "state_lock")?;
        crate::diagnostics::export_locale(
            s.diagnostics.report(report)?,
            format,
            &s.document.preferences.locale,
        )
    }
    pub fn diagnostic_action(
        &self,
        op: u64,
        report: u64,
        finding: Option<String>,
        action: String,
        disposition: Option<crate::diagnostics::Disposition>,
        reason: Option<String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        match action.as_str() {
            "assess" => {
                if op != s.services.operation
                    || s.services.generation != s.inventory.snapshot.generation
                    || s.services.port_operation != s.ports.operation
                    || s.ports.stage != "completed"
                {
                    return Err("stale_services".into());
                }
                let job = s.services.clone();
                s.diagnostics.assess(&job, now())?;
            }
            "select" => s.diagnostics.select(report, finding)?,
            "disposition" => s.diagnostics.disposition(
                report,
                finding.as_deref().ok_or("finding_required")?,
                disposition.ok_or("disposition_required")?,
                reason.as_deref().unwrap_or(""),
            )?,
            "services" | "network" => {
                let r = s.diagnostics.report(report)?.clone();
                if r.generation != s.inventory.snapshot.generation
                    || r.service_operation != s.services.operation
                    || s.services.port_operation != s.ports.operation
                    || s.ports.stage != "completed"
                    || s.services.error.is_some()
                {
                    return Err("stale_diagnostic_target".into());
                }
                let port = if let Some(id) = finding.as_ref() {
                    Some(
                        r.findings
                            .iter()
                            .find(|f| &f.id == id)
                            .ok_or("stale_finding")?
                            .port,
                    )
                } else {
                    None
                };
                let device = s
                    .inventory
                    .snapshot
                    .rows
                    .iter()
                    .find(|d| {
                        d.device.address == r.target
                            && d.device.interface == r.interface
                            && d.kind != "not_seen"
                    })
                    .map(|d| d.device.id.clone());
                if action == "network" && device.is_none() {
                    return Err("stale_diagnostic_target".into());
                }
                let old = s.document.clone();
                s.document.preferences.view = if action == "services" {
                    "servers"
                } else {
                    "network"
                }
                .into();
                if let Err(e) = self.persist(&s.document) {
                    s.document = old;
                    return Err(e);
                }
                if action == "services" {
                    s.services.selected_port = port;
                } else {
                    s.inventory.snapshot.selected_id = device;
                    s.ports_focus += 1;
                }
            }
            "rule" => {
                let r = s.diagnostics.report(report)?;
                if r.generation != s.inventory.snapshot.generation
                    || r.service_operation != s.services.operation
                    || s.services.port_operation != s.ports.operation
                    || s.ports.stage != "completed"
                    || s.services.error.is_some()
                    || s.document.preferences.mode != "live"
                {
                    return Err("stale_diagnostic_target".into());
                }
                let f = r
                    .findings
                    .iter()
                    .find(|f| Some(&f.id) == finding.as_ref())
                    .ok_or("stale_finding")?;
                let candidates: Vec<_> = s
                    .connections
                    .iter()
                    .filter(|c| {
                        c.destination == r.target
                            && c.port == f.port
                            && c.protocol == "TCP"
                            && c.identity.as_ref().is_some_and(|i| i.valid())
                    })
                    .collect();
                if candidates.len() != 1 {
                    return Err("ambiguous_or_unobserved_application".into());
                }
                let selected = candidates[0].id.clone();
                let old = s.document.clone();
                s.document.preferences.view = "map".into();
                if let Err(e) = self.persist(&s.document) {
                    s.document = old;
                    return Err(e);
                }
                s.selected_id = Some(selected);
            }
            _ => return Err("invalid_diagnostic_action".into()),
        }
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn services_csv(&self, op: u64) -> Result<String, String> {
        let s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.services.operation != op
            || ["running", "cancelling", ""].contains(&s.services.stage.as_str())
            || s.services.rows.is_empty()
        {
            return Err("stale_or_unfinished_services".into());
        }
        Ok(crate::services::csv(
            &s.services,
            s.inventory.snapshot.generation,
            s.ports.operation,
        ))
    }
    pub fn port_panel(&self, panel: crate::port_layout::Panel) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.ports.panel = panel;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn port_panel_closed(&self) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.ports.panel == crate::port_layout::Panel::Floating {
            s.ports.panel = crate::port_layout::Panel::Bottom;
            s.revision += 1;
        }
        Ok(s.snapshot())
    }
    pub fn set_port_layout(
        &self,
        expected: &crate::port_layout::Layout,
        value: crate::port_layout::Layout,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        s.ports.layout.replace(expected, value)?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn select_port(&self, operation: u64, port: u16) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.ports.operation != operation || !s.ports.rows.iter().any(|r| r.port == port) {
            return Err("stale_port".into());
        }
        s.ports.selected_port = Some(port);
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn edit_service_note(
        &self,
        operation: u64,
        command: crate::service_notes::Edit,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if s.services.operation != operation
            || !["completed", "cancelled", "failed"].contains(&s.services.stage.as_str())
            || !s.services.rows.iter().any(|r| r.port == command.port)
        {
            return Err("stale_or_running_service_note".into());
        }
        crate::service_notes::edit(&mut s.services.notes, command, now())?;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn select_service(&self, op: u64, port: u16) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.services.operation != op || !s.services.rows.iter().any(|r| r.port == port) {
            return Err("stale_service".into());
        }
        s.services.selected_port = Some(port);
        s.revision += 1;
        Ok(s.snapshot())
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
            || !s
                .discovery
                .devices
                .iter()
                .any(|d| d.id == id && crate::ports::eligible(d))
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
        plan.settings.validate(plan.ports.len())?;
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if ["running", "cancelling"].contains(&s.ports.stage.as_str())
            || ["running", "cancelling"].contains(&s.discovery.stage.as_str())
            || s.inventory.snapshot.generation != plan.generation
        {
            return Err("ports_busy_or_stale".into());
        }
        if ["running", "cancelling"].contains(&s.services.stage.as_str()) {
            return Err("services_busy".into());
        }
        s.ports_operation += 1;
        s.ports_cancel = false;
        let layout = s.ports.layout.clone();
        let panel = s.ports.panel;
        s.ports = plan.snapshot();
        s.ports.panel = panel;
        s.ports.layout = layout;
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
        result.panel = s.ports.panel;
        result.layout = s.ports.layout.clone();
        result.operation = op;
        if let Some(plan) = s.ports_plan.clone() {
            s.ports_comparator.update(&plan, &mut result);
        }
        result.selected_port = s
            .ports
            .selected_port
            .filter(|port| result.rows.iter().any(|r| r.port == *port));
        s.ports = result;
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn get_lan_archive(
        &self,
        generation: u64,
        observed_at: u64,
    ) -> Result<crate::inventory::Archive, String> {
        self.inner
            .lock()
            .map_err(|_| "state_lock")?
            .inventory
            .get_archive(generation, observed_at)
    }
    pub fn configure_lan_public(&self, enabled: bool, expected: u64) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        s.inventory.public.configure(enabled, expected)?;
        s.inventory.snapshot.public_info = s.inventory.public.snapshot.clone();
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_lan_public(&self, expected: u64) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        let op = s.inventory.public.begin(expected)?;
        s.inventory.snapshot.public_info = s.inventory.public.snapshot.clone();
        s.revision += 1;
        Ok((op, s.snapshot()))
    }
    pub fn finish_lan_public(
        &self,
        op: u64,
        result: Result<crate::public_network::Snapshot, String>,
    ) -> Result<Snapshot, String> {
        let mut s = self.inner.lock().map_err(|_| "state_lock")?;
        s.inventory.public.finish(op, result);
        s.inventory.snapshot.public_info = s.inventory.public.snapshot.clone();
        s.revision += 1;
        Ok(s.snapshot())
    }
    pub fn begin_discovery(
        &self,
        plan: &crate::discovery::Plan,
    ) -> Result<(u64, Snapshot), String> {
        let mut s = self.inner.lock().map_err(|_| "state_unavailable")?;
        if s.updates.snapshot.stage == "installing" {
            return Err("update_installing".into());
        }
        if ["running", "cancelling"].contains(&s.discovery.stage.as_str()) {
            return Err("discovery_busy".into());
        }
        if ["running", "cancelling"].contains(&s.ports.stage.as_str()) {
            return Err("ports_busy".into());
        }
        if ["running", "cancelling"].contains(&s.services.stage.as_str()) {
            return Err("services_busy".into());
        }
        s.ports_comparator.invalidate();
        s.services_comparator.invalidate();
        s.services.comparison = crate::service_comparison::Comparison {
            status: "invalidated".into(),
            ..Default::default()
        };
        s.discovery_operation += 1;
        s.discovery_cancel = false;
        let generation = s.discovery_operation;
        s.inventory.begin(plan, generation);
        s.discovery_plan = Some(plan.clone());
        s.discovery = crate::discovery::DiscoverySnapshot {
            context: plan.context.clone(),
            options: plan.options.clone(),
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
        let retention = s.document.preferences.history_retention;
        retention.prune(&mut s.history, now());
        retention.prune(&mut s.demo_history, now());
        s.firewall = firewall;
        s.destination_names.observe(&network);
        if s.firewall.error.is_some() {
            s.destination_names.invalidate();
            {
                let mode = s.document.preferences.mode.clone();
                let network = s.network.clone();
                let instance = s.extension.status.instance_id.clone();
                let connections = s.connections.clone();
                s.flow_names
                    .observe(&mode, &network, instance.as_deref(), &connections);
            }
        }
        s.network = network;
        {
            let mode = s.document.preferences.mode.clone();
            let network = s.network.clone();
            let instance = if s.firewall.error.is_none() {
                s.extension.status.instance_id.clone()
            } else {
                None
            };
            let connections = s.connections.clone();
            s.flow_names
                .observe(&mode, &network, instance.as_deref(), &connections);
        }
        if s.services_plan
            .as_ref()
            .is_some_and(|p| !p.tcp.matches(&s.network))
        {
            s.services_comparator.invalidate();
            s.services.comparison = crate::service_comparison::Comparison {
                status: "invalidated".into(),
                ..Default::default()
            };
            if ["completed", "cancelled"].contains(&s.services.stage.as_str()) {
                s.services.stage = "failed".into();
                s.services.error = Some("network_changed".into());
            }
        }
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
            s.services_comparator.invalidate();
            s.services.comparison = crate::service_comparison::Comparison {
                status: "invalidated".into(),
                ..Default::default()
            };
            if ["completed", "cancelled"].contains(&s.services.stage.as_str()) {
                s.services.stage = "failed".into();
                s.services.error = Some("network_changed".into());
            }
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
    pub fn flow_name_snapshot(&self) -> crate::flow_names::Snapshot {
        self.flow_names.snapshot(
            &self.document.preferences.mode,
            &self.network,
            if self.extension.status.signed && self.firewall.error.is_none() {
                self.extension.status.instance_id.as_deref()
            } else {
                None
            },
        )
    }

    pub fn snapshot_for_gpu(&self) -> Snapshot {
        self.snapshot()
    }
    fn security_snapshot(&self) -> crate::security::SecuritySnapshot {
        let demo = self.document.preferences.mode == "demo";
        let base = crate::security::normalize(&self.visible(), demo, self.last_observed);
        if demo {
            base
        } else {
            self.routers.augment(base, now())
        }
    }

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
            gpu_panel_owner: self.gpu_owner.clone(),
            gpu_float_operation: self.gpu_float_operation,
            gpu_dock_operation: self.gpu_dock_operation,
            temporary_protection: self.temporary.snapshot(),
            flow_names: self.flow_name_snapshot(),
            gpu_filter: self.gpu_filter.clone(),
            diagnostics: self.diagnostics.for_ui(),
            destination_names: self.destination_names.snapshot(),
            firewall_control: self.firewall_control.snapshot.clone(),
            syslog: self.syslog.snapshot.clone(),
            asks: self.asks.snapshot.clone(),
            security: self.security_snapshot(),
            updates: self.updates.snapshot.clone(),
            services: self.services.clone(),
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
                self.document
                    .preferences
                    .history_retention
                    .visible(&self.demo_history, now())
            } else {
                self.document
                    .preferences
                    .history_retention
                    .visible(&self.history, now())
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
    fn ask_poll_does_not_revoke_an_explicit_temporary_session() {
        let state = state();
        {
            let mut s = state.inner.lock().unwrap();
            let mut c = s.demo[0].clone();
            c.local = "192.0.2.2:50000".into();
            c.destination = "192.0.2.1".into();
            c.identity = Some(crate::identity::SigningIdentity {
                authority: "developer".into(),
                team_id: "TESTTEAM01".into(),
                identifier: "com.example.test".into(),
            });
            let network = crate::network::NetworkSnapshot {
                addresses: vec![crate::network::InterfaceAddress {
                    interface: "test0".into(),
                    address: "192.0.2.2".into(),
                    family: "IPv4".into(),
                    netmask: "255.255.255.0".into(),
                    up: true,
                }],
                ..Default::default()
            };
            s.temporary
                .add(
                    c,
                    &network,
                    "boot",
                    "outbound",
                    "tls:example.test",
                    60,
                    true,
                )
                .unwrap();
            s.extension.status.signed = true;
            s.extension.status.busy = false;
            s.extension.status.stage = "ready".into();
            s.extension.status.instance_id = Some("boot".into());
        }
        assert!(state.begin_ask_poll().is_ok());
        assert_eq!(
            state.inner.lock().unwrap().temporary.snapshot().rows.len(),
            1
        );
    }
    #[test]
    fn diagnostic_rule_navigation_requires_one_current_signed_connection_and_never_saves_rules() {
        let state = state();
        let identity = crate::identity::SigningIdentity {
            authority: "developer".into(),
            team_id: "TESTTEAM01".into(),
            identifier: "com.example.diag".into(),
        };
        {
            let mut s = state.inner.lock().unwrap();
            let mut c = s.demo[0].clone();
            c.identity = Some(identity);
            c.destination = "192.0.2.1".into();
            c.port = 443;
            s.connections = vec![c];
            s.document.preferences.mode = "live".into();
            s.document.preferences.view = "diagnostics".into();
            s.inventory.snapshot.generation = 7;
            s.services.operation = 42;
            s.ports.stage = "completed".into();
            s.diagnostics.reports.push(crate::diagnostics::Report {
                id: 1,
                service_operation: 42,
                generation: 7,
                target: "192.0.2.1".into(),
                interface: "en0".into(),
                created_at: 1,
                findings: vec![crate::diagnostics::Finding {
                    id: "443:tls".into(),
                    port: 443,
                    category: "tls".into(),
                    severity: "warning".into(),
                    certainty: "unverified".into(),
                    evidence: "test".into(),
                    recommendation: "review_tls".into(),
                    observed_at: 1,
                    change: "new".into(),
                    disposition: Default::default(),
                    reason: String::new(),
                }],
            });
        }
        let before = state.snapshot().unwrap();
        let next = state
            .diagnostic_action(0, 1, Some("443:tls".into()), "rule".into(), None, None)
            .unwrap();
        assert_eq!(next.document.preferences.view, "map");
        assert_eq!(next.selected_id, Some(next.connections[0].id.clone()));
        assert_eq!(next.document.rules, before.document.rules);
        assert_eq!(
            serde_json::to_value(next.extension).unwrap(),
            serde_json::to_value(before.extension).unwrap()
        );
        {
            let mut s = state.inner.lock().unwrap();
            let mut duplicate = s.connections[0].clone();
            duplicate.id = "another".into();
            s.connections.push(duplicate);
        }
        assert!(state
            .diagnostic_action(0, 1, Some("443:tls".into()), "rule".into(), None, None)
            .is_err());
        state.inner.lock().unwrap().inventory.snapshot.generation = 8;
        assert!(state
            .diagnostic_action(0, 1, None, "services".into(), None, None)
            .is_err());
    }
    #[test]
    fn history_retention_migrates_persists_and_never_persists_rows() {
        use crate::history::Retention;
        let legacy: Preferences =
            serde_json::from_str(r#"{"locale":"ja","view":"map","panel":"right","mode":"live"}"#)
                .unwrap();
        assert_eq!(legacy.history_retention, Retention::Session);
        let state = state();
        {
            let mut s = state.inner.lock().unwrap();
            s.history = s.demo.clone();
        }
        state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Hour1,
            })
            .unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&state.path).unwrap()).unwrap();
        assert_eq!(saved["preferences"]["historyRetention"], "1h");
        assert!(saved.get("history").is_none());
        let path = state.path.clone();
        drop(state);
        let restarted = SharedState::load(path).unwrap().snapshot().unwrap();
        assert_eq!(
            restarted.document.preferences.history_retention,
            Retention::Hour1
        );
        assert!(restarted.history.is_empty());
    }
    #[test]
    fn history_pruning_is_atomic_and_does_not_change_active_connections_or_policy() {
        use crate::history::Retention;
        let mut state = state();
        {
            let mut s = state.inner.lock().unwrap();
            s.connections = s.demo.clone();
            s.history = s.demo.clone();
            s.history[0].observed_at = now() - 901;
            s.selected_id = Some(s.connections[0].id.clone());
        }
        let before = state.snapshot().unwrap();
        let next = state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Minutes15,
            })
            .unwrap();
        assert_eq!(next.history.len(), before.history.len() - 1);
        assert_eq!(
            serde_json::to_value(&next.connections).unwrap(),
            serde_json::to_value(&before.connections).unwrap()
        );
        assert_eq!(next.selected_id, before.selected_id);
        assert_eq!(
            serde_json::to_value(&next.extension).unwrap(),
            serde_json::to_value(&before.extension).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&next.document.rules).unwrap(),
            serde_json::to_value(&before.document.rules).unwrap()
        );
        state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Session,
            })
            .unwrap();
        assert_eq!(state.snapshot().unwrap().history.len(), next.history.len());
        state.path = PathBuf::from("/dev/null/document.json");
        assert!(state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Off
            })
            .is_err());
        let failed = state.snapshot().unwrap();
        assert_eq!(
            failed.document.preferences.history_retention,
            Retention::Session
        );
        assert_eq!(failed.history.len(), next.history.len());
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert_eq!(
            state
                .dispatch(Command::SetHistoryRetention {
                    retention: Retention::Off
                })
                .err()
                .unwrap(),
            "update_installing"
        );
        assert_eq!(state.snapshot().unwrap().history.len(), next.history.len());
    }
    #[test]
    fn observer_expires_history_even_on_failure_and_keeps_first_observation_time() {
        use crate::history::Retention;
        let state = state();
        let (mut current, mut firewall) = {
            let mut s = state.inner.lock().unwrap();
            s.document.preferences.history_retention = Retention::Minutes15;
            let mut row = s.demo[0].clone();
            row.observed_at = now() - 901;
            s.history = vec![row.clone()];
            s.connections = vec![row.clone()];
            (vec![row], s.firewall.clone())
        };
        current[0].observed_at = now();
        let next = state
            .observed(current.clone(), firewall.clone(), Default::default())
            .unwrap();
        assert!(next.history.is_empty());
        assert!(next.connections[0].observed_at < now() - 900);
        {
            let mut s = state.inner.lock().unwrap();
            s.history = vec![s.connections[0].clone()];
        }
        firewall.error = Some("observer_failed".into());
        assert!(state
            .observed(vec![], firewall, Default::default())
            .unwrap()
            .history
            .is_empty());
        assert!(state.inner.lock().unwrap().history.is_empty());
    }
    #[test]
    fn disabled_history_continues_observation_and_reenabling_records_only_new_connections() {
        use crate::history::Retention;
        let state = state();
        let base = state.snapshot().unwrap();
        let current = state.inner.lock().unwrap().demo.clone();
        state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Off,
            })
            .unwrap();
        let off = state
            .observed(current.clone(), base.firewall.clone(), Default::default())
            .unwrap();
        assert_eq!(off.connections.len(), 4);
        assert!(off.history.is_empty());
        state
            .dispatch(Command::SetHistoryRetention {
                retention: Retention::Session,
            })
            .unwrap();
        assert!(state
            .observed(current.clone(), base.firewall.clone(), Default::default())
            .unwrap()
            .history
            .is_empty());
        let mut updated = current;
        updated[0].id = "new-connection".into();
        updated[0].observed_at = now();
        let next = state
            .observed(updated, base.firewall, Default::default())
            .unwrap();
        assert_eq!(next.history.len(), 1);
        assert_eq!(next.history[0].id, "new-connection");
        state
            .dispatch(Command::SetMode {
                mode: "demo".into(),
            })
            .unwrap();
        assert!(state.snapshot().unwrap().history.is_empty()); // sample history also stays cleared
        assert_eq!(state.snapshot().unwrap().connections.len(), 4);
        state
            .dispatch(Command::SetMode {
                mode: "live".into(),
            })
            .unwrap();
        assert_eq!(state.snapshot().unwrap().history.len(), 1);
    }
    #[test]
    fn app_icons_accept_only_observed_paths_or_verified_subject_keys() {
        let state = state();
        let key = crate::identity::SigningIdentity {
            authority: "developer".into(),
            team_id: "TESTTEAM01".into(),
            identifier: "com.example.icons".into(),
        };
        {
            let mut s = state.inner.lock().unwrap();
            let mut c = s.demo[0].clone();
            c.app = "Misleading Safari Name".into();
            c.app_key = "/Applications/Actual.app/Contents/MacOS/Actual".into();
            c.identity = Some(key.clone());
            s.connections = vec![c];
            s.document.preferences.mode = "live".into();
        }
        let path = std::path::PathBuf::from("/Applications/Actual.app/Contents/MacOS/Actual");
        assert_eq!(state.icon_subject(&key.key()), Some(path.clone()));
        assert_eq!(
            state.icon_subject(path.to_str().unwrap()),
            Some(path.clone())
        );
        assert!(state.icon_subject("Misleading Safari Name").is_none());
        assert!(state.icon_subject("/etc/passwd").is_none());
        {
            let mut s = state.inner.lock().unwrap();
            s.history = s.connections.clone();
            s.connections.clear();
        }
        assert_eq!(state.icon_subject(&key.key()), Some(path));
        state.inner.lock().unwrap().document.preferences.mode = "demo".into();
        assert!(state.icon_subject(&key.key()).is_none());
    }
    #[test]
    fn icon_queries_do_not_mutate_documents_rules_or_protection() {
        let state = state();
        let before = state.snapshot().unwrap();
        assert!(state
            .icon_subject("/Applications/Unobserved.app/Contents/MacOS/test")
            .is_none());
        let after = state.snapshot().unwrap();
        assert_eq!(before.revision, after.revision);
        assert_eq!(
            serde_json::to_value(before.document).unwrap(),
            serde_json::to_value(after.document).unwrap()
        );
        assert_eq!(
            serde_json::to_value(before.extension).unwrap(),
            serde_json::to_value(after.extension).unwrap()
        );
        assert!(!state.path.exists());
    }
    #[test]
    fn destination_lookup_requires_current_live_connection_and_session_consent() {
        let state = state();
        let network = crate::destination_names::tests::network();
        {
            let mut s = state.inner.lock().unwrap();
            s.document.preferences.mode = "live".into();
            let mut c = s.demo[0].clone();
            c.destination = "192.0.2.1".into();
            s.connections = vec![c];
            s.network = network.clone();
            s.firewall.error = None;
            s.updates.snapshot.stage = "ready".into();
            s.updates.snapshot.development = false;
        }
        let before = state.snapshot().unwrap();
        let id = before.connections[0].id.clone();
        assert!(crate::updates::can_install(&before));
        assert!(state
            .begin_destination_name(&id, "192.0.2.1", 0, &network)
            .is_err());
        let configured = state
            .configure_destination_names(true, 0, Some(network.clone()))
            .unwrap();
        assert!(!configured.destination_names.busy);
        assert!(configured.destination_names.rows.is_empty());
        let op = configured.destination_names.operation;
        assert!(state
            .begin_destination_name(&id, "192.0.2.2", op, &network)
            .is_err());
        let (ticket, started) = state
            .begin_destination_name(&id, "192.0.2.1", op, &network)
            .unwrap();
        assert!(!crate::updates::can_install(&started));
        let revoked = state.configure_destination_names(false, 0, None).unwrap();
        assert!(!revoked.destination_names.enabled);
        assert!(revoked.destination_names.cancelling);
        assert!(ticket.cancel.load(std::sync::atomic::Ordering::SeqCst));
        assert!(state
            .configure_destination_names(true, op, Some(network.clone()))
            .is_err());
        state
            .configure_destination_names(
                true,
                revoked.destination_names.operation,
                Some(network.clone()),
            )
            .unwrap();
        let finished = state
            .finish_destination_name(&ticket, Some("late.example".into()), "resolved", &network)
            .unwrap();
        assert!(finished.destination_names.rows.is_empty());
        assert!(!finished.destination_names.busy);
        assert_eq!(finished.document.rules, before.document.rules);
        assert!(!state.path.exists());
        let old = finished.destination_names.operation;
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state
            .configure_destination_names(true, old, Some(network))
            .is_err());
        assert!(state.configure_destination_names(false, old, None).is_ok());
    }
    #[test]
    fn destination_lookup_results_are_cleared_by_mode_changes_and_observer_failure() {
        let state = state();
        let network = crate::destination_names::tests::network();
        {
            let mut s = state.inner.lock().unwrap();
            s.document.preferences.mode = "live".into();
            let mut c = s.demo[0].clone();
            c.destination = "192.0.2.1".into();
            s.connections = vec![c];
            s.network = network.clone();
            s.firewall.error = None;
        }
        let configured = state
            .configure_destination_names(true, 0, Some(network.clone()))
            .unwrap();
        let id = configured.connections[0].id.clone();
        let (ticket, _) = state
            .begin_destination_name(
                &id,
                "192.0.2.1",
                configured.destination_names.operation,
                &network,
            )
            .unwrap();
        let named = state
            .finish_destination_name(&ticket, Some("host.example".into()), "resolved", &network)
            .unwrap();
        assert_eq!(named.destination_names.rows.len(), 1);
        let demo = state
            .dispatch(Command::SetMode {
                mode: "demo".into(),
            })
            .unwrap();
        assert!(!demo.destination_names.enabled);
        assert!(demo.destination_names.rows.is_empty());
        state
            .dispatch(Command::SetMode {
                mode: "live".into(),
            })
            .unwrap();
        let op = state.snapshot().unwrap().destination_names.operation;
        state
            .configure_destination_names(true, op, Some(network.clone()))
            .unwrap();
        let mut firewall = state.snapshot().unwrap().firewall;
        firewall.error = Some("controlled observer failure".into());
        let failed = state.observed(vec![], firewall, network).unwrap();
        assert!(!failed.destination_names.enabled);
    }
    #[test]
    fn protection_guide_acknowledgement_persists_without_changing_protection() {
        let state = state();
        let path = state.path.clone();
        {
            let mut session = state.inner.lock().unwrap();
            let identity = crate::identity::SigningIdentity {
                authority: "developer".into(),
                team_id: "TESTTEAM01".into(),
                identifier: "com.example.guide".into(),
            };
            session.document.rules.push(Rule {
                transport: "TCP".into(),
                system_exception: false,
                timeout_action: "allow".into(),
                app_key: identity.key(),
                identity: Some(identity),
                id: "guide-kept-rule".into(),
                app: "Guide test".into(),
                destination: "192.0.2.1".into(),
                port: 443,
                direction: "outbound".into(),
                action: "block".into(),
                updated_at: 1,
                source: "live".into(),
            });
            let rules = session.document.rules.clone();
            session.extension.configure(true, true, true);
            let op = session.extension.begin("syncing").unwrap();
            session
                .extension
                .accepted(op, "guide-test-provider".into(), 1, rules.clone(), &rules);
        }
        let before = state.snapshot().unwrap();
        assert_eq!(before.extension.stage, "applied");
        assert!(!before.document.preferences.protection_guide_seen);
        let after = state.dispatch(Command::AcknowledgeProtectionGuide).unwrap();
        assert!(after.document.preferences.protection_guide_seen);
        assert_eq!(before.document.rules, after.document.rules);
        assert_eq!(
            serde_json::to_value(before.extension).unwrap(),
            serde_json::to_value(after.extension).unwrap()
        );
        assert_eq!(
            serde_json::to_value(before.firewall).unwrap(),
            serde_json::to_value(after.firewall).unwrap()
        );
        drop(state);
        let reopened = SharedState::load(path).unwrap();
        assert!(
            reopened
                .snapshot()
                .unwrap()
                .document
                .preferences
                .protection_guide_seen
        );
    }
    #[test]
    fn protection_guide_legacy_preference_is_unseen_and_type_checked() {
        let mut document = serde_json::to_value(Document::default()).unwrap();
        document["preferences"]
            .as_object_mut()
            .unwrap()
            .remove("protectionGuideSeen");
        let legacy: Document = serde_json::from_value(document.clone()).unwrap();
        assert!(!legacy.preferences.protection_guide_seen);
        document["preferences"]["protectionGuideSeen"] = serde_json::json!("true");
        assert!(serde_json::from_value::<Document>(document).is_err());
    }
    #[test]
    fn protection_guide_failed_save_and_installation_do_not_acknowledge() {
        let state = state();
        let before = state.snapshot().unwrap().revision;
        fs::create_dir(&state.path).unwrap();
        assert!(state.dispatch(Command::AcknowledgeProtectionGuide).is_err());
        assert!(
            !state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .protection_guide_seen
        );
        assert_eq!(state.snapshot().unwrap().revision, before);
        fs::remove_dir(&state.path).unwrap();
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state.dispatch(Command::AcknowledgeProtectionGuide).is_err());
        assert!(
            !state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .protection_guide_seen
        );
    }
    #[test]
    fn firewall_control_is_shared_ephemeral_and_excludes_installation() {
        let state = state();
        {
            let mut s = state.inner.lock().unwrap();
            s.updates.snapshot.stage = "ready".into();
            s.updates.snapshot.development = false;
        }
        assert!(crate::updates::can_install(&state.snapshot().unwrap()));
        let before = std::fs::read(&state.path).ok();
        let (op, snap) = state
            .begin_firewall_control(crate::firewall_control::Action::Status, None, false)
            .unwrap();
        assert!(snap.firewall_control.busy);
        assert!(state.snapshot().unwrap().firewall_control.busy);
        assert!(!crate::updates::can_install(&snap));
        assert!(state
            .begin_firewall_control(crate::firewall_control::Action::Register, None, true)
            .is_err());
        state
            .finish_firewall_control(op, Err("timeout".into()))
            .unwrap();
        assert!(!state.snapshot().unwrap().firewall_control.busy);
        assert_eq!(std::fs::read(&state.path).ok(), before);
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state
            .begin_firewall_control(crate::firewall_control::Action::Status, None, false)
            .is_err());
    }
    #[test]
    fn syslog_job_is_shared_ephemeral_and_update_install_gated() {
        let state = state();
        let (ticket, started) = state
            .begin_syslog(
                crate::syslog::tests::config(),
                &crate::syslog::tests::network(),
            )
            .unwrap();
        assert_eq!(started.syslog.stage, "starting");
        assert_eq!(state.snapshot().unwrap().syslog.operation, ticket.operation);
        assert!(!state.path.exists());
        state.stop_syslog(ticket.operation).unwrap();
        let before = state.snapshot().unwrap();
        let late = crate::syslog::Snapshot {
            operation: ticket.operation,
            stage: "listening".into(),
            ..Default::default()
        };
        assert!(state.update_syslog(late).is_err());
        assert_eq!(state.snapshot().unwrap().revision, before.revision);
        state
            .update_syslog(crate::syslog::Snapshot {
                operation: ticket.operation,
                stage: "stopped".into(),
                ..Default::default()
            })
            .unwrap();
        state.clear_syslog(ticket.operation).unwrap();
        assert!(state.snapshot().unwrap().syslog.rows.is_empty());
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state
            .begin_syslog(
                crate::syslog::tests::config(),
                &crate::syslog::tests::network()
            )
            .is_err());
        assert!(!state.path.exists());
    }
    #[test]
    fn router_results_are_shared_ephemeral_and_isolated_from_demo() {
        let _guard = crate::routers::tests::READ_LOCK.lock().unwrap();
        let state = state();
        state
            .register_router(crate::routers::tests::source())
            .unwrap();
        {
            state.inner.lock().unwrap().document.preferences.mode = "live".into();
        }
        let snapshot = state
            .collect_router(
                "home",
                Box::new(crate::routers::tests::Fixture { at: now() }),
                std::time::Duration::from_secs(1),
            )
            .unwrap();
        assert_eq!(snapshot.security.flows.len(), 1);
        assert_eq!(
            snapshot.security.flows[0].source,
            crate::security::ObservationSource::Router
        );
        assert_eq!(
            state.snapshot().unwrap().security.router_sources[0].stage,
            "ready"
        );
        assert!(snapshot.document.rules.is_empty());
        assert!(!state.path.exists());
        state.inner.lock().unwrap().document.preferences.mode = "demo".into();
        let demo = state.snapshot().unwrap();
        assert!(demo.security.router_sources.is_empty());
        assert!(demo
            .security
            .flows
            .iter()
            .all(|flow| flow.source == crate::security::ObservationSource::Demo));
        state.remove_router("home").unwrap();
        state.inner.lock().unwrap().document.preferences.mode = "live".into();
        assert!(state.snapshot().unwrap().security.router_sources.is_empty());
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state
            .register_router(crate::routers::tests::source())
            .is_err());
        assert!(state
            .collect_router(
                "home",
                Box::new(crate::routers::tests::Fixture { at: now() }),
                std::time::Duration::from_secs(1)
            )
            .is_err());
    }
    #[test]
    fn router_stop_preempts_collection_without_locking_the_shared_state() {
        use crate::routers::*;
        let _guard = crate::routers::tests::READ_LOCK.lock().unwrap();
        let state = std::sync::Arc::new(state());
        state
            .register_router(crate::routers::tests::source())
            .unwrap();
        struct Waiting(std::sync::mpsc::Sender<()>);
        impl RouterAdapter for Waiting {
            fn collect(&mut self, _: &Source, budget: &Budget) -> Result<ReadOutput, Error> {
                self.0.send(()).unwrap();
                loop {
                    budget.check()?;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = state.clone();
        let reader = std::thread::spawn(move || {
            worker.collect_router(
                "home",
                Box::new(Waiting(tx)),
                std::time::Duration::from_secs(1),
            )
        });
        rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
        let stopped = state.stop_router("home").unwrap();
        let revision = stopped.revision;
        assert_eq!(reader.join().unwrap().err().unwrap().code, Code::Stale);
        assert_eq!(state.snapshot().unwrap().revision, revision);
    }
    #[test]
    fn notification_opt_in_persists_and_rolls_back_on_failure() {
        let state = state();
        assert!(
            !state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .ask_notifications
        );
        assert!(
            !state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .unknown_notifications
        );
        state
            .dispatch(Command::SetUnknownNotifications { enabled: true })
            .unwrap();
        let enabled = state
            .dispatch(Command::SetAskNotifications { enabled: true })
            .unwrap();
        assert!(enabled.document.preferences.ask_notifications);
        let stored: Document = serde_json::from_slice(&fs::read(&state.path).unwrap()).unwrap();
        assert!(stored.preferences.ask_notifications);
        assert!(stored.preferences.unknown_notifications);
        fs::remove_file(&state.path).unwrap();
        fs::create_dir(&state.path).unwrap();
        assert!(state
            .dispatch(Command::SetAskNotifications { enabled: false })
            .is_err());
        assert!(
            state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .ask_notifications
        );
        fs::remove_dir(&state.path).unwrap();
    }
    #[test]
    fn protection_exceptions_require_review_conflict_guard_and_persist() {
        let state = state();
        let rules: Vec<Rule> = serde_json::from_str(r#"[{"id":"protected","appKey":"signed:v1:apple::com.apple.mDNSResponder","identity":{"authority":"apple","teamId":"","identifier":"com.apple.mDNSResponder"},"app":"DNS","destination":"1.1.1.1","port":53,"direction":"both","action":"block","updatedAt":1,"source":"live","systemException":true}]"#).unwrap();
        let snapshot = state.dispatch(Command::RestoreRules { rules }).unwrap();
        assert!(!snapshot.document.rules[0].system_exception);
        let command = || Command::SetSystemException {
            id: "protected".into(),
            enabled: true,
        };
        assert!(state.dispatch(command()).is_err());
        assert!(state.dispatch_checked(command(), Some(&[])).is_err());
        let next = state
            .dispatch_checked(command(), Some(&snapshot.document.rules))
            .unwrap();
        assert!(next.document.rules[0].system_exception);
        let bytes = fs::read(&state.path).unwrap();
        let stored: Document = serde_json::from_slice(&bytes).unwrap();
        assert!(stored.rules[0].system_exception);
        assert!(state
            .dispatch_checked(
                Command::SetSystemException {
                    id: "protected".into(),
                    enabled: false
                },
                Some(&snapshot.document.rules)
            )
            .is_err());
        let disabled = state
            .dispatch_checked(
                Command::SetSystemException {
                    id: "protected".into(),
                    enabled: false,
                },
                Some(&next.document.rules),
            )
            .unwrap();
        assert!(!disabled.document.rules[0].system_exception);
        // Persistence failure must leave the shared state protected.
        fs::remove_file(&state.path).unwrap();
        fs::create_dir(&state.path).unwrap();
        assert!(state
            .dispatch_checked(command(), Some(&disabled.document.rules))
            .is_err());
        assert!(!state.snapshot().unwrap().document.rules[0].system_exception);
        fs::remove_dir(&state.path).unwrap();
    }

    #[test]
    fn ipv6_ports_require_current_scoped_records_and_an_on_link_source() {
        let state = state();
        let address = |ip: &str, mask: &str, family: &str| crate::network::InterfaceAddress {
            interface: "en0".into(),
            address: ip.into(),
            family: family.into(),
            netmask: mask.into(),
            up: true,
        };
        let mut network = crate::network::NetworkSnapshot {
            addresses: vec![
                address("192.168.1.20", "255.255.255.0", "IPv4"),
                address("fe80::20%en0", "64", "IPv6"),
                address("fd12:3456::20", "64", "IPv6"),
                address("2001:db8:1::20", "64", "IPv6"),
            ],
            ..Default::default()
        };
        let discovery =
            crate::discovery::Plan::new(&network, "en0", "192.168.1.20", false).unwrap();
        let (op, started) = state.begin_discovery(&discovery).unwrap();
        let mut result = started.discovery;
        result.stage = "completed".into();
        result.devices = vec![crate::discovery::Device {
            id: "ipv6-record".into(),
            interface: "en0".into(),
            address: "fe80::2".into(),
            family: "IPv6".into(),
            mac: None,
            source: "ndp".into(),
            response: "unverified".into(),
            observed_at: now(),
            name: Default::default(),
        }];
        state.update_discovery(op, result).unwrap();
        let mut snapshot = state.snapshot().unwrap();
        let create = |snapshot: &Snapshot, n: &crate::network::NetworkSnapshot| {
            crate::ports::Plan::new(snapshot, n, "ipv6-record", op, 22, 22)
        };
        let ll = create(&snapshot, &network).unwrap();
        assert_eq!(ll.snapshot().target, "fe80::2%en0");
        assert_eq!(ll.snapshot().source_address, "fe80::20%en0");
        assert_eq!(ll.snapshot().family, "IPv6");
        assert!(ll.matches(&network));
        assert_eq!(
            state.open_ports("ipv6-record", op).unwrap().ports.operation,
            0
        );
        for ip in [
            "fe80::2%en1",
            "::1",
            "::",
            "::ffff:192.168.1.2",
            "ff02::1",
            "2001:db8:2::2",
            "4000::1",
        ] {
            snapshot.discovery.devices[0].address = ip.into();
            assert!(create(&snapshot, &network).is_err(), "{ip}");
        }
        for (ip, source) in [
            ("fd12:3456::2", "fd12:3456::20"),
            ("2001:db8:1::2", "2001:db8:1::20"),
        ] {
            snapshot.discovery.devices[0].address = ip.into();
            assert_eq!(
                create(&snapshot, &network)
                    .unwrap()
                    .snapshot()
                    .source_address,
                source
            );
        }
        snapshot.discovery.devices[0].address = "fe80::2%en0".into();
        network.addresses[1].netmask = "63".into();
        assert!(create(&snapshot, &network).is_err());
        network.addresses[1].netmask = "64".into();
        network.addresses[1].up = false;
        assert!(create(&snapshot, &network).is_err());
        assert!(!ll.matches(&network));
        network.addresses[1].up = true;
        network.addresses[0].netmask = "255.255.255.128".into();
        assert!(create(&snapshot, &network).is_err());
        network.addresses.retain(|a| a.family == "IPv6");
        snapshot.inventory.local_address = "fe80::20%en0".into();
        snapshot.inventory.netmask = "64".into();
        assert_eq!(
            create(&snapshot, &network).unwrap().snapshot().family,
            "IPv6"
        );
    }
    #[test]
    fn update_preferences_migrate_and_install_freezes_mutations() {
        let state = state();
        let legacy: Preferences =
            serde_json::from_str(r#"{"locale":"ja","view":"map","panel":"right","mode":"live"}"#)
                .unwrap();
        assert!(legacy.auto_updates);
        state
            .dispatch(Command::SetAutoUpdates { enabled: false })
            .unwrap();
        let persisted: Document =
            serde_json::from_slice(&std::fs::read(&state.path).unwrap()).unwrap();
        assert!(!persisted.preferences.auto_updates);
        state
            .update_change(|m| {
                m.snapshot.stage = "installing".into();
                Ok(())
            })
            .unwrap();
        assert!(state
            .dispatch(Command::SetAutoUpdates { enabled: true })
            .is_err());
        assert!(!state.snapshot().unwrap().document.preferences.auto_updates);
        assert!(state.begin_extension("activating").is_err());
    }
    #[test]
    fn service_plans_and_cancellation_are_fenced_to_open_ports() {
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
        let (generation, started) = state.begin_discovery(&discovery).unwrap();
        let mut result = started.discovery;
        result.stage = "completed".into();
        result.devices = vec![crate::discovery::Device {
            id: "target".into(),
            interface: "en0".into(),
            address: "192.168.1.30".into(),
            family: "IPv4".into(),
            mac: None,
            source: "arp".into(),
            response: "unverified".into(),
            observed_at: now(),
            name: Default::default(),
        }];
        state.update_discovery(generation, result).unwrap();
        let tcp = crate::ports::Plan::new(
            &state.snapshot().unwrap(),
            &network,
            "target",
            generation,
            22,
            23,
        )
        .unwrap();
        let (op, started) = state.begin_ports(&tcp).unwrap();
        let mut result = started.ports;
        result.stage = "completed".into();
        result.completed = 2;
        result.rows = vec![
            crate::ports::Row {
                port: 22,
                status: "open".into(),
                elapsed_ms: 1,
                observed_at: now(),
            },
            crate::ports::Row {
                port: 23,
                status: "closed".into(),
                elapsed_ms: 1,
                observed_at: now(),
            },
        ];
        state.update_ports(op, result).unwrap();
        let snapshot = state.snapshot().unwrap();
        let plan = crate::services::Plan::new(&snapshot, &network, op, "auto").unwrap();
        assert_eq!(
            plan.clone()
                .with_hostname(" Service.Example.Test ")
                .unwrap()
                .snapshot()
                .hostname
                .as_deref(),
            Some("service.example.test")
        );
        assert!(plan.clone().with_hostname("https://example.com").is_err());
        let mut http = plan.clone();
        http.mode = "http".into();
        assert!(http.with_hostname("service.example.test").is_err());
        assert_eq!(plan.tcp.ports, vec![22]);
        assert!(crate::services::Plan::new(&snapshot, &network, op + 1, "auto").is_err());
        assert!(crate::services::Plan::new(&snapshot, &network, op, "bad").is_err());
        let mut stale = snapshot.clone();
        stale.ports.source_address = "192.168.1.21".into();
        assert!(crate::services::Plan::new(&stale, &network, op, "auto").is_err());
        stale = snapshot.clone();
        stale.ports.rows[0].status = "timeout".into();
        assert!(crate::services::Plan::new(&stale, &network, op, "auto").is_err());
        stale = snapshot.clone();
        stale.ports.rows = (1..=65)
            .map(|port| crate::ports::Row {
                port,
                status: "open".into(),
                elapsed_ms: 0,
                observed_at: 0,
            })
            .collect();
        assert!(crate::services::Plan::new(&stale, &network, op, "auto").is_err());
        let (job, started) = state.begin_services(&plan).unwrap();
        assert_eq!(started.document.preferences.view, "servers");
        assert!(state.begin_services(&plan).is_err());
        assert!(state.begin_ports(&tcp).is_err());
        assert!(state.begin_discovery(&discovery).is_err());
        assert!(state.services_csv(job).is_err());
        state.stop_services().unwrap();
        assert!(state.services_cancelled(job));
        assert!(state
            .update_services(job, started.services.clone())
            .is_err());
        let mut late = started.services;
        late.stage = "completed".into();
        late.rows.push(crate::services::Row {
            port: 22,
            candidate: "SSH".into(),
            service: Some("SSH".into()),
            status: "confirmed".into(),
            method: "ssh".into(),
            product: None,
            response: None,
            tls: None,
            tls_details: None,
            observed_at: 0,
        });
        assert!(state
            .update_services(job, late)
            .unwrap()
            .services
            .rows
            .is_empty());
        let (next, started) = state.begin_services(&plan).unwrap();
        assert!(state.update_services(job, Default::default()).is_err());
        let mut complete = started.services;
        complete.stage = "completed".into();
        complete.rows.push(crate::services::Row {
            port: 22,
            candidate: "SSH".into(),
            service: Some("SSH".into()),
            status: "confirmed".into(),
            method: "ssh".into(),
            product: None,
            response: None,
            tls: None,
            tls_details: None,
            observed_at: 0,
        });
        complete.completed = 1;
        let first = state.update_services(next, complete).unwrap();
        assert_eq!(first.services.comparison.status, "first");
        let note = |port, expected_revision| crate::service_notes::Edit {
            port,
            expected_revision,
            label: "管理用".into(),
            comment: "確認メモ".into(),
        };
        assert!(state.edit_service_note(next + 1, note(22, 0)).is_err());
        assert!(state.edit_service_note(next, note(80, 0)).is_err());
        let annotated = state.edit_service_note(next, note(22, 0)).unwrap();
        assert_eq!(annotated.services.notes[&22].label, "管理用");
        assert_eq!(annotated.services.notes[&22].revision, 1);
        assert_eq!(
            annotated.services.comparison.status,
            first.services.comparison.status
        );
        assert_eq!(
            serde_json::to_value(&annotated.services.rows).unwrap(),
            serde_json::to_value(&first.services.rows).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&annotated.document).unwrap(),
            serde_json::to_value(&first.document).unwrap()
        );
        assert_eq!(
            state.edit_service_note(next, note(22, 0)).unwrap_err(),
            "service_note_conflict"
        );
        assert!(state.services_csv(next).unwrap().contains("管理用"));

        let (again, started) = state.begin_services(&plan).unwrap();
        assert_eq!(started.services.comparison.baseline_operation, Some(next));
        assert!(started.services.notes.is_empty());
        assert!(state.edit_service_note(again, note(22, 0)).is_err());
        state.stop_services().unwrap();
        let mut late = first.services.clone();
        late.operation = again;
        let stopped = state.update_services(again, late).unwrap();
        assert_eq!(stopped.services.comparison.status, "incomplete");
        let (final_op, _) = state.begin_services(&plan).unwrap();
        let mut repeated = first.services;
        repeated.operation = final_op;
        repeated.rows[0].product = Some("Changed/2".into());
        let compared = state.update_services(final_op, repeated).unwrap();
        assert_eq!(compared.services.comparison.baseline_operation, Some(next));
        assert_eq!(compared.services.comparison.rows[0].kind, "changed");
        let next = final_op;
        assert!(state.services_csv(job).is_err());
        assert!(state
            .services_csv(next)
            .unwrap()
            .contains("product_reported"));
        assert_eq!(
            state
                .select_service(next, 22)
                .unwrap()
                .services
                .selected_port,
            Some(22)
        );
        assert!(state.select_service(next, 80).is_err());
        assert!(state.select_service(job, 22).is_err());
        let mut changed = network.clone();
        changed.addresses[0].address = "192.168.1.21".into();
        let firewall = state.snapshot().unwrap().firewall;
        let invalidated = state.observed(vec![], firewall, changed).unwrap();
        assert_eq!(invalidated.services.stage, "failed");
        assert_eq!(invalidated.services.comparison.status, "invalidated");
        assert!(invalidated.services.comparison.rows.is_empty());
        assert_eq!(
            invalidated.services.error.as_deref(),
            Some("network_changed")
        );
        assert!(state.begin_services(&plan).is_err());
    }
    #[test]
    fn service_notes_are_blocked_while_running_or_installing_and_scope_by_operation() {
        let state = state();
        {
            let mut s = state.inner.lock().unwrap();
            s.services.operation = 7;
            s.services.stage = "completed".into();
            s.services.rows = vec![crate::services::Row {
                port: 443,
                candidate: "HTTPS".into(),
                service: None,
                status: "timeout".into(),
                method: "https".into(),
                product: None,
                response: None,
                tls: None,
                tls_details: None,
                observed_at: 1,
            }];
        }
        let edit = || crate::service_notes::Edit {
            port: 443,
            expected_revision: 0,
            label: "admin".into(),
            comment: "unknown".into(),
        };
        state.inner.lock().unwrap().services.stage = "running".into();
        assert!(state.edit_service_note(7, edit()).is_err());
        state.inner.lock().unwrap().services.stage = "cancelled".into();
        assert!(state.edit_service_note(6, edit()).is_err());
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state.edit_service_note(7, edit()).is_err());
        state.inner.lock().unwrap().updates.snapshot.stage = "idle".into();
        assert!(state.edit_service_note(7, edit()).is_ok());
        assert_eq!(state.snapshot().unwrap().services.rows[0].status, "timeout");
    }
    #[test]
    fn port_panel_close_restores_only_floating_without_editing_results_or_document() {
        use crate::port_layout::Panel;
        let state = state();
        let before = state.snapshot().unwrap();
        assert_eq!(before.ports.panel, Panel::Bottom);
        for panel in [Panel::Left, Panel::Right, Panel::Bottom] {
            state.port_panel(panel).unwrap();
            let current = state.port_panel_closed().unwrap();
            assert_eq!(current.ports.panel, panel); // A late close does not undo a dock choice.
        }
        let floating = state.port_panel(Panel::Floating).unwrap();
        assert_eq!(floating.ports.panel, Panel::Floating);
        let closed = state.port_panel_closed().unwrap();
        assert_eq!(closed.ports.panel, Panel::Bottom);
        assert_eq!(closed.ports.operation, before.ports.operation);
        assert_eq!(closed.ports.rows.len(), before.ports.rows.len());
        assert_eq!(closed.ports.layout, before.ports.layout);
        assert_eq!(
            serde_json::to_value(&closed.document).unwrap(),
            serde_json::to_value(&before.document).unwrap()
        );
        for panel in ["left", "right", "bottom", "floating"] {
            assert!(serde_json::from_str::<Panel>(&format!("\"{panel}\"")).is_ok());
        }
        assert!(serde_json::from_str::<Panel>("\"other\"").is_err());
    }
    #[test]
    fn port_layout_is_shared_conflict_checked_and_does_not_edit_the_document() {
        let state = state();
        let before = state.snapshot().unwrap();
        let mut layout = before.ports.layout.clone();
        layout.widths.insert("target".into(), 400);
        let after = state
            .set_port_layout(&before.ports.layout, layout.clone())
            .unwrap();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(state.snapshot().unwrap().ports.layout, layout);
        assert_eq!(
            serde_json::to_value(&after.document).unwrap(),
            serde_json::to_value(&before.document).unwrap()
        );
        assert_eq!(after.ports.rows.len(), before.ports.rows.len());
        assert_eq!(after.ports.operation, before.ports.operation);
        assert_eq!(
            state
                .set_port_layout(&before.ports.layout, before.ports.layout.clone())
                .unwrap_err(),
            "port_layout_conflict"
        );
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state.set_port_layout(&layout, before.ports.layout).is_err());
        assert_eq!(state.snapshot().unwrap().ports.layout, layout);
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
        let initial_layout = state.snapshot().unwrap().ports.layout;
        let mut compact_layout = initial_layout.clone();
        compact_layout.columns = vec!["port".into(), "status".into(), "inspect".into()];
        state
            .set_port_layout(&initial_layout, compact_layout.clone())
            .unwrap();
        state
            .port_panel(crate::port_layout::Panel::Floating)
            .unwrap();
        let (job, started) = state.begin_ports(&plan).unwrap();
        assert_eq!(started.ports.panel, crate::port_layout::Panel::Floating);
        assert_eq!(started.ports.layout, compact_layout);
        let mut resized_layout = compact_layout.clone();
        resized_layout.widths.insert("port".into(), 260);
        state
            .set_port_layout(&compact_layout, resized_layout.clone())
            .unwrap();
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
        assert_eq!(frozen.ports.layout, resized_layout);
        assert_eq!(frozen.ports.panel, crate::port_layout::Panel::Floating);
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
        finished.layout = Default::default(); // A worker does not own presentation settings.
        let updated = state.update_ports(next, finished).unwrap();
        assert_eq!(updated.ports.layout, resized_layout);
        assert_eq!(updated.ports.panel, crate::port_layout::Panel::Floating);
        assert!(state.ports_csv(job).is_err());
        assert!(state.ports_csv(next).unwrap().contains("requested_ports"));
        let csv_before_layout = state.ports_csv(next).unwrap();
        state
            .set_port_layout(&resized_layout, Default::default())
            .unwrap();
        assert_eq!(state.ports_csv(next).unwrap(), csv_before_layout);

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
                values: vec![],
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
        assert!(state.lan_copy(op).is_err());
        state
            .lan_edit(
                op,
                crate::inventory::Edit::Selection {
                    ids: vec!["en0|192.168.1.1".into()],
                    expected: vec![],
                },
            )
            .unwrap();
        let edited = state
            .lan_edit(
                op,
                crate::inventory::Edit::Comment {
                    id: "en0|192.168.1.1".into(),
                    value: "shared comment".into(),
                    expected: "".into(),
                },
            )
            .unwrap();
        assert_eq!(
            edited.inventory.comments["en0|192.168.1.1"],
            "shared comment"
        );
        assert!(state.lan_copy(op).unwrap().contains("shared comment"));
        state.begin_discovery(&plan).unwrap();
        assert!(state.lan_copy(op).is_err());
        assert!(state
            .lan_edit(
                op,
                crate::inventory::Edit::Selection {
                    ids: vec![],
                    expected: vec![]
                }
            )
            .is_err());
        assert!(state.snapshot().unwrap().inventory.comments.is_empty());
        assert!(state.snapshot().unwrap().inventory.checked_ids.is_empty());
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
            transport: "TCP".into(),
            system_exception: false,
            timeout_action: "ask".into(),
            identity: Some(crate::identity::SigningIdentity {
                authority: "developer".into(),
                team_id: "ABCDE12345".into(),
                identifier: "com.test.app".into(),
            }),
            id: "r".into(),
            app_key: "signed:v1:developer:ABCDE12345:com.test.app".into(),
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
        valid.timeout_action = "block".into();
        let mut demo = rule;
        demo.source = "demo".into();
        demo.id = "demo".into();
        s.inner.lock().unwrap().document.rules = vec![valid.clone(), demo];
        let (_, sent, snapshot) = s.begin_extension("activating").unwrap();
        assert_eq!(sent, vec![valid]);
        assert!(snapshot.extension.busy);
    }
    #[test]
    fn non_tcp_observations_cannot_create_tcp_intentions() {
        let s = state();
        for protocol in ["UDP", "unknown", ""] {
            let mut c = crate::model::demo_connections().remove(0);
            c.protocol = protocol.into();
            {
                let mut inner = s.inner.lock().unwrap();
                inner.document.preferences.mode = "live".into();
                inner.connections = vec![c.clone()];
            }
            let before = s.snapshot().unwrap();
            assert_eq!(
                s.dispatch(Command::SaveRule {
                    connection_id: c.id,
                    action: "block".into(),
                    direction: "both".into()
                })
                .err()
                .unwrap(),
                "unsupported_protocol"
            );
            assert_eq!(s.snapshot().unwrap().revision, before.revision);
            assert!(s.snapshot().unwrap().document.rules.is_empty());
        }
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
    #[test]
    fn recovery_admission_is_atomic_and_blocked_by_install() {
        let state = state();
        state
            .extension_change(|e, _| e.configure(true, true, true))
            .unwrap();
        let (op, rules, _) = state.begin_extension("applying").unwrap();
        state
            .extension_change(|e, _| e.fail(op, "timeout".into()))
            .unwrap();
        state.inner.lock().unwrap().updates.snapshot.stage = "installing".into();
        assert!(state.begin_recovery(100).is_err());
        state.inner.lock().unwrap().updates.snapshot.stage = "idle".into();
        let (new, sent, snapshot) = state.begin_recovery(100).unwrap();
        assert_eq!(sent, rules);
        assert!(snapshot.extension.busy);
        assert!(new > op);
        assert!(state.begin_recovery(101).is_err());
        state
            .extension_change(|e, current| e.accepted(new, "restarted".into(), 1, sent, current))
            .unwrap();
        assert!(state.begin_recovery(1000).is_err());
    }
    #[test]
    fn legacy_backup_is_retained_but_never_sent_or_acknowledged() {
        let state = state();
        let mut rules = Vec::new();
        let c = demo_connections().remove(0);
        rules.push(Rule {
            transport: "TCP".into(),
            system_exception: false,
            timeout_action: "allow".into(),
            identity: None,
            id: "legacy".into(),
            app_key: c.app_key,
            app: c.app,
            destination: c.destination,
            port: c.port,
            direction: "both".into(),
            action: "allow".into(),
            updated_at: 10,
            source: "live".into(),
        });
        state
            .dispatch(Command::RestoreRules {
                rules: rules.clone(),
            })
            .unwrap();
        state
            .extension_change(|e, _| e.configure(true, true, true))
            .unwrap();
        let (_, sent, snapshot) = state.begin_extension("applying").unwrap();
        assert!(sent.is_empty());
        assert_eq!(snapshot.document.rules, rules);
        assert_eq!(
            crate::extension::policy_payload(&rules).unwrap(),
            br#"{"rules":[]}"#
        );
        let path = state.path.clone();
        drop(state);
        let reopened = SharedState::load(path).unwrap().snapshot().unwrap();
        assert_eq!(reopened.document.rules, rules);
    }
    #[test]
    fn unsigned_or_missing_process_cannot_save_or_migrate() {
        let state = state();
        let mut c = demo_connections().remove(0);
        c.pid = 0;
        c.identity = None;
        state.inner.lock().unwrap().connections = vec![c.clone()];
        let legacy = Rule {
            transport: "TCP".into(),
            system_exception: false,
            timeout_action: "allow".into(),
            identity: None,
            id: "legacy".into(),
            app_key: c.app_key.clone(),
            app: c.app,
            destination: c.destination,
            port: c.port,
            direction: "both".into(),
            action: "block".into(),
            updated_at: 10,
            source: "live".into(),
        };
        state
            .dispatch(Command::RestoreRules {
                rules: vec![legacy.clone()],
            })
            .unwrap();
        assert!(state
            .dispatch(Command::SaveRule {
                connection_id: c.id.clone(),
                action: "allow".into(),
                direction: "both".into()
            })
            .is_err());
        assert!(state
            .dispatch(Command::MigrateRule {
                id: legacy.id.clone(),
                connection_id: c.id
            })
            .is_err());
        assert_eq!(state.snapshot().unwrap().document.rules, vec![legacy]);
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires host Security guest access; no OS configuration changes"]
    fn migration_revalidates_signed_guest_preserves_intent_and_rolls_back() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let result = (|| {
            let identity = crate::identity::for_pid(child.id()).unwrap();
            let state = state();
            let mut c = demo_connections().remove(0);
            c.app_key = "/bin/sleep".into();
            c.pid = child.id();
            c.identity = Some(identity.clone());
            state.inner.lock().unwrap().connections = vec![c.clone()];
            let legacy = Rule {
                transport: "TCP".into(),
                system_exception: false,
                timeout_action: "allow".into(),
                identity: None,
                id: "legacy".into(),
                app_key: c.app_key.clone(),
                app: c.app,
                destination: c.destination.clone(),
                port: c.port,
                direction: "outbound".into(),
                action: "block".into(),
                updated_at: 10,
                source: "live".into(),
            };
            state
                .dispatch(Command::RestoreRules {
                    rules: vec![legacy.clone()],
                })
                .unwrap();
            // The same path is insufficient when the current observed subject changed.
            state.inner.lock().unwrap().connections[0].identity = None;
            assert!(state
                .dispatch(Command::MigrateRule {
                    id: legacy.id.clone(),
                    connection_id: c.id.clone()
                })
                .is_err());
            state.inner.lock().unwrap().connections[0].identity = Some(identity.clone());
            let migrated = state
                .dispatch(Command::MigrateRule {
                    id: legacy.id.clone(),
                    connection_id: c.id.clone(),
                })
                .unwrap();
            let rule = &migrated.document.rules[0];
            assert_eq!(rule.identity.as_ref(), Some(&identity));
            assert_eq!(rule.app_key, identity.key());
            assert_eq!(rule.action, legacy.action);
            assert_eq!(rule.direction, legacy.direction);
            assert_eq!(rule.destination, legacy.destination);
            assert_eq!(rule.id, legacy.id);
            assert!(!migrated.extension.applied_rule_ids.contains(&rule.id));
            // Undo safely restores the disabled legacy rule without signature reattachment.
            state
                .dispatch(Command::RestoreRules {
                    rules: vec![legacy.clone()],
                })
                .unwrap();
            // Existing signed scope must not be silently overwritten by migration.
            let mut signed = rule.clone();
            signed.id = "existing".into();
            state
                .dispatch(Command::RestoreRules {
                    rules: vec![legacy.clone(), signed],
                })
                .unwrap();
            assert!(state
                .dispatch(Command::MigrateRule {
                    id: legacy.id.clone(),
                    connection_id: c.id.clone()
                })
                .is_err());
            state
                .dispatch(Command::RestoreRules {
                    rules: vec![legacy.clone()],
                })
                .unwrap();
            // Force atomic-write failure and ensure the in-memory migration is rolled back.
            std::fs::create_dir(state.path.with_extension("tmp")).unwrap();
            assert!(state
                .dispatch(Command::MigrateRule {
                    id: legacy.id.clone(),
                    connection_id: c.id.clone()
                })
                .is_err());
            assert_eq!(state.snapshot().unwrap().document.rules, vec![legacy]);
            // After the guest exits, an observed signature cannot authorize another save.
            let _ = child.kill();
            let _ = child.wait();
            assert!(state
                .dispatch(Command::SaveRule {
                    connection_id: c.id,
                    action: "allow".into(),
                    direction: "both".into()
                })
                .is_err());
        })();
        let _ = child.kill();
        let _ = child.wait();
        result
    }
    #[test]
    fn ask_timeout_default_persists_and_updates_intentions_with_conflict_guard() {
        let state = state();
        assert_eq!(
            state
                .snapshot()
                .unwrap()
                .document
                .preferences
                .ask_timeout_action,
            "allow"
        );
        state
            .dispatch(Command::SetMode {
                mode: "demo".into(),
            })
            .unwrap();
        let c = state.snapshot().unwrap().connections[0].id.clone();
        state
            .dispatch(Command::SaveRule {
                connection_id: c,
                action: "ask".into(),
                direction: "both".into(),
            })
            .unwrap();
        let previous = state.snapshot().unwrap().document.rules;
        let snapshot = state
            .dispatch_checked(
                Command::SetAskTimeoutAction {
                    action: "block".into(),
                },
                Some(&previous),
            )
            .unwrap();
        assert_eq!(snapshot.document.preferences.ask_timeout_action, "block");
        assert_eq!(snapshot.document.rules[0].timeout_action, "block");
        assert!(state
            .dispatch_checked(
                Command::SetAskTimeoutAction {
                    action: "allow".into()
                },
                Some(&previous)
            )
            .is_err());
        let path = state.path.clone();
        drop(state);
        let reopened = SharedState::load(path).unwrap().snapshot().unwrap();
        assert_eq!(reopened.document.preferences.ask_timeout_action, "block");
        assert!(reopened.extension.applied_rule_ids.is_empty());
    }
}
