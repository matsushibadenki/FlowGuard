use crate::{model::Snapshot, state::SharedState, HostRuntime};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use tauri::Manager;
#[derive(Default)]
pub struct Planner {
    enabled: Option<bool>,
    seen: HashSet<String>,
    attempts: Vec<u64>,
}
impl Planner {
    pub fn tick(&mut self, s: &Snapshot, now: u64) -> Option<usize> {
        let current: HashSet<_> = s
            .asks
            .requests
            .iter()
            .map(|r| format!("{}:{}", r.instance_id, r.request_id))
            .collect();
        self.seen.retain(|key| current.contains(key));
        self.attempts
            .retain(|time| now.saturating_sub(*time) < 3600);
        let fresh = current.iter().any(|key| !self.seen.contains(key));
        if !s.document.preferences.ask_notifications
            || !s.extension.signed
            || s.extension.busy
            || !["ready", "applied", "stale"].contains(&s.extension.stage.as_str())
        {
            self.seen = current;
            return None;
        }
        if !fresh
            || self.attempts.len() >= 10
            || self
                .attempts
                .last()
                .is_some_and(|time| now.saturating_sub(*time) < 60)
        {
            return None;
        }
        self.seen = current;
        self.attempts.push(now);
        Some(self.seen.len())
    }
}
static REPLIES: OnceLock<Mutex<HashMap<u64, std::sync::mpsc::Sender<String>>>> = OnceLock::new();
static TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
extern "C" fn received(status: *const std::ffi::c_char, token: u64) {
    if let Some(sender) = REPLIES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&token)
    {
        let _ = sender.send(
            crate::native_runtime::callback_status(status).unwrap_or_else(|| "unknown".into()),
        );
    }
}
pub fn clear(app: &tauri::AppHandle, revision: u64) {
    if let Some(runtime) = &app.state::<HostRuntime>().0 {
        runtime.notification(3, revision, b"", received, 0);
    }
}
pub fn poll(app: &tauri::AppHandle, planner: &mut Planner) {
    let Ok(s) = app.state::<SharedState>().snapshot() else {
        return;
    };
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    let now = START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs();
    if !s.document.preferences.ask_notifications && planner.enabled != Some(false) {
        clear(app, s.revision);
    }
    planner.enabled = Some(s.document.preferences.ask_notifications);
    if let Some(count) = planner.tick(&s, now) {
        if let Some(runtime) = &app.state::<HostRuntime>().0 {
            let payload = serde_json::to_vec(
                &serde_json::json!({"count":count,"locale":s.document.preferences.locale}),
            )
            .unwrap();
            runtime.notification(2, s.revision, &payload, received, 0);
        }
    }
}
#[tauri::command]
pub async fn notification_permission(
    app: tauri::AppHandle,
    request: bool,
) -> Result<String, String> {
    let runtime = app
        .state::<HostRuntime>()
        .0
        .clone()
        .ok_or("native_runtime_unavailable")?;
    tauri::async_runtime::spawn_blocking(move || {
        let token = TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, rx) = std::sync::mpsc::channel();
        {
            let mut replies = REPLIES.get_or_init(Default::default).lock().unwrap();
            if replies.len() >= 8 {
                return "unknown".into();
            }
            replies.insert(token, tx);
        }
        runtime.notification(if request { 1 } else { 0 }, 0, b"", received, token);
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap_or_else(|_| "unknown".into());
        REPLIES.get().unwrap().lock().unwrap().remove(&token);
        result
    })
    .await
    .map_err(|_| "notification_unavailable".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        let dir = tempfile::tempdir().unwrap();
        let state = SharedState::load(dir.path().join("doc.json")).unwrap();
        let mut s = state.snapshot().unwrap();
        s.document.preferences.ask_notifications = true;
        s.extension.signed = true;
        s.extension.stage = "ready".into();
        s
    }
    fn request(id: usize) -> crate::asks::Request {
        serde_json::from_value(serde_json::json!({"requestId":format!("r{id}"),"instanceId":"boot","policyId":"policy","ruleId":"r","identity":{"authority":"apple","teamId":"","identifier":"com.apple.mDNSResponder"},"app":"Private App","destination":"1.1.1.1","port":53,"direction":"outbound","timeoutAction":"block","remainingSeconds":15})).unwrap()
    }
    #[test]
    fn deduplicates_limits_batches_and_requires_opt_in() {
        let mut s = fixture();
        let mut planner = Planner::default();
        s.asks.requests = vec![request(1)];
        assert_eq!(planner.tick(&s, 0), Some(1));
        assert_eq!(planner.tick(&s, 1), None);
        s.asks.requests.push(request(2));
        assert_eq!(planner.tick(&s, 59), None);
        assert_eq!(planner.tick(&s, 60), Some(2));
        s.document.preferences.ask_notifications = false;
        s.asks.requests.push(request(3));
        assert_eq!(planner.tick(&s, 120), None);
        s.document.preferences.ask_notifications = true;
        assert_eq!(planner.tick(&s, 121), None);
        for i in 4..12 {
            s.asks.requests.push(request(i));
            assert!(planner.tick(&s, i as u64 * 60).is_some());
        }
        s.asks.requests.push(request(12));
        assert_eq!(planner.tick(&s, 2000), None);
        assert!(planner.tick(&s, 3600).is_some());
        assert!(planner.seen.len() <= 128);
    }
    #[test]
    fn unsigned_busy_and_gone_requests_never_alert() {
        let mut s = fixture();
        let mut planner = Planner::default();
        s.asks.requests = vec![request(1)];
        s.extension.signed = false;
        assert_eq!(planner.tick(&s, 0), None);
        s.extension.signed = true;
        s.extension.busy = true;
        s.asks.requests.push(request(2));
        assert_eq!(planner.tick(&s, 60), None);
        s.extension.busy = false;
        s.asks.requests.clear();
        assert_eq!(planner.tick(&s, 120), None);
    }
    #[test]
    fn opt_out_default_and_no_requests_never_notify() {
        let dir = tempfile::tempdir().unwrap();
        let state = SharedState::load(dir.path().join("doc.json")).unwrap();
        let s = state.snapshot().unwrap();
        assert!(!s.document.preferences.ask_notifications);
        assert_eq!(Planner::default().tick(&s, 0), None);
    }
}
