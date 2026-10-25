// パス: src-tauri/src/asks.rs
use crate::identity::SigningIdentity;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub instance_id: String,
    pub policy_id: String,
    pub rule_id: String,
    pub identity: SigningIdentity,
    pub app: String,
    pub destination: String,
    pub port: u16,
    pub direction: String,
    pub timeout_action: String,
    pub remaining_seconds: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Outcome {
    pub request: Request,
    pub action: String,
    pub reason: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Envelope {
    pub protocol_version: u32,
    pub instance_id: String,
    pub policy_id: String,
    pub requests: Vec<Request>,
    pub outcomes: Vec<Outcome>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub protocol_version: u32,
    pub instance_id: String,
    pub policy_id: String,
    pub request_id: String,
    pub action: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    pub protocol_version: u32,
    pub instance_id: String,
    pub policy_id: String,
    pub request_id: String,
    pub status: String,
    pub action: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub requests: Vec<Request>,
    pub outcomes: Vec<Outcome>,
    pub busy_ids: Vec<String>,
    pub poll_revision: u64,
    pub error: Option<String>,
    pub last_answer: Option<String>,
}
#[derive(Default)]
pub struct Machine {
    pub snapshot: Snapshot,
    generation: u64,
    busy: HashMap<String, u64>,
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn action(value: &str) -> bool {
    ["allow", "block"].contains(&value)
}
impl Request {
    fn valid(&self) -> bool {
        uuid(&self.request_id)
            && uuid(&self.instance_id)
            && uuid(&self.policy_id)
            && self.identity.valid()
            && !self.rule_id.is_empty()
            && self.rule_id.len() <= 8192
            && self.app.len() <= 256
            && !self.destination.is_empty()
            && self.destination.len() <= 512
            && !self.destination.contains('\0')
            && self.port > 0
            && ["inbound", "outbound"].contains(&self.direction.as_str())
            && action(&self.timeout_action)
            && self.remaining_seconds <= 15
    }
}
impl Envelope {
    fn valid(&self) -> bool {
        let mut ids = std::collections::HashSet::new();
        self.protocol_version == 5
            && uuid(&self.instance_id)
            && uuid(&self.policy_id)
            && self.requests.len() <= 128
            && self.outcomes.len() <= 64
            && self.requests.iter().all(|r| {
                r.valid()
                    && r.instance_id == self.instance_id
                    && r.policy_id == self.policy_id
                    && ids.insert(&r.request_id)
            })
            && self.outcomes.iter().all(|o| {
                o.request.valid()
                    && o.request.instance_id == self.instance_id
                    && action(&o.action)
                    && [
                        "answered",
                        "timed_out",
                        "policy_changed",
                        "provider_stopped",
                        "capacity",
                        "transport_unavailable",
                    ]
                    .contains(&o.reason.as_str())
                    && (o.reason == "answered" || o.action == o.request.timeout_action)
            })
    }
}
impl Machine {
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.busy.clear();
        self.snapshot.requests.clear();
        self.snapshot.busy_ids.clear();
        self.snapshot.outcomes.clear();
        self.snapshot.last_answer = None;
        self.snapshot.error = None;
    }
    pub fn begin_poll(&mut self) -> Option<u64> {
        if !self.busy.is_empty() {
            return None;
        }
        self.generation += 1;
        Some(self.generation)
    }
    pub fn polled(
        &mut self,
        operation: u64,
        expected_instance: &str,
        result: Result<Envelope, String>,
    ) -> bool {
        if operation != self.generation {
            return false;
        }
        match result {
            Ok(e) if e.valid() && e.instance_id == expected_instance => {
                self.snapshot.requests = e.requests;
                self.snapshot.outcomes = e.outcomes;
                self.snapshot.error = None;
                self.snapshot.poll_revision += 1;
            }
            _ => {
                self.snapshot.requests.clear();
                self.snapshot.error = Some("ask_sync_failed".into());
            }
        }
        true
    }
    pub fn claim(
        &mut self,
        id: &str,
        instance: &str,
        policy: &str,
        chosen: &str,
    ) -> Result<(u64, Answer), String> {
        if !action(chosen) || self.busy.len() >= 8 || self.busy.contains_key(id) {
            return Err("ask_unavailable".into());
        }
        let request = self
            .snapshot
            .requests
            .iter()
            .find(|r| r.request_id == id && r.instance_id == instance && r.policy_id == policy)
            .ok_or("ask_unavailable")?;
        let answer = Answer {
            protocol_version: 5,
            instance_id: request.instance_id.clone(),
            policy_id: request.policy_id.clone(),
            request_id: id.into(),
            action: chosen.into(),
        };
        self.generation += 1;
        self.busy.insert(id.into(), self.generation);
        self.snapshot.busy_ids = self.busy.keys().cloned().collect();
        self.snapshot.last_answer = None;
        Ok((self.generation, answer))
    }
    pub fn answered(
        &mut self,
        operation: u64,
        answer: &Answer,
        result: Result<Reply, String>,
    ) -> bool {
        if self.busy.get(&answer.request_id) != Some(&operation) {
            return false;
        }
        self.busy.remove(&answer.request_id);
        self.snapshot.busy_ids = self.busy.keys().cloned().collect();
        match result {
            Ok(r)
                if r.protocol_version == 5
                    && r.request_id == answer.request_id
                    && r.instance_id == answer.instance_id
                    && r.policy_id == answer.policy_id
                    && [
                        "answered",
                        "expired",
                        "already_handled",
                        "not_found",
                        "stale",
                        "invalid",
                    ]
                    .contains(&r.status.as_str())
                    && (r.action.as_ref().is_none_or(|a| action(a)))
                    && (r.status != "answered"
                        || r.action.as_deref() == Some(answer.action.as_str())) =>
            {
                self.snapshot
                    .requests
                    .retain(|r| r.request_id != answer.request_id);
                self.snapshot.last_answer = Some(r.status);
                self.snapshot.error = None;
            }
            _ => {
                self.snapshot.error = Some("ask_answer_unknown".into());
            }
        }
        true
    }
}
fn helper(contents: &Path, flag: &str, input: Option<Vec<u8>>) -> Result<String, String> {
    if !crate::extension::package_status(contents).bridge_available {
        return Err("extension_not_packaged".into());
    }
    let binary = contents.join("MacOS/FlowGuardBridge");
    let (success, output) = crate::observer::run_bounded_input(
        binary.to_str().ok_or("invalid_bundle_path")?,
        &[flag],
        input,
    )?;
    if !success {
        return Err("ask_bridge_failed".into());
    }
    Ok(output)
}
pub fn poll(contents: &Path) -> Result<Envelope, String> {
    serde_json::from_str(&helper(contents, "--asks", None)?).map_err(|_| "ask_sync_failed".into())
}
pub fn answer(contents: &Path, answer: &Answer) -> Result<Reply, String> {
    let input = serde_json::to_vec(answer).map_err(|_| "ask_unavailable")?;
    serde_json::from_str(&helper(contents, "--answer", Some(input))?)
        .map_err(|_| "ask_answer_unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    const INSTANCE: &str = "11111111-1111-1111-1111-111111111111";
    const POLICY: &str = "22222222-2222-2222-2222-222222222222";
    const ID: &str = "33333333-3333-3333-3333-333333333333";
    fn request() -> Request {
        Request {
            request_id: ID.into(),
            instance_id: INSTANCE.into(),
            policy_id: POLICY.into(),
            rule_id: "rule".into(),
            identity: SigningIdentity {
                authority: "developer".into(),
                team_id: "ABCDE12345".into(),
                identifier: "com.test.app".into(),
            },
            app: "Test".into(),
            destination: "1.2.3.4".into(),
            port: 443,
            direction: "outbound".into(),
            timeout_action: "block".into(),
            remaining_seconds: 15,
        }
    }
    fn envelope() -> Envelope {
        Envelope {
            protocol_version: 5,
            instance_id: INSTANCE.into(),
            policy_id: POLICY.into(),
            requests: vec![request()],
            outcomes: vec![],
        }
    }
    fn loaded() -> Machine {
        let mut m = Machine::default();
        let op = m.begin_poll().unwrap();
        assert!(m.polled(op, INSTANCE, Ok(envelope())));
        m
    }
    fn reply(answer: &Answer) -> Reply {
        Reply {
            protocol_version: 5,
            instance_id: answer.instance_id.clone(),
            policy_id: answer.policy_id.clone(),
            request_id: answer.request_id.clone(),
            status: "answered".into(),
            action: Some(answer.action.clone()),
        }
    }
    #[test]
    fn one_window_claims_and_stale_poll_cannot_restore_it() {
        let mut m = loaded();
        let old = m.begin_poll().unwrap();
        let (op, answer) = m.claim(ID, INSTANCE, POLICY, "allow").unwrap();
        assert!(m.claim(ID, INSTANCE, POLICY, "block").is_err());
        assert!(m.begin_poll().is_none());
        assert!(!m.polled(old, INSTANCE, Ok(envelope())));
        assert!(m.answered(op, &answer, Ok(reply(&answer))));
        assert!(m.snapshot.requests.is_empty());
        assert_eq!(m.snapshot.last_answer.as_deref(), Some("answered"));
    }
    #[test]
    fn invalidation_and_mismatched_reply_never_claim_success() {
        let mut m = loaded();
        let (op, answer) = m.claim(ID, INSTANCE, POLICY, "block").unwrap();
        let mut wrong = reply(&answer);
        wrong.action = Some("allow".into());
        m.answered(op, &answer, Ok(wrong));
        assert!(m.snapshot.last_answer.is_none());
        assert!(m.snapshot.error.is_some());
        let (op, answer) = m.claim(ID, INSTANCE, POLICY, "block").unwrap();
        m.invalidate();
        assert!(!m.answered(op, &answer, Ok(reply(&answer))));
        assert!(m.snapshot.requests.is_empty());
    }
    #[test]
    fn bounds_and_restart_reject_untrusted_queue_snapshots() {
        let mut e = envelope();
        e.requests.push(request());
        assert!(!e.valid());
        e = envelope();
        e.requests[0].remaining_seconds = 16;
        assert!(!e.valid());
        e = envelope();
        e.requests[0].identity.team_id.clear();
        assert!(!e.valid());
        let mut m = loaded();
        let op = m.begin_poll().unwrap();
        assert!(m.polled(op, "44444444-4444-4444-4444-444444444444", Ok(envelope())));
        assert!(m.snapshot.requests.is_empty());
        assert!(m.snapshot.error.is_some());
    }
    #[test]
    fn expired_reply_is_not_a_user_allow_acknowledgement() {
        let mut m = loaded();
        let (op, answer) = m.claim(ID, INSTANCE, POLICY, "allow").unwrap();
        let mut expired = reply(&answer);
        expired.status = "expired".into();
        expired.action = Some("block".into());
        m.answered(op, &answer, Ok(expired));
        assert_eq!(m.snapshot.last_answer.as_deref(), Some("expired"));
        assert!(m.snapshot.requests.is_empty());
    }
}
