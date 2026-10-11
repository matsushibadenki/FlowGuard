use crate::{enforcement::Enforcement, model::*, renderer};
use std::{collections::HashSet, fs, path::PathBuf, sync::Mutex};
pub struct SharedState {
    pub inner: Mutex<Session>,
    pub path: PathBuf,
    _document_lock: fs::File,
}
pub struct Session {
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
            || !["map", "apps", "rules", "history", "settings"].contains(&p.view.as_str())
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
                if !["map", "apps", "rules", "history", "settings"].contains(&view.as_str()) {
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
    pub fn observed(
        &self,
        mut connections: Vec<Connection>,
        firewall: FirewallStatus,
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
