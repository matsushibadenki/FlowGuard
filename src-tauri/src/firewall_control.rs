//! Explicit OS firewall operations. This switch is independent of FlowGuard's per-app filter.
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Status,
    Register,
    Unregister,
    ApprovalSettings,
    Enable,
    Disable,
}
impl Action {
    pub fn args(self, expected: Option<bool>) -> Result<Vec<&'static str>, String> {
        Ok(match self {
            Self::Status => vec!["--status"],
            Self::Register => vec!["--register"],
            Self::Unregister => vec!["--unregister"],
            Self::ApprovalSettings => vec!["--approval_settings"],
            Self::Enable | Self::Disable => vec![
                if self == Self::Enable {
                    "--enable"
                } else {
                    "--disable"
                },
                if expected.ok_or("known_state_required")? {
                    "on"
                } else {
                    "off"
                },
            ],
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    pub service: String,
    pub enabled: Option<bool>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub operation: u64,
    pub busy: bool,
    pub service: String,
    pub enabled: Option<bool>,
    pub error: Option<String>,
}
#[derive(Default)]
pub struct Machine {
    pub snapshot: Snapshot,
}
impl Machine {
    pub fn begin(
        &mut self,
        action: Action,
        expected: Option<bool>,
        acknowledged: bool,
    ) -> Result<u64, String> {
        if self.snapshot.busy {
            return Err("firewall_busy".into());
        }
        if action != Action::Status && !acknowledged {
            return Err("confirmation_required".into());
        }
        action.args(expected)?;
        if matches!(action, Action::Enable | Action::Disable)
            && (self.snapshot.service != "enabled"
                || expected.is_none()
                || expected != self.snapshot.enabled
                || self.snapshot.error.is_some())
        {
            return Err("refresh_required".into());
        }
        self.snapshot.operation += 1;
        self.snapshot.busy = true;
        self.snapshot.error = None;
        // Clear evidence while an explicit change may be in progress.
        self.snapshot.enabled = None;
        Ok(self.snapshot.operation)
    }
    pub fn finish(&mut self, op: u64, result: Result<Reply, String>) -> Result<(), String> {
        if op != self.snapshot.operation || !self.snapshot.busy {
            return Err("stale_operation".into());
        }
        self.snapshot.busy = false;
        match result {
            Ok(reply) => {
                self.snapshot.service = reply.service;
                self.snapshot.enabled = reply.enabled;
                self.snapshot.error = reply.error;
            }
            Err(_) => {
                self.snapshot.enabled = None;
                self.snapshot.service = "unavailable".into();
                self.snapshot.error = Some("result_unknown".into());
            }
        }
        Ok(())
    }
}
pub fn execute(contents: &Path, action: Action, expected: Option<bool>) -> Result<Reply, String> {
    let args = action.args(expected)?;
    let binary = contents.join("MacOS/FlowGuardFirewallHelper");
    if !binary.is_file() {
        return Ok(Reply {
            service: "not_packaged".into(),
            enabled: None,
            error: Some("not_packaged".into()),
        });
    }
    let (success, data) =
        crate::observer::run_firewall_helper(binary.to_str().ok_or("bundle_path")?, &args)?;
    if data.len() > 4096 {
        return Err("invalid_reply".into());
    }
    let reply: Reply = serde_json::from_str(&data).map_err(|_| "invalid_reply")?;
    if !success
        || ![
            "enabled",
            "requires_approval",
            "not_registered",
            "unavailable",
        ]
        .contains(&reply.service.as_str())
    {
        return Err("helper_unavailable".into());
    }
    Ok(reply)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_commands_and_strict_input() {
        assert!(serde_json::from_str::<Action>("\"sudo\"").is_err());
        assert!(Action::Enable.args(None).is_err());
        assert_eq!(
            Action::Disable.args(Some(true)).unwrap(),
            ["--disable", "on"]
        );
        assert_eq!(Action::Status.args(None).unwrap(), ["--status"]);
    }
    #[test]
    fn confirmation_concurrency_unknown_and_stale_results() {
        let mut m = Machine::default();
        assert!(m.begin(Action::Register, None, false).is_err());
        assert!(m.begin(Action::Enable, Some(true), true).is_err());
        let op = m.begin(Action::Status, None, false).unwrap();
        assert!(m.begin(Action::Status, None, false).is_err());
        assert!(m.finish(op + 1, Err("late".into())).is_err());
        m.finish(
            op,
            Ok(Reply {
                service: "enabled".into(),
                enabled: Some(false),
                error: None,
            }),
        )
        .unwrap();
        assert!(m.begin(Action::Enable, Some(true), true).is_err());
        let op = m.begin(Action::Enable, Some(false), true).unwrap();
        assert!(m.snapshot.enabled.is_none());
        m.finish(op, Err("timeout".into())).unwrap();
        assert!(m.snapshot.enabled.is_none());
        assert!(m.begin(Action::Disable, Some(true), true).is_err());
        assert!(m
            .finish(
                op,
                Ok(Reply {
                    service: "enabled".into(),
                    enabled: Some(true),
                    error: None
                })
            )
            .is_err());
    }
}
