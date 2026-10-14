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
    target: Option<Vec<Rule>>,
    recovery: bool,
    emergency_latched: bool,
    retries: u8,
    retry_at: u64,
}
impl Enforcement {
    pub fn requested(&mut self, rules: &[Rule]) {
        self.target = Some(
            rules
                .iter()
                .filter(|r| crate::model::enforceable(r))
                .cloned()
                .collect(),
        );
        self.recovery = false;
        self.retries = 0;
        self.retry_at = 0;
    }
    pub fn recovery_due(&self, current: &[Rule], now: u64) -> bool {
        let live: Vec<_> = current
            .iter()
            .filter(|r| crate::model::enforceable(r))
            .cloned()
            .collect();
        self.recovery
            && !self.emergency_latched
            && self.status.signed
            && !self.status.busy
            && self.retries < 3
            && now >= self.retry_at
            && self.target.as_ref() == Some(&live)
            && crate::extension::policy_payload(&live).is_ok()
            && ![
                "disabled",
                "removed",
                "approval_required",
                "restart_required",
                "configuration_conflict",
            ]
            .contains(&self.status.stage.as_str())
    }
    pub fn recovering(&mut self, now: u64) {
        self.retries += 1;
        self.retry_at = now + 5 * (1u64 << self.retries);
        self.recovery = false;
    }
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
        if self.status.busy && (stage != "disabling" || self.status.stage == "disabling") {
            return Err("extension_busy".into());
        }
        if !self.status.packaged || !self.status.runtime_available {
            return Err("extension_not_packaged".into());
        }
        // Let activation traverse the actual Swift signature gate; applying requires a signed host.
        if stage == "applying" && self.emergency_latched {
            return Err("extension_inactive".into());
        }
        if stage == "applying" && !self.status.signed {
            return Err("signature_required".into());
        }
        if stage == "applying"
            && ["disabled", "removed", "restart_required"].contains(&self.status.stage.as_str())
        {
            return Err("extension_inactive".into());
        }
        if stage == "activating" {
            self.emergency_latched = false;
        }
        if ["disabling", "removing"].contains(&stage) {
            self.emergency_latched = true;
            self.target = None;
            self.recovery = false;
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
            "initial_sync",
            "disabled",
            "removed",
            "invalid_initial_policy",
        ]
        .contains(&stage)
        {
            return false;
        }
        self.status.stage = stage.into();
        self.status.busy = ["approval_required", "initial_sync"].contains(&stage);
        if ["disabled", "removed"].contains(&stage) {
            self.applied.clear();
            self.target = None;
            self.recovery = false;
            self.status.instance_id = None;
            self.status.provider_revision = None;
        }
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
        if ["applying", "initial_sync"].contains(&self.status.stage.as_str()) {
            self.recovery = self.target.is_some()
                && ![
                    "startup_policy_configuration_conflict",
                    "unsupported_or_invalid_rules",
                    "startup_policy_signature_required",
                ]
                .contains(&error.as_str());
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
        self.recovery = false;
        self.retries = 0;
        self.retry_at = 0;
        self.status.busy = false;
        self.status.error = None;
        self.status.stage = "applied".into();
        self.reconcile(current);
    }
    pub fn reconcile(&mut self, current: &[Rule]) {
        let live: Vec<_> = current
            .iter()
            .filter(|r| crate::model::enforceable(r))
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
            self.recovery = self.target.is_some();
            self.applied.clear();
            self.status.applied_rule_ids.clear();
            self.status.stage = "ready".into();
            self.status.instance_id = Some(instance.into());
            self.status.provider_revision = Some(revision);
        } else if !["applied", "stale"].contains(&self.status.stage.as_str()) {
            self.status.stage = "ready".into();
        }
        self.status.error = None;
        if self.recovery && self.retries >= 3 {
            self.status.stage = "failed".into();
            self.status.error = Some("recovery_exhausted".into());
        }
    }
    pub fn mismatch(&mut self) {
        if self.status.busy {
            return;
        }
        self.recovery = self.target.is_some();
        self.applied.clear();
        self.status.applied_rule_ids.clear();
        self.status.stage = if self.retries >= 3 { "failed" } else { "ready" }.into();
        if self.retries >= 3 {
            self.status.error = Some("recovery_exhausted".into());
        }
    }
    pub fn expected_digest(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|r| crate::extension::policy_digest(r))
    }
    pub fn disconnected(&mut self, error: &str) {
        if self.status.busy {
            return;
        }
        self.recovery = self.target.is_some();
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
            system_exception: false,
            timeout_action: "allow".into(),
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
    fn initial_sync_keeps_operation_busy_and_edits_stale() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        let op = state.begin("activating").unwrap();
        assert!(state.lifecycle(op, "initial_sync"));
        assert!(state.status.busy);
        assert!(state.begin("applying").is_err());
        let sent = rules();
        let mut current = sent.clone();
        current[0].action = "allow".into();
        state.accepted(op, "instance".into(), 1, sent, &current);
        assert_eq!(state.status.stage, "stale");
        assert!(state.status.applied_rule_ids.is_empty());
    }
    #[test]
    fn disable_and_remove_clear_receipts_and_reject_late_callbacks() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        let op = state.begin("applying").unwrap();
        state.accepted(op, "instance".into(), 1, rules(), &rules());
        let disable = state.begin("disabling").unwrap();
        assert!(state.status.applied_rule_ids.is_empty());
        assert!(!state.lifecycle(op, "configuration_saved"));
        state.lifecycle(disable, "disabled");
        assert_eq!(state.status.stage, "disabled");
        assert!(state.status.instance_id.is_none());
        assert!(state.begin("applying").is_err());
        let remove = state.begin("removing").unwrap();
        state.lifecycle(remove, "restart_required");
        assert_eq!(state.status.stage, "restart_required");
        assert!(!state.status.busy);
        let retry = state.begin("removing").unwrap();
        state.fail(retry, "denied".into());
        assert_eq!(state.status.stage, "failed");
        let retry = state.begin("removing").unwrap();
        state.lifecycle(retry, "removed");
        assert_eq!(state.status.stage, "removed");
        assert!(state.begin("applying").is_err());
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
    #[test]
    fn recovery_requires_explicit_intent_and_bounds_retries() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        state.observed("first", 0, true);
        assert!(!state.recovery_due(&rules(), 100));
        let op = state.begin("applying").unwrap();
        state.requested(&rules());
        state.accepted(op, "first".into(), 1, rules(), &rules());
        state.observed("restarted", 0, true);
        assert!(state.recovery_due(&rules(), 100));
        let mut edited = rules();
        edited[0].action = "allow".into();
        assert!(!state.recovery_due(&edited, 100));
        for now in [100, 120, 160] {
            let op = state.begin("applying").unwrap();
            state.recovering(now);
            state.fail(op, "timeout".into());
            assert!(!state.recovery_due(&rules(), now));
        }
        assert!(!state.recovery_due(&rules(), 1000));
        let op = state.begin("applying").unwrap();
        state.requested(&rules());
        state.fail(op, "timeout".into());
        assert!(state.recovery_due(&rules(), 1000));
        let op = state.begin("disabling").unwrap();
        state.fail(op, "timeout".into());
        assert!(!state.recovery_due(&rules(), 1000));
    }
    #[test]
    fn emergency_disable_preempts_busy_and_fences_every_old_completion() {
        for stage in ["activating", "applying", "removing"] {
            let mut state = Enforcement::default();
            state.configure(true, true, true);
            let old = state.begin(stage).unwrap();
            state.requested(&rules());
            let emergency = state.begin("disabling").unwrap();
            assert!(emergency > old);
            assert!(state.begin("disabling").is_err());
            assert!(!state.lifecycle(old, "configuration_saved"));
            state.fail(old, "late_timeout".into());
            state.accepted(old, "old".into(), 1, rules(), &rules());
            assert_eq!(state.status.stage, "disabling");
            assert!(state.status.applied_rule_ids.is_empty());
            assert!(!state.recovery_due(&rules(), u64::MAX));
            assert!(state.lifecycle(emergency, "disabled"));
            assert!(!state.recovery_due(&rules(), u64::MAX));
        }
    }

    #[test]
    fn failed_disable_does_not_silently_reenable_recovery() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        state.requested(&rules());
        let operation = state.begin("disabling").unwrap();
        assert!(state.lifecycle(operation, "failed"));
        assert!(!state.recovery_due(&rules(), u64::MAX));
        assert!(state.status.applied_rule_ids.is_empty());
        assert_eq!(state.status.stage, "failed");
        // A failed emergency cannot be bypassed by manual apply or automatic recovery.
        assert!(state.begin("applying").is_err());
        assert!(state.begin("activating").is_ok());
        assert!(!state.emergency_latched);
    }

    #[test]
    fn recovery_late_ack_cannot_overwrite_disable() {
        let mut state = Enforcement::default();
        state.configure(true, true, true);
        let old = state.begin("applying").unwrap();
        state.requested(&rules());
        state.fail(old, "timeout".into());
        let current = state.begin("removing").unwrap();
        state.lifecycle(current, "removed");
        state.accepted(old, "late".into(), 1, rules(), &rules());
        assert_eq!(state.status.stage, "removed");
        assert!(state.status.applied_rule_ids.is_empty());
    }
}
