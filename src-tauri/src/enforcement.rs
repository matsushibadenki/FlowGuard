use crate::model::Rule;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStatus {
    pub stage: String,
    pub packaged: bool,
    pub runtime_available: bool,
    pub signed: bool,
    pub busy: bool,
    pub instance_id: Option<String>,
    pub provider_revision: Option<u64>,
    pub applied_rule_ids: Vec<String>,
    pub error: Option<String>,
}
impl Default for ExtensionStatus {
    fn default() -> Self {
        Self {
            stage: "not_packaged".into(),
            packaged: false,
            runtime_available: false,
            signed: false,
            busy: false,
            instance_id: None,
            provider_revision: None,
            applied_rule_ids: vec![],
            error: None,
        }
    }
}
#[derive(Default)]
pub struct Enforcement {
    pub status: ExtensionStatus,
    applied: Vec<Rule>,
    operation: u64,
}
impl Enforcement {
    pub fn generation(&self) -> u64 {
        self.operation
    }
    pub fn configure(&mut self, packaged: bool, runtime: bool, signed: bool) {
        self.status.packaged = packaged;
        self.status.runtime_available = runtime;
        self.status.signed = signed;
        self.status.stage = if !packaged {
            "not_packaged"
        } else if !runtime {
            "runtime_unavailable"
        } else if !signed {
            "signature_required"
        } else {
            "inactive"
        }
        .into();
    }
    pub fn begin(&mut self, stage: &str) -> Result<u64, String> {
        if stage == "activating"
            && [
                "ready",
                "applied",
                "stale",
                "configuration_saved",
                "restart_required",
            ]
            .contains(&self.status.stage.as_str())
        {
            return Err("extension_already_active".into());
        }
        if self.status.busy {
            return Err("extension_busy".into());
        }
        if !self.status.packaged || !self.status.runtime_available {
            return Err("extension_not_packaged".into());
        }
        // Let activation traverse the actual Swift signature gate; applying requires a signed host.
        if stage == "applying" && !self.status.signed {
            return Err("signature_required".into());
        }
        self.operation += 1;
        self.status.busy = true;
        self.status.stage = stage.into();
        self.status.error = None;
        self.status.applied_rule_ids.clear();
        Ok(self.operation)
    }
    pub fn lifecycle(&mut self, operation: u64, stage: &str) -> bool {
        if self.operation != operation || !self.status.busy {
            return false;
        }
        if ![
            "approval_required",
            "restart_required",
            "configuration_saved",
            "signature_required",
            "configuration_conflict",
            "failed",
            "busy",
        ]
        .contains(&stage)
        {
            return false;
        }
        self.status.stage = stage.into();
        self.status.busy = stage == "approval_required";
        if ["failed", "configuration_conflict", "busy"].contains(&stage) {
            self.status.error = Some(stage.into());
        }
        self.status.applied_rule_ids.clear();
        true
    }
    pub fn fail(&mut self, operation: u64, error: String) {
        if self.operation != operation {
            return;
        }
        self.status.stage = "failed".into();
        self.status.error = Some(error);
        self.status.busy = false;
        self.status.applied_rule_ids.clear();
    }
    pub fn accepted(
        &mut self,
        operation: u64,
        instance: String,
        revision: u64,
        rules: Vec<Rule>,
        current: &[Rule],
    ) {
        if self.operation != operation || !self.status.busy {
            return;
        }
        self.status.instance_id = Some(instance);
        self.status.provider_revision = Some(revision);
        self.applied = rules;
        self.status.busy = false;
        self.status.error = None;
        self.status.stage = "applied".into();
        self.reconcile(current);
    }
    pub fn reconcile(&mut self, current: &[Rule]) {
        let live: Vec<_> = current
            .iter()
            .filter(|r| r.source == "live")
            .cloned()
            .collect();
        if ["applied", "stale"].contains(&self.status.stage.as_str()) {
            if self.applied == live {
                self.status.stage = "applied".into();
                self.status.applied_rule_ids = live.iter().map(|r| r.id.clone()).collect();
            } else {
                self.status.stage = "stale".into();
                self.status.applied_rule_ids.clear();
            }
        }
    }
    pub fn observed(&mut self, instance: &str, revision: u64, ready: bool) {
        if self.status.busy {
            return;
        }
        let same = self.status.instance_id.as_deref() == Some(instance)
            && self.status.provider_revision == Some(revision);
        if !ready {
            self.disconnected("provider_unavailable");
            return;
        }
        if !same {
            self.applied.clear();
            self.status.applied_rule_ids.clear();
            self.status.stage = "ready".into();
            self.status.instance_id = Some(instance.into());
            self.status.provider_revision = Some(revision);
        } else if !["applied", "stale"].contains(&self.status.stage.as_str()) {
            self.status.stage = "ready".into();
        }
        self.status.error = None;
    }
    pub fn disconnected(&mut self, error: &str) {
        if self.status.busy {
            return;
        }
        self.status.stage = "disconnected".into();
        self.status.applied_rule_ids.clear();
        self.status.error = Some(error.into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Vec<Rule> {
        vec![Rule {
            id: "r".into(),
            app_key: "/test".into(),
            app: "Test".into(),
            destination: "1.2.3.4".into(),
            port: 443,
            direction: "both".into(),
            action: "block".into(),
            updated_at: 1,
            source: "live".into(),
        }]
    }
    #[test]
    fn configuration_and_approval_do_not_claim_application() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        let op = state.begin("activating").unwrap();
        assert!(state.begin("activating").is_err());
        state.lifecycle(op, "approval_required");
        assert!(state.status.busy);
        state.lifecycle(op, "configuration_saved");
        assert!(!state.status.busy);
        assert!(state.status.applied_rule_ids.is_empty());
    }
    #[test]
    fn edits_during_apply_restart_and_disconnect_invalidate_receipts() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        let op = state.begin("applying").unwrap();
        let original = rules();
        let mut edited = original.clone();
        edited[0].action = "allow".into();
        state.accepted(op, "instance".into(), 1, original.clone(), &edited);
        assert_eq!(state.status.stage, "stale");
        assert!(state.status.applied_rule_ids.is_empty());
        state.reconcile(&original);
        assert_eq!(state.status.stage, "applied");
        state.observed("restarted", 0, true);
        assert_eq!(state.status.stage, "ready");
        assert!(state.status.applied_rule_ids.is_empty());
        state.disconnected("lost");
        assert_eq!(state.status.stage, "disconnected");
    }
    #[test]
    fn unsigned_and_failed_requests_never_report_applied() {
        let mut state = Enforcement::default();
        state.configure(true, true, false);
        assert!(state.begin("applying").is_err());
        let op = state.begin("activating").unwrap();
        state.lifecycle(op, "signature_required");
        assert_eq!(state.status.stage, "signature_required");
        assert!(!state.status.busy);
        assert!(!state.lifecycle(op, "configuration_saved"));
    }
}
