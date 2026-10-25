use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{path::Component, sync::Arc, time::Duration};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
fn configure_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
const ENDPOINT: &str =
    "https://github.com/matsushibadenki/FlowGuard/releases/latest/download/latest.json";
const MAX_DOWNLOAD: usize = 200 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryEvent {
    pub at: u64,
    pub status: String,
    pub from: String,
    pub to: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub channel: String,
    pub history: Vec<HistoryEvent>,
    pub history_persistent: bool,
    pub stage: String,
    pub operation: u64,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub checked_at: Option<u64>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<String>,
    pub development: bool,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            channel: "stable".into(),
            history: vec![],
            history_persistent: true,
            stage: "idle".into(),
            operation: 0,
            current_version: env!("CARGO_PKG_VERSION").into(),
            version: None,
            notes: None,
            checked_at: None,
            downloaded: 0,
            total: None,
            error: None,
            development: cfg!(debug_assertions),
        }
    }
}
#[derive(Default)]
pub struct Machine {
    pub snapshot: Snapshot,
    pub update: Option<Update>,
    pub bytes: Option<Arc<Vec<u8>>>,
}
impl Machine {
    pub fn with_history(path: &std::path::Path, channel: &str) -> Self {
        let mut m = Self::default();
        m.snapshot.channel = channel.into();
        if path.exists() {
            let parsed = std::fs::symlink_metadata(path)
                .ok()
                .filter(|v| v.is_file() && !v.file_type().is_symlink() && v.len() <= 65536)
                .and_then(|_| std::fs::read(path).ok())
                .and_then(|b| serde_json::from_slice::<Vec<HistoryEvent>>(&b).ok());
            if let Some(events) = parsed.filter(|rows| {
                rows.len() <= 20
                    && rows.iter().all(|e| {
                        semver::Version::parse(&e.from).is_ok()
                            && e.to
                                .as_ref()
                                .is_none_or(|v| semver::Version::parse(v).is_ok())
                            && [
                                "ready",
                                "installing",
                                "failed",
                                "cancelled",
                                "launched_version",
                            ]
                            .contains(&e.status.as_str())
                    })
            }) {
                m.snapshot.history = events;
            } else {
                m.snapshot.history_persistent = false;
            }
        }
        if let Some(event) = m
            .snapshot
            .history
            .iter()
            .find(|e| {
                e.status == "installing" && e.to.as_deref() == Some(env!("CARGO_PKG_VERSION"))
            })
            .cloned()
        {
            if !m
                .snapshot
                .history
                .iter()
                .any(|e| e.status == "launched_version" && e.to == event.to && e.at >= event.at)
            {
                m.snapshot.history.insert(
                    0,
                    HistoryEvent {
                        at: crate::model::now(),
                        status: "launched_version".into(),
                        from: event.from,
                        to: event.to,
                    },
                );
                m.persist_history(path);
            }
        }
        m
    }
    pub fn record_history(&mut self, path: &std::path::Path) {
        if !["ready", "installing", "failed", "cancelled"].contains(&self.snapshot.stage.as_str()) {
            return;
        }
        self.snapshot.history.insert(
            0,
            HistoryEvent {
                at: crate::model::now(),
                status: self.snapshot.stage.clone(),
                from: self.snapshot.current_version.clone(),
                to: self.snapshot.version.clone(),
            },
        );
        self.snapshot.history.truncate(20);
        self.persist_history(path);
    }
    fn persist_history(&mut self, path: &std::path::Path) {
        let result = (|| -> Result<(), std::io::Error> {
            use std::io::Write;
            let tmp = path.with_extension("json.tmp");
            if tmp.exists() || tmp.is_symlink() {
                std::fs::remove_file(&tmp)?;
            }
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            }
            let mut file = options.open(&tmp)?;
            file.write_all(&serde_json::to_vec(&self.snapshot.history)?)?;
            file.sync_all()?;
            std::fs::rename(tmp, path)?;
            Ok(())
        })();
        self.snapshot.history_persistent = result.is_ok();
    }
    pub fn check(&mut self) -> Result<u64, String> {
        if ["checking", "downloading", "ready", "installing"]
            .contains(&self.snapshot.stage.as_str())
        {
            return Err("updater_busy".into());
        }
        let operation = self.snapshot.operation + 1;
        self.snapshot = Snapshot {
            stage: "checking".into(),
            operation,
            channel: self.snapshot.channel.clone(),
            history: self.snapshot.history.clone(),
            history_persistent: self.snapshot.history_persistent,
            ..Default::default()
        };
        self.update = None;
        self.bytes = None;
        Ok(operation)
    }
    fn current(&self, op: u64, stage: &str) -> Result<(), String> {
        if self.snapshot.operation != op || self.snapshot.stage != stage {
            return Err("stale_update".into());
        }
        Ok(())
    }
    pub fn found(&mut self, op: u64, update: Option<Update>) -> Result<(), String> {
        self.current(op, "checking")?;
        self.snapshot.checked_at = Some(crate::model::now());
        if let Some(update) = update {
            validate_channel_release(
                &update.current_version,
                &update.version,
                update.download_url.as_str(),
                &self.snapshot.channel,
            )?;
            self.snapshot.version = Some(update.version.clone());
            self.snapshot.notes = update.body.as_ref().map(|v| {
                v.chars()
                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
                    .take(8192)
                    .collect()
            });
            self.update = Some(update);
            self.snapshot.stage = "available".into();
        } else {
            self.snapshot.stage = "up_to_date".into();
        }
        Ok(())
    }
    pub fn download(&mut self) -> Result<(u64, Update), String> {
        if self.snapshot.stage != "available" {
            return Err("update_unavailable".into());
        }
        let update = self.update.clone().ok_or("update_unavailable")?;
        self.snapshot.stage = "downloading".into();
        self.snapshot.error = None;
        self.snapshot.downloaded = 0;
        self.snapshot.total = None;
        Ok((self.snapshot.operation, update))
    }
    pub fn progress(&mut self, op: u64, downloaded: u64, total: Option<u64>) -> Result<(), String> {
        self.current(op, "downloading")?;
        if downloaded > MAX_DOWNLOAD as u64 || total.is_some_and(|n| n > MAX_DOWNLOAD as u64) {
            return Err("update_too_large".into());
        }
        self.snapshot.downloaded = downloaded;
        self.snapshot.total = total;
        Ok(())
    }
    pub fn ready(&mut self, op: u64, bytes: Vec<u8>) -> Result<(), String> {
        self.current(op, "downloading")?;
        self.bytes = Some(Arc::new(bytes));
        self.snapshot.stage = "ready".into();
        Ok(())
    }
    pub fn fail(&mut self, op: u64, error: &str) -> Result<(), String> {
        if self.snapshot.operation != op
            || !["checking", "downloading", "installing"].contains(&self.snapshot.stage.as_str())
        {
            return Err("stale_update".into());
        }
        self.bytes = None;
        self.update = None;
        self.snapshot.stage = "failed".into();
        self.snapshot.error = Some(error.into());
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), String> {
        if !["checking", "downloading", "available", "ready", "failed"]
            .contains(&self.snapshot.stage.as_str())
        {
            return Err("update_not_cancellable".into());
        }
        self.snapshot.operation += 1;
        self.snapshot.stage = "cancelled".into();
        self.snapshot.downloaded = 0;
        self.snapshot.total = None;
        self.snapshot.error = None;
        self.bytes = None;
        self.update = None;
        Ok(())
    }
    pub fn install(&mut self) -> Result<(u64, Update, Arc<Vec<u8>>), String> {
        self.current(self.snapshot.operation, "ready")?;
        let update = self.update.clone().ok_or("update_unavailable")?;
        let bytes = self.bytes.clone().ok_or("update_unavailable")?;
        self.snapshot.stage = "installing".into();
        Ok((self.snapshot.operation, update, bytes))
    }
}
pub fn can_install(s: &crate::model::Snapshot) -> bool {
    s.updates.stage == "ready"
        && !s.updates.development
        && !s
            .security
            .router_sources
            .iter()
            .any(|source| source.stage == "collecting")
        && !["starting", "listening", "stopping"].contains(&s.syslog.stage.as_str())
        && !s.destination_names.busy
        && !s.flow_names.busy
        && !s.temporary_protection.busy
        && s.temporary_protection.rows.is_empty()
        && !s.firewall_control.busy
        && !s.extension.busy
        && ![
            "approval_required",
            "restart_required",
            "configuration_saved",
        ]
        .contains(&s.extension.stage.as_str())
        && [&s.discovery.stage, &s.ports.stage, &s.services.stage]
            .iter()
            .all(|stage| !["running", "cancelling"].contains(&stage.as_str()))
}
fn validate_release(current: &str, version: &str, url: &str) -> Result<(), String> {
    validate_channel_release(current, version, url, "stable")
}
fn validate_channel_release(
    current: &str,
    version: &str,
    url: &str,
    channel: &str,
) -> Result<(), String> {
    let current = semver::Version::parse(current).map_err(|_| "invalid_version")?;
    let next = semver::Version::parse(version).map_err(|_| "invalid_version")?;
    let url = reqwest::Url::parse(url).map_err(|_| "invalid_download_url")?;
    if !["stable", "beta"].contains(&channel)
        || next <= current
        || channel == "stable" && !next.pre.is_empty()
    {
        return Err("invalid_version".into());
    }
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || !url
            .path()
            .starts_with("/matsushibadenki/FlowGuard/releases/download/")
        || !url.path().ends_with(".app.tar.gz")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_download_url".into());
    }
    Ok(())
}
fn public_key() -> String {
    serde_json::from_str::<serde_json::Value>(include_str!("../tauri.conf.json")).unwrap()
        ["plugins"]["updater"]["pubkey"]
        .as_str()
        .unwrap()
        .into()
}
fn signature(data: &[u8], encoded: &str, key: &str, version: &str) -> Result<(), String> {
    let text = |v: &str| {
        STANDARD
            .decode(v)
            .ok()
            .and_then(|v| String::from_utf8(v).ok())
    };
    let public = minisign_verify::PublicKey::decode(&text(key).ok_or("signature_invalid")?)
        .map_err(|_| "signature_invalid")?;
    let signature = minisign_verify::Signature::decode(&text(encoded).ok_or("signature_invalid")?)
        .map_err(|_| "signature_invalid")?;
    public
        .verify(data, &signature, false)
        .map_err(|_| "signature_invalid")?;
    if signature
        .trusted_comment()
        .split('\t')
        .filter_map(|v| v.strip_prefix("version:"))
        .collect::<Vec<_>>()
        != vec![version]
    {
        return Err("signature_version".into());
    }
    Ok(())
}
fn trusted_signature(
    bytes: &[u8],
    encoded: &str,
    version: &str,
    channel: &str,
) -> Result<(), String> {
    let keys = crate::update_trust::keys(
        include_str!("../update-trust.json"),
        &public_key(),
        version,
        channel,
    )?;
    let mut error = "signature_invalid".to_owned();
    for key in keys {
        match signature(bytes, encoded, &key, version) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e == "signature_version" {
                    error = e;
                }
            }
        }
    }
    Err(error)
}
// Validate before invoking the plugin installer, which strips the .app root during extraction.
fn archive_arch(data: &[u8], arch: &str) -> Result<(), String> {
    use std::io::Read;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(data));
    let mut found = false;
    for entry in tar.entries().map_err(|_| "invalid_archive")? {
        let entry = entry.map_err(|_| "invalid_archive")?;
        if entry.path().map_err(|_| "invalid_archive")?.as_ref()
            == std::path::Path::new("FlowGuard.app/Contents/MacOS/flowguard")
        {
            let size = entry.size();
            let mut header = vec![];
            entry
                .take(520)
                .read_to_end(&mut header)
                .map_err(|_| "invalid_archive")?;
            if !crate::update_arch::contains(&header, size, arch) {
                return Err("invalid_architecture".into());
            }
            found = true;
        }
    }
    if found {
        Ok(())
    } else {
        Err("invalid_archive".into())
    }
}
fn archive(data: &[u8], version: &str) -> Result<(), String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(data));
    let mut paths = std::collections::HashSet::new();
    let mut count = 0;
    let mut total = 0u64;
    let mut info = false;
    let mut executable = false;
    for entry in archive.entries().map_err(|_| "invalid_archive")? {
        let mut entry = entry.map_err(|_| "invalid_archive")?;
        count += 1;
        total = total.checked_add(entry.size()).ok_or("invalid_archive")?;
        if count > 10000 || total > 512 * 1024 * 1024 {
            return Err("invalid_archive".into());
        }
        let path = entry.path().map_err(|_| "invalid_archive")?.into_owned();
        if !paths.insert(path.clone()) {
            return Err("invalid_archive".into());
        }
        let components = path.components().collect::<Vec<_>>();
        if components.first() != Some(&Component::Normal(std::ffi::OsStr::new("FlowGuard.app")))
            || components
                .iter()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("invalid_archive".into());
        }
        let kind = entry.header().entry_type();
        if !kind.is_dir() && !kind.is_file() {
            return Err("invalid_archive".into());
        }
        if path == std::path::Path::new("FlowGuard.app/Contents/Info.plist") {
            if !kind.is_file() || entry.size() > 65536 {
                return Err("invalid_archive".into());
            }
            let p = plist::Value::from_reader(std::io::Cursor::new({
                use std::io::Read;
                let mut bytes = Vec::new();
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|_| "invalid_archive")?;
                bytes
            }))
            .map_err(|_| "invalid_archive")?;
            let p = p.as_dictionary().ok_or("invalid_archive")?;
            if p.get("CFBundleIdentifier").and_then(|v| v.as_string())
                != Some("com.flowguard.desktop")
                || p.get("CFBundleShortVersionString")
                    .and_then(|v| v.as_string())
                    != Some(version)
                || p.get("CFBundleExecutable").and_then(|v| v.as_string()) != Some("flowguard")
            {
                return Err("invalid_archive".into());
            }
            info = true;
        }
        if path == std::path::Path::new("FlowGuard.app/Contents/MacOS/flowguard") {
            if !kind.is_file()
                || entry.size() == 0
                || entry.header().mode().map_err(|_| "invalid_archive")? & 0o111 == 0
            {
                return Err("invalid_archive".into());
            }
            executable = true;
        }
    }
    if !info || !executable {
        return Err("invalid_archive".into());
    }
    Ok(())
}
fn publish<T>(
    app: &tauri::AppHandle,
    change: impl FnOnce(&mut Machine) -> Result<T, String>,
) -> Result<T, String> {
    let (value, snapshot) = app
        .state::<crate::state::SharedState>()
        .update_change(change)?;
    let _ = app.emit("state-changed", snapshot);
    Ok(value)
}
fn cancelled(app: &tauri::AppHandle, op: u64) -> bool {
    app.state::<crate::state::SharedState>()
        .inner
        .lock()
        .map(|s| s.updates.snapshot.operation != op || s.updates.snapshot.stage != "downloading")
        .unwrap_or(true)
}

pub async fn check(
    app: tauri::AppHandle,
    automatic: bool,
) -> Result<crate::model::Snapshot, String> {
    configure_crypto();
    if automatic {
        let s = app.state::<crate::state::SharedState>().snapshot()?;
        if !s.document.preferences.auto_updates
            || s.updates.development
            || ["checking", "downloading", "ready", "installing"]
                .contains(&s.updates.stage.as_str())
        {
            return Ok(s);
        }
    }
    let channel = app
        .state::<crate::state::SharedState>()
        .snapshot()?
        .document
        .preferences
        .update_channel;
    let op = publish(&app, |m| {
        m.snapshot.channel = channel.clone();
        m.check()
    })?;
    let result = async {
        let updater = app
            .updater_builder()
            .endpoints(vec![if channel == "beta" {
                "https://github.com/matsushibadenki/FlowGuard/releases/download/beta/latest.json"
            } else {
                ENDPOINT
            }
            .parse()
            .unwrap()])
            .map_err(|_| "check_failed")?
            .timeout(Duration::from_secs(20))
            .configure_client(|builder| {
                builder
                    .https_only(true)
                    .redirect(reqwest::redirect::Policy::limited(3))
            })
            .build()
            .map_err(|_| "check_failed".to_string())?;
        let update = updater.check().await.map_err(|e| {
            if matches!(e, tauri_plugin_updater::Error::ReleaseNotFound) {
                "release_unavailable".to_string()
            } else {
                "check_failed".to_string()
            }
        })?;
        publish(&app, |m| m.found(op, update))
    }
    .await;
    if let Err(error) = result {
        let _ = publish(&app, |m| m.fail(op, &error));
        return app.state::<crate::state::SharedState>().snapshot();
    }
    if automatic
        && app
            .state::<crate::state::SharedState>()
            .snapshot()?
            .document
            .preferences
            .auto_updates
        && app
            .state::<crate::state::SharedState>()
            .snapshot()?
            .updates
            .stage
            == "available"
    {
        let _ = download(app.clone()).await;
    }
    app.state::<crate::state::SharedState>().snapshot()
}
async fn fetch_parts(
    client: &reqwest::Client,
    url: reqwest::Url,
    cache: &mut crate::update_cache::Cache,
    bytes: &mut Vec<u8>,
    cancelled: impl Fn() -> bool,
    progress: impl Fn(u64, Option<u64>) -> Result<(), String>,
) -> Result<(), String> {
    let offset = bytes.len();
    let mut request = client.get(url);
    if offset > 0 {
        request = request
            .header(reqwest::header::RANGE, format!("bytes={offset}-"))
            .header(
                reqwest::header::IF_RANGE,
                cache.etag.clone().ok_or("update_cache_failed")?,
            );
    }
    let mut send = Box::pin(request.send());
    let mut response = loop {
        if cancelled() {
            return Err("cancelled".into());
        }
        match tokio::time::timeout(Duration::from_secs(1), send.as_mut()).await {
            Ok(result) => break result.map_err(|_| "download_failed")?,
            Err(_) => continue,
        }
    };
    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            cache.reset(None)?;
        }
        return Err("download_failed".into());
    }
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let total = if offset > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        if etag != cache.etag {
            return Err("download_failed".into());
        }
        let total = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| crate::update_cache::range_total(v, offset))
            .ok_or("download_failed")?;
        Some(total as u64)
    } else {
        if response.status() != reqwest::StatusCode::OK {
            return Err("download_failed".into());
        }
        bytes.clear();
        cache.reset(etag)?;
        response.content_length()
    };
    if total.is_some_and(|n| n > MAX_DOWNLOAD as u64) {
        return Err("update_too_large".into());
    }
    let mut last_progress = std::time::Instant::now();
    loop {
        if cancelled() {
            return Err("cancelled".into());
        }
        let chunk = match tokio::time::timeout(Duration::from_secs(1), response.chunk()).await {
            Ok(result) => result.map_err(|_| "download_failed")?,
            Err(_) => continue,
        };
        let Some(chunk) = chunk else {
            break;
        };
        if cancelled() {
            return Err("cancelled".into());
        }
        if bytes.len() + chunk.len() > MAX_DOWNLOAD {
            return Err("update_too_large".into());
        }
        cache.append(&chunk, bytes.len())?;
        bytes.extend_from_slice(&chunk);
        if last_progress.elapsed() >= Duration::from_millis(250) {
            progress(bytes.len() as u64, total)?;
            last_progress = std::time::Instant::now();
        }
    }
    progress(bytes.len() as u64, total)?;
    if total.is_some_and(|n| n != bytes.len() as u64) {
        return Err("download_failed".into());
    }
    Ok(())
}
pub async fn download(app: tauri::AppHandle) -> Result<crate::model::Snapshot, String> {
    configure_crypto();
    let (op, update) = publish(&app, Machine::download)?;
    let channel = app
        .state::<crate::state::SharedState>()
        .snapshot()?
        .updates
        .channel;
    let result = async {
        validate_channel_release(
            &update.current_version,
            &update.version,
            update.download_url.as_str(),
            &channel,
        )?;
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let url = attempt.url();
                if attempt.previous().len() < 5
                    && url.scheme() == "https"
                    && matches!(
                        url.host_str(),
                        Some(
                            "github.com"
                                | "release-assets.githubusercontent.com"
                                | "objects.githubusercontent.com"
                        )
                    )
                {
                    attempt.follow()
                } else {
                    attempt.error("invalid update redirect")
                }
            }))
            .build()
            .map_err(|_| "download_failed")?;
        let cache_root = app
            .path()
            .app_cache_dir()
            .map_err(|_| "update_cache_failed")?
            .join("updater-v1");
        let (mut cache, mut bytes) = crate::update_cache::Cache::open(
            cache_root,
            crate::update_cache::Identity {
                version: update.version.clone(),
                url: update.download_url.to_string(),
                signature: update.signature.clone(),
                architecture: std::env::consts::ARCH.into(),
            },
        )?;
        if !bytes.is_empty()
            && trusted_signature(&bytes, &update.signature, &update.version, &channel).is_ok()
        {
            if let Err(error) = archive(&bytes, &update.version)
                .and_then(|_| archive_arch(&bytes, std::env::consts::ARCH))
            {
                cache.reset(None)?;
                return Err(error);
            }
            publish(&app, |m| {
                m.progress(op, bytes.len() as u64, Some(bytes.len() as u64))
            })?;
            return publish(&app, |m| m.ready(op, bytes));
        }
        fetch_parts(
            &client,
            update.download_url.clone(),
            &mut cache,
            &mut bytes,
            || cancelled(&app, op),
            |n, total| publish(&app, |m| m.progress(op, n, total)).map(|_| ()),
        )
        .await?;
        if let Err(error) = trusted_signature(&bytes, &update.signature, &update.version, &channel)
            .and_then(|_| archive(&bytes, &update.version))
            .and_then(|_| archive_arch(&bytes, std::env::consts::ARCH))
        {
            cache.reset(None)?;
            return Err(error);
        }
        publish(&app, |m| m.ready(op, bytes))
    }
    .await;
    if let Err(error) = result {
        let _ = publish(&app, |m| m.fail(op, &error));
    }
    app.state::<crate::state::SharedState>().snapshot()
}
pub fn install_with_backup(
    update: Update,
    bytes: Arc<Vec<u8>>,
    bundle: &std::path::Path,
) -> Result<(), String> {
    let identity = |path: &std::path::Path| -> Option<String> {
        let output = std::process::Command::new("/usr/bin/codesign")
            .args(["-d", "--verbose=4"])
            .arg(path)
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .find_map(|line| line.strip_prefix("TeamIdentifier="))
            .filter(|team| {
                team.len() == 10
                    && team
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            })
            .map(str::to_string)
    };
    let team = identity(bundle).ok_or("install_failed")?;
    let verify = || {
        std::process::Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(bundle)
            .status()
            .is_ok_and(|s| s.success())
            && identity(bundle).as_deref() == Some(team.as_str())
            && std::process::Command::new("/usr/sbin/spctl")
                .args(["--assess", "--type", "execute"])
                .arg(bundle)
                .status()
                .is_ok_and(|s| s.success())
    };
    replace_with_backup(
        bundle,
        || update.install(bytes.as_slice()).map_err(|_| ()),
        verify,
    )
}
fn replace_with_backup(
    bundle: &std::path::Path,
    install: impl FnOnce() -> Result<(), ()>,
    verify: impl Fn() -> bool,
) -> Result<(), String> {
    let parent = bundle.parent().ok_or("install_failed")?;
    let backup = parent.join(format!(".FlowGuard-update-backup-{}", std::process::id()));
    if backup.exists() {
        return Err("install_failed".into());
    }
    let copy = |source: &std::path::Path, target: &std::path::Path| {
        std::process::Command::new("/usr/bin/ditto")
            .arg(source)
            .arg(target)
            .status()
            .is_ok_and(|s| s.success())
    };
    if !copy(bundle, &backup) {
        return Err("install_failed".into());
    }
    if install().is_err() || !verify() {
        let _ = std::fs::remove_dir_all(bundle);
        if !copy(&backup, bundle) {
            return Err("rollback_failed".into());
        }
        let _ = std::fs::remove_dir_all(&backup);
        return Err("install_failed".into());
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

pub fn schedule(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            let _ = check(app.clone(), true).await;
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "local HTTP range server only; no external network or installer"]
    fn resumable_http_uses_strong_validator_and_rechecks_complete_signature() {
        configure_crypto();
        use std::io::{Read, Write};
        let data = include_bytes!("../tests/fixtures/updater-valid.app.tar.gz").to_vec();
        let split = data.len() / 2;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/artifact", listener.local_addr().unwrap());
        let copy = data.clone();
        let server = std::thread::spawn(move || {
            for pass in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = vec![];
                while !request.ends_with(b"\r\n\r\n") {
                    let mut b = [0];
                    socket.read_exact(&mut b).unwrap();
                    request.push(b[0]);
                    if request.len() > 8192 {
                        panic!("request limit");
                    }
                }
                let text = String::from_utf8(request).unwrap().to_ascii_lowercase();
                if pass == 0 {
                    assert!(!text.contains("range:"));
                    write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"fixed\"\r\nConnection: close\r\n\r\n",copy.len()).unwrap();
                    socket.write_all(&copy[..split]).unwrap();
                    socket.flush().unwrap();
                    std::thread::sleep(Duration::from_millis(100));
                } else {
                    assert!(text.contains(&format!("range: bytes={split}-")));
                    assert!(text.contains("if-range: \"fixed\""));
                    write!(socket,"HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nETag: \"fixed\"\r\nConnection: close\r\n\r\n",copy.len()-split,split,copy.len()-1,copy.len()).unwrap();
                    socket.write_all(&copy[split..]).unwrap();
                }
            }
        });
        let root = tempfile::tempdir().unwrap();
        let identity = crate::update_cache::Identity {
            version: "0.2.0".into(),
            url: url.clone(),
            signature: include_str!("../tests/fixtures/updater-valid.app.tar.gz.sig")
                .trim()
                .into(),
            architecture: "test".into(),
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let (mut cache, mut bytes) =
            crate::update_cache::Cache::open(root.path().join("cache"), identity.clone()).unwrap();
        let first = tauri::async_runtime::block_on(fetch_parts(
            &client,
            url.parse().unwrap(),
            &mut cache,
            &mut bytes,
            || false,
            |_, _| Ok(()),
        ));
        assert!(first.is_err());
        assert_eq!(bytes.len(), split);
        drop(cache);
        let (mut cache, mut bytes) =
            crate::update_cache::Cache::open(root.path().join("cache"), identity).unwrap();
        assert_eq!(bytes.len(), split);
        tauri::async_runtime::block_on(fetch_parts(
            &client,
            url.parse().unwrap(),
            &mut cache,
            &mut bytes,
            || false,
            |_, _| Ok(()),
        ))
        .unwrap();
        assert_eq!(bytes, data);
        signature(
            &bytes,
            include_str!("../tests/fixtures/updater-valid.app.tar.gz.sig").trim(),
            include_str!("../tests/fixtures/updater-test.pub").trim(),
            "0.2.0",
        )
        .unwrap();
        server.join().unwrap();
    }
    #[test]
    fn channel_validation_and_persisted_history_never_claim_install_before_launch() {
        let url="https://github.com/matsushibadenki/FlowGuard/releases/download/v0.2.0/FlowGuard.app.tar.gz";
        assert!(validate_channel_release("0.1.0", "0.2.0-beta.1", url, "beta").is_ok());
        assert!(validate_channel_release("0.1.0", "0.2.0-beta.1", url, "stable").is_err());
        assert!(validate_channel_release("0.2.0", "0.1.0", url, "beta").is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut m = Machine::default();
        m.snapshot.stage = "installing".into();
        m.snapshot.version = Some(env!("CARGO_PKG_VERSION").into());
        m.record_history(&path);
        assert_eq!(m.snapshot.history[0].status, "installing");
        let loaded = Machine::with_history(&path, "beta");
        assert_eq!(loaded.snapshot.channel, "beta");
        assert_eq!(loaded.snapshot.history[0].status, "launched_version");
        let loaded_again = Machine::with_history(&path, "stable");
        assert_eq!(loaded_again.snapshot.history.len(), 2);
        m.snapshot.stage = "ready".into();
        m.record_history(std::path::Path::new("/dev/null/updates.json"));
        assert!(!m.snapshot.history_persistent);
        assert_eq!(m.snapshot.stage, "ready");
    }
    #[test]
    fn signature_requires_unchanged_bytes_and_signed_version() {
        let data = include_bytes!("../tests/fixtures/updater-message.txt");
        let signed = include_str!("../tests/fixtures/updater-message.txt.sig").trim();
        assert!(signature(data, signed, &public_key(), "0.1.0").is_ok());
        assert!(signature(b"tampered", signed, &public_key(), "0.1.0").is_err());
        assert_eq!(
            signature(data, signed, &public_key(), "99.0.0").unwrap_err(),
            "signature_version"
        );
        assert!(signature(data, "invalid", &public_key(), "0.1.0").is_err());
    }
    #[test]
    fn release_targets_and_monotonic_versions() {
        let url="https://github.com/matsushibadenki/FlowGuard/releases/download/v0.2.0/FlowGuard_aarch64.app.tar.gz";
        assert!(validate_release("0.1.0", "0.2.0", url).is_ok());
        for v in ["0.1.0", "0.0.9", "0.2.0-beta.1", "bad"] {
            assert!(validate_release("0.1.0", v, url).is_err());
        }
        for url in [
            "http://github.com/matsushibadenki/FlowGuard/releases/download/v0.2.0/a.app.tar.gz",
            "https://evil.example/a.app.tar.gz",
            "https://github.com/other/repo/releases/download/a.app.tar.gz",
        ] {
            assert!(validate_release("0.1.0", "0.2.0", url).is_err());
        }
    }
    #[test]
    fn cancellation_and_late_callbacks() {
        let mut m = Machine::default();
        let op = m.check().unwrap();
        assert!(m.check().is_err());
        m.cancel().unwrap();
        assert!(m.found(op, None).is_err());
        let op = m.check().unwrap();
        m.found(op, None).unwrap();
        assert_eq!(m.snapshot.stage, "up_to_date");
        m.snapshot.stage = "downloading".into();
        assert!(m.progress(op, MAX_DOWNLOAD as u64 + 1, None).is_err());
        m.cancel().unwrap();
        assert!(m.ready(op, vec![1]).is_err());
        assert!(m.bytes.is_none());
    }
    #[test]
    fn archive_identity_version_and_paths() {
        fn make(version: &str, path: &str) -> Vec<u8> {
            let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
                Vec::new(),
                flate2::Compression::default(),
            ));
            let plist=format!("<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>com.flowguard.desktop</string><key>CFBundleExecutable</key><string>flowguard</string><key>CFBundleShortVersionString</key><string>{version}</string></dict></plist>");
            for (name, data, mode) in [
                ("FlowGuard.app/Contents/Info.plist", plist.as_bytes(), 0o644),
                (path, b"test executable".as_slice(), 0o755),
            ] {
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(mode);
                h.set_cksum();
                tar.append_data(&mut h, name, data).unwrap();
            }
            tar.into_inner().unwrap().finish().unwrap()
        }
        let data = make("0.2.0", "FlowGuard.app/Contents/MacOS/flowguard");
        assert!(archive(&data, "0.2.0").is_ok());
        assert!(archive(&data, "99.0.0").is_err());
        assert!(archive(
            &make("0.2.0", "Other.app/Contents/MacOS/flowguard"),
            "0.2.0"
        )
        .is_err());
        assert!(archive(b"invalid", "0.2.0").is_err());
    }
    #[test]
    #[ignore = "HTTP loopback test server and disposable app only"]
    fn real_plugin_download_signature_and_disposable_install() {
        use std::io::{Read, Write};
        let data = include_bytes!("../tests/fixtures/updater-valid.app.tar.gz").to_vec();
        let signed = include_str!("../tests/fixtures/updater-valid.app.tar.gz.sig").trim();
        let key = include_str!("../tests/fixtures/updater-test.pub").trim();
        assert_ne!(key, public_key());
        for tampered in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "flowguard-updater-fixture-{}-{}",
                std::process::id(),
                tampered
            ));
            let bundle = root.join("FlowGuard.app");
            std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
            std::fs::write(bundle.join("Contents/MacOS/flowguard"), b"old fixture").unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let url = format!("http://{address}/update");
            let archive_url = format!("http://{address}/fixture.app.tar.gz");
            let target = tauri_plugin_updater::target().unwrap();
            let manifest=serde_json::json!({"version":"0.2.0","notes":"Local fixture","platforms":{target:{"url":archive_url,"signature":signed}}}).to_string().into_bytes();
            let mut artifact = data.clone();
            if tampered {
                artifact[0] ^= 1;
            }
            let server = std::thread::spawn(move || {
                for body in [manifest, artifact] {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut buf = [0; 2048];
                    stream.read(&mut buf).unwrap();
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                    stream.write_all(&body).unwrap();
                }
            });
            let mut context = tauri::test::mock_context(tauri::test::noop_assets());
            context.config_mut().plugins.0.insert("updater".into(),serde_json::json!({"dangerousInsecureTransportProtocol":true,"pubkey":key,"endpoints":[url]}));
            let app = tauri::test::mock_builder()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .build(context)
                .unwrap();
            let updater = app
                .updater_builder()
                .executable_path(bundle.join("Contents/MacOS/flowguard"))
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap();
            let update = tauri::async_runtime::block_on(updater.check())
                .unwrap()
                .unwrap();
            let bytes = tauri::async_runtime::block_on(update.download(|_, _| {}, || {}));
            if tampered {
                assert!(bytes.is_err());
                assert_eq!(
                    std::fs::read(bundle.join("Contents/MacOS/flowguard")).unwrap(),
                    b"old fixture"
                );
            } else {
                let bytes = bytes.unwrap();
                signature(&bytes, signed, key, "0.2.0").unwrap();
                archive(&bytes, "0.2.0").unwrap();
                update.install(bytes).unwrap();
                assert_eq!(
                    std::fs::read(bundle.join("Contents/MacOS/flowguard")).unwrap(),
                    b"Controlled test fixture; never execute"
                );
            }
            server.join().unwrap();
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    #[ignore = "host ditto and disposable app rollback only"]
    fn backup_restores_a_failed_or_untrusted_replacement() {
        let root =
            std::env::temp_dir().join(format!("flowguard-rollback-fixture-{}", std::process::id()));
        let bundle = root.join("FlowGuard.app");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("payload"), b"original").unwrap();
        for failed in [true, false] {
            let result = replace_with_backup(
                &bundle,
                || {
                    std::fs::write(bundle.join("payload"), b"new").unwrap();
                    if failed {
                        Err(())
                    } else {
                        Ok(())
                    }
                },
                || false,
            );
            assert!(result.is_err());
            assert_eq!(std::fs::read(bundle.join("payload")).unwrap(), b"original");
        }
        replace_with_backup(
            &bundle,
            || {
                std::fs::write(bundle.join("payload"), b"new").unwrap();
                Ok(())
            },
            || true,
        )
        .unwrap();
        assert_eq!(std::fs::read(bundle.join("payload")).unwrap(), b"new");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod release_tests {
    use super::*;
    #[test]
    #[ignore = "requires explicitly prepared local release assets"]
    fn prepared_release_matches_production_updater() {
        let directory = std::env::var("FLOWGUARD_RELEASE_DIRECTORY").expect("release directory");
        let directory = std::path::Path::new(&directory);
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("latest.json")).unwrap()).unwrap();
        let version = manifest["version"].as_str().unwrap();
        for (_, target) in manifest["platforms"].as_object().unwrap() {
            let url = target["url"].as_str().unwrap();
            validate_release("0.0.0", version, url).unwrap();
            let bytes = std::fs::read(directory.join(url.rsplit('/').next().unwrap())).unwrap();
            signature(
                &bytes,
                target["signature"].as_str().unwrap(),
                &public_key(),
                version,
            )
            .unwrap();
            archive(&bytes, version).unwrap();
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod signed_upgrade_tests {
    use super::*;
    use std::io::{Read, Write};
    fn plist(path: &std::path::Path) -> plist::Dictionary {
        plist::Value::from_file(path)
            .unwrap()
            .into_dictionary()
            .unwrap()
    }
    fn verify_code(bundle: &std::path::Path) -> bool {
        std::process::Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(bundle)
            .status()
            .unwrap()
            .success()
    }
    #[test]
    #[ignore = "explicit local signed release and disposable copy; Gatekeeper may access Apple trust services"]
    fn production_key_signed_upgrade_on_disposable_app() {
        let directory = std::path::PathBuf::from(
            std::env::var("FLOWGUARD_RELEASE_DIRECTORY").expect("new release assets"),
        );
        let source = std::path::PathBuf::from(
            std::env::var("FLOWGUARD_PREVIOUS_APP").expect("previous signed app"),
        );
        let expect_rejection =
            std::env::var("FLOWGUARD_EXPECT_GATEKEEPER_REJECTION").as_deref() == Ok("1");
        let old = plist(&source.join("Contents/Info.plist"));
        let current = old["CFBundleShortVersionString"].as_string().unwrap();
        assert_eq!(
            old["CFBundleIdentifier"].as_string(),
            Some("com.flowguard.desktop")
        );
        assert!(verify_code(&source));
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("latest.json")).unwrap()).unwrap();
        let version = metadata["version"].as_str().unwrap();
        let target = tauri_plugin_updater::target().unwrap();
        let original_url = metadata["platforms"][&target]["url"].as_str().unwrap();
        validate_release(current, version, original_url).unwrap();
        let data = std::fs::read(directory.join(original_url.rsplit('/').next().unwrap())).unwrap();
        let signed = metadata["platforms"][&target]["signature"]
            .as_str()
            .unwrap();
        assert!(data.len() <= MAX_DOWNLOAD);
        signature(&data, signed, &public_key(), version).unwrap();
        archive(&data, version).unwrap();
        if expect_rejection {
            let candidate = directory.parent().unwrap().join("FlowGuard.app");
            assert!(verify_code(&candidate));
            let assessment = std::process::Command::new("/usr/sbin/spctl")
                .args(["--assess", "--type", "execute", "--verbose=4"])
                .arg(&candidate)
                .output()
                .unwrap();
            let details = String::from_utf8_lossy(&assessment.stderr);
            assert!(
                !assessment.status.success() && details.contains("Unnotarized Developer ID"),
                "Expected an unnotarized-candidate policy rejection: {details}"
            );
            println!("PASS candidate explicitly rejected as Unnotarized Developer ID");
        }
        let root = tempfile::tempdir().unwrap();
        // Every mutation is confined to this newly created temporary directory.
        let bundle = root.path().join("FlowGuard.app");
        assert!(std::process::Command::new("/usr/bin/ditto")
            .arg(&source)
            .arg(&bundle)
            .status()
            .unwrap()
            .success());
        let document = root.path().join("Application Support/document-v1.json");
        std::fs::create_dir_all(document.parent().unwrap()).unwrap();
        let document_bytes=br#"{"rules":[],"preferences":{"locale":"ja","view":"map","mode":"demo","panel":"right"}}"#;
        std::fs::write(&document, document_bytes).unwrap();
        // The real signed old app must be restored even if a replacement destroys its executable.
        assert!(replace_with_backup(
            &bundle,
            || {
                std::fs::write(
                    bundle.join("Contents/MacOS/flowguard"),
                    b"controlled failed replacement",
                )
                .unwrap();
                Ok(())
            },
            || false
        )
        .is_err());
        assert!(verify_code(&bundle));
        assert_eq!(
            plist(&bundle.join("Contents/Info.plist"))["CFBundleShortVersionString"].as_string(),
            Some(current)
        );
        for tampered in [true, false] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let endpoint = format!("http://{address}/latest.json");
            let mut manifest = metadata.clone();
            manifest["platforms"][&target]["url"] =
                format!("http://{address}/FlowGuard.app.tar.gz").into();
            let mut artifact = data.clone();
            if tampered {
                artifact[0] ^= 1;
            }
            let server = std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + Duration::from_secs(30);
                for body in [serde_json::to_vec(&manifest).unwrap(), artifact] {
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((s, _)) => break s,
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(
                                    std::time::Instant::now() < deadline,
                                    "test server deadline"
                                );
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            Err(e) => panic!("{e}"),
                        }
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    stream
                        .set_write_timeout(Some(Duration::from_secs(20)))
                        .unwrap();
                    let mut request = [0u8; 4096];
                    stream.read(&mut request).unwrap();
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(&body).unwrap();
                }
            });
            let mut context = tauri::test::mock_context(tauri::test::noop_assets());
            context.package_info_mut().version = current.parse().unwrap();
            // Loopback transport is test-only; production endpoint/redirect policies are unchanged.
            context.config_mut().plugins.0.insert("updater".into(),serde_json::json!({"dangerousInsecureTransportProtocol":true,"pubkey":public_key(),"endpoints":[endpoint]}));
            let app = tauri::test::mock_builder()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .build(context)
                .unwrap();
            let update = tauri::async_runtime::block_on(
                app.updater_builder()
                    .executable_path(bundle.join("Contents/MacOS/flowguard"))
                    .timeout(Duration::from_secs(20))
                    .build()
                    .unwrap()
                    .check(),
            )
            .unwrap()
            .expect("strictly newer stable update");
            assert_eq!(update.version, version);
            let downloaded = tauri::async_runtime::block_on(update.download(|_, _| {}, || {}));
            if tampered {
                assert!(downloaded.is_err());
                assert!(verify_code(&bundle));
            } else {
                let bytes = downloaded.unwrap();
                signature(&bytes, signed, &public_key(), version).unwrap();
                archive(&bytes, version).unwrap();
                let result = install_with_backup(update, Arc::new(bytes), &bundle);
                if expect_rejection {
                    assert_eq!(result, Err("install_failed".into()));
                    assert_eq!(
                        plist(&bundle.join("Contents/Info.plist"))["CFBundleShortVersionString"]
                            .as_string(),
                        Some(current)
                    );
                } else {
                    result.unwrap();
                    assert_eq!(
                        plist(&bundle.join("Contents/Info.plist"))["CFBundleShortVersionString"]
                            .as_string(),
                        Some(version)
                    );
                }
                assert!(verify_code(&bundle));
            }
            assert_eq!(std::fs::read(&document).unwrap(), document_bytes);
            assert!(!root
                .path()
                .join(format!(".FlowGuard-update-backup-{}", std::process::id()))
                .exists());
            server.join().unwrap();
        }
        println!("PASS production key: tamper rejected; signed copy transaction {}; old signed rollback and external test document preserved; no app launched",if expect_rejection{"Gatekeeper rejection restored the previous app"}else{"verified and installed"});
    }
}

pub fn verify_artifact(path: &std::path::Path, version: &str, channel: &str) -> Result<(), String> {
    verify_artifact_for_arch(path, version, channel, std::env::consts::ARCH)
}
pub fn verify_artifact_for_arch(
    path: &std::path::Path,
    version: &str,
    channel: &str,
    arch: &str,
) -> Result<(), String> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "invalid_archive")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_DOWNLOAD as u64
    {
        return Err("invalid_archive".into());
    }
    let mut data = vec![];
    std::fs::File::open(path)
        .map_err(|_| "invalid_archive")?
        .take(MAX_DOWNLOAD as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| "invalid_archive")?;
    if data.len() > MAX_DOWNLOAD {
        return Err("invalid_archive".into());
    }
    let sigpath = std::path::PathBuf::from(format!("{}.sig", path.display()));
    let m = std::fs::symlink_metadata(&sigpath).map_err(|_| "signature_invalid")?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > 8192 {
        return Err("signature_invalid".into());
    }
    let encoded = std::fs::read_to_string(sigpath).map_err(|_| "signature_invalid")?;
    trusted_signature(&data, encoded.trim(), version, channel)?;
    archive(&data, version)?;
    archive_arch(&data, arch)
}
